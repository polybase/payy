const assert = require('node:assert/strict')
const bs58 = require('bs58')
const test = require('node:test')
const fixture = require('../../../../fixtures/payy-evm-client/v3.json')

const {
  createPayyClient,
  encodeDirectClaimLink,
  encodeEphemeralClaimLink,
  externalTransferTopic,
  noteCommitment,
  noteNullifier,
  payyNetworks,
  privacyAddressOwner,
  privacyAddressPrefix,
  zeroHash
} = require('../dist')
const {
  bigIntToB256,
  computeMerkleRoot,
  ephemeralOwner,
  firstNonceHash,
  nextNonceHash,
  publicKeyFromPrivateKey,
  publicKeyFromPrivacyAddress
} = require('../dist/crypto')
const { ethersRawTransactionSubmitter } = require('../dist/ethers')
const { viemRawTransactionSubmitter } = require('../dist/viem')

const TOKEN = '0x0000000000000000000000000000000000000001'
const EVM_ACCOUNT = '0x00000000000000000000000000000000000000aa'
const PRIVATE_KEY =
  '0x0101010101010101010101010101010101010101010101010101010101010101'
const EPHEMERAL_CLAIM_PRIVATE_KEY =
  '0x0303030303030303030303030303030303030303030303030303030303030303'
const PROOF = `0x${'11'.repeat(128)}`
const BN254_SCALAR_MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n

test('privacy accounts and default account come from signer', async () => {
  const env = await createEnv()
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()
  const empty = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner({
      ...env.signer,
      accounts: async () => []
    })
    .privacy()

  assert.deepEqual(await client.accounts(), [env.account])
  assert.deepEqual(await client.defaultAccount(), env.account)
  assert.equal(await empty.defaultAccount(), null)
})

test('owned-note lookup skips spent notes and setCheckpoint resumes forward', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const note0 = note({ owner, nonce: 0n, psi: 3n, value: 10n })
  const note1 = note({ owner, nonce: 1n, psi: 4n, value: 8n })
  const owned0 = await env.addOwnedNote(note0, true)
  const owned1 = await env.addOwnedNote(
    note1,
    false,
    await nextNonceHash(
      note0.kind,
      note0.token,
      note0.owner,
      note0.nonce + 1n,
      note0.psi
    )
  )
  const checkpoint = {
    privacyAccount: env.privacyAddress,
    token: TOKEN,
    ownedNote: owned0,
    checkedBlock: 7n
  }
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()

  await client.setCheckpoint(checkpoint)
  const state = await client.notes().get({
    privacyAccount: env.account,
    token: TOKEN
  })

  assert.equal(state.ownedNote.commitment, owned1.commitment)
  assert.equal(state.ownedNote.nullifier, owned1.nullifier)
  assert.deepEqual(state.ownedNote.note, owned1.note)
  assert.equal(state.ownedNote.sourceBlock, undefined)
  assert.equal(state.ownedNote.sourceBridgeTxHash, owned1.sourceBridgeTxHash)
  assert.equal(state.checkedBlock, env.blockNumber)
})

test('malformed cached checkpoints are discarded and resolved from chain', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 9n, value: 5n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()
  const checkpoint = {
    privacyAccount: env.privacyAddress,
    token: TOKEN,
    ownedNote: {
      ...owned,
      commitment: word(404n)
    },
    checkedBlock: 7n
  }

  const state = await client.notes().withCheckpoint(checkpoint).get({
    privacyAccount: env.account,
    token: TOKEN
  })

  assert.equal(state.ownedNote.commitment, owned.commitment)
  assert.equal(state.ownedNote.nullifier, owned.nullifier)
})

test('first-note checkpoints with malformed nonce hash are discarded', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 12n, value: 5n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()
  const checkpoint = {
    privacyAccount: env.privacyAddress,
    token: TOKEN,
    ownedNote: {
      ...owned,
      nonceHash: word(404n)
    },
    checkedBlock: 7n
  }
  await assert.rejects(() => client.setCheckpoint(checkpoint), {
    code: 'nonce_hash_mismatch'
  })

  const state = await client.notes().withCheckpoint(checkpoint).get({
    privacyAccount: env.account,
    token: TOKEN
  })

  assert.equal(state.ownedNote.commitment, owned.commitment)
  assert.equal(state.ownedNote.nonceHash, owned.nonceHash)
})

test('checkpoint source block cannot exceed checked block', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 11n, value: 5n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()

  await assert.rejects(
    () =>
      client.setCheckpoint({
        privacyAccount: env.privacyAddress,
        token: TOKEN,
        ownedNote: {
          ...owned,
          sourceBlock: 13n
        },
        checkedBlock: 12n
      }),
    { code: 'checkpoint_mismatch' }
  )
})

test('empty owned-note checkpoints rescan chain on later lookups', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()

  const empty = await client.notes().get({
    privacyAccount: env.account,
    token: TOKEN
  })
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 10n, value: 7n }),
    false
  )
  const discovered = await client.notes().get({
    privacyAccount: env.account,
    token: TOKEN
  })

  assert.equal(empty.ownedNote, null)
  assert.equal(discovered.ownedNote.commitment, owned.commitment)
  assert.equal(discovered.ownedNote.nullifier, owned.nullifier)
})

test('incoming list sorts canonically and filters spent notes by default', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const spent = note({ owner, nonce: 0n, psi: 5n, value: 1n })
  const unspent = note({ owner, nonce: 0n, psi: 6n, value: 2n })
  const spentTxHash = await env.addIncomingNote(spent, true)
  const unspentTxHash = await env.addIncomingNote(unspent, false)
  env.logs.push(
    log({
      txHash: spentTxHash,
      blockNumber: 3n,
      transactionIndex: 0,
      logIndex: 0,
      prefixTopic: env.prefixTopic
    }),
    log({
      txHash: unspentTxHash,
      blockNumber: 2n,
      transactionIndex: 9,
      logIndex: 1,
      prefixTopic: env.prefixTopic
    })
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()

  const filtered = await client.incoming().list({
    privacyAccount: env.account,
    fromBlock: 0n
  })
  const all = await client.incoming().list({
    privacyAccount: env.account,
    fromBlock: 0n,
    includeSpent: true
  })

  assert.equal(filtered.length, 1)
  assert.equal(filtered[0].commitment, await noteCommitment(unspent))
  assert.equal(filtered[0].nullifier, await noteNullifier(unspent))
  assert.equal(filtered[0].status, 'claimable')
  assert.equal(all.length, 2)
  assert.equal(all[1].status, 'spent')
  assert.equal(all[0].sourceTxHash, unspentTxHash)
  assert.equal(all[1].sourceTxHash, spentTxHash)
})

test('incoming list ignores decrypted notes for another owner', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const wrongOwner = owner === 1n ? 2n : 1n
  const candidate = note({ owner: wrongOwner, nonce: 0n, psi: 31n, value: 3n })
  const txHash = await env.addIncomingNote(candidate, false)
  env.logs.push(
    log({
      txHash,
      blockNumber: 4n,
      transactionIndex: 0,
      logIndex: 0,
      prefixTopic: env.prefixTopic
    })
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()

  const incoming = await client.incoming().list({
    privacyAccount: env.account,
    fromBlock: 0n
  })

  assert.equal(incoming.length, 0)
})

test('signer-backed privacy accounts work without global signer decryption', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 7n, value: 6n }),
    false
  )
  const incoming = note({ owner, nonce: 0n, psi: 8n, value: 2n })
  const txHash = await env.addIncomingNote(incoming, false)
  const embeddedAccount = {
    privacyAddress: env.privacyAddress,
    signer: env.signer
  }
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(failingGlobalSigner())
    .privacy()
  env.logs.push(
    log({
      txHash,
      blockNumber: 4n,
      transactionIndex: 0,
      logIndex: 0,
      prefixTopic: env.prefixTopic
    })
  )

  const state = await client.notes().get({
    privacyAccount: embeddedAccount,
    token: TOKEN
  })
  const incomingNotes = await client.incoming().list({
    privacyAccount: embeddedAccount,
    fromBlock: 0n
  })

  assert.equal(state.ownedNote.commitment, owned.commitment)
  assert.equal(incomingNotes.length, 1)
  assert.equal(incomingNotes[0].commitment, await noteCommitment(incoming))
})

test('balances derive spendable amount from owned-note state', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 11n, value: 19n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()

  const state = await client.balances().get({
    privacyAccount: env.account,
    token: TOKEN
  })

  assert.equal(state.balance.spendable, 19n)
  assert.equal(state.ownedNoteState.ownedNote.note.value, 19n)
})

test('incoming watch returns the next inclusive block after a finite range', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const txHash = await env.addIncomingNote(
    note({ owner, nonce: 0n, psi: 12n, value: 1n }),
    false
  )
  env.logs.push(
    log({
      txHash,
      blockNumber: 4n,
      transactionIndex: 0,
      logIndex: 0,
      prefixTopic: env.prefixTopic
    })
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()
  const delivered = []

  const result = await client.incoming().watch(
    {
      privacyAccount: env.account,
      fromBlock: 4n,
      toBlock: 4n,
      pollIntervalMs: 1
    },
    (incoming) => {
      delivered.push(incoming)
    }
  )

  assert.equal(delivered.length, 1)
  assert.equal(result.nextFromBlock, 5n)
})

test('incoming watch returns checkpoint with callback failure', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const txHash = await env.addIncomingNote(
    note({ owner, nonce: 0n, psi: 33n, value: 1n }),
    false
  )
  env.logs.push(
    log({
      txHash,
      blockNumber: 4n,
      transactionIndex: 0,
      logIndex: 0,
      prefixTopic: env.prefixTopic
    })
  )
  const client = createPayyClient({
    publicClient: env.readClient
  })
    .privacySigner(env.signer)
    .privacy()
  const error = new Error('stop')

  const result = await client.incoming().watch(
    {
      privacyAccount: env.account,
      fromBlock: 4n,
      toBlock: 4n,
      pollIntervalMs: 1
    },
    () => {
      throw error
    }
  )

  assert.equal(result.nextFromBlock, 4n)
  assert.equal(result.error, error)
})

test('direct send prepare consumes checkpoint and returns delivery metadata', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 7n, value: 10n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const checkpoint = {
    privacyAccount: env.privacyAddress,
    token: TOKEN,
    ownedNote: owned,
    checkedBlock: 7n
  }

  const prepared = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 4n,
      recipient: env.privacyAddress
    })
    .withCheckpoint(checkpoint)
    .prepare()
  const linked = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 2n,
      recipient: env.privacyAddress
    })
    .withCheckpoint(checkpoint)
    .link('hello')
  const parsedLink = await client.links().parse(linked.result.payload[1].value)

  assert.equal(env.prover.calls[0].operation, 'transfer_send')
  assert.deepEqual(
    Object.keys(prepared.result.preparedCall),
    fixture.prepared_call_public_fields
  )
  assert.equal(prepared.result.preparedCall.chainId, payyNetworks.dev.chainId)
  assert.deepEqual(Object.keys(prepared.result.preparedCall.statePreview), [
    'privacyAccount',
    'token',
    'recentRoot',
    'inputCommitments',
    'inputNullifiers',
    'outputCommitments'
  ])
  assert.equal(
    prepared.result.payload.recipient.bytes,
    env.privacyAddress.bytes
  )
  assert.equal(prepared.result.payload.note.value, 4n)
  assert.equal(
    prepared.result.payload.commitment,
    await noteCommitment(prepared.result.payload.note)
  )
  assert.equal(
    prepared.result.payload.sourceBridgeTxHash,
    client
      .bridge()
      .computeTxHash(
        prepared.result.preparedCall.verificationKeyHash,
        prepared.result.preparedCall.proof,
        prepared.result.preparedCall.publicInputs
      )
  )
  assert.deepEqual(prepared.result.preparedCall.statePreview.inputNullifiers, [
    owned.nullifier
  ])
  assert.equal(
    prepared.result.preparedCall.bridgeRequest.to,
    payyNetworks.dev.privacyBridge
  )
  assert.equal(parsedLink.claimSourceKind, 'direct')
  assert.equal(parsedLink.message, 'hello')
  assert.equal(parsedLink.incomingTransfer, undefined)
  assert.equal(
    parsedLink.directNote.note.owner,
    linked.result.payload[0].note.owner
  )
})

test('auto spend input uses root paired with merkle path', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const inputNote = note({ owner, nonce: 0n, psi: 32n, value: 10n })
  const meta = await ownedNote(inputNote)
  const owned = await env.addOwnedNote(inputNote, false)
  env.setMerklePathRoot(meta.recentRoot)
  env.setRoot(word(999n))
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()

  const prepared = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 4n,
      recipient: env.privacyAddress
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .prepare()

  assert.equal(
    prepared.result.preparedCall.statePreview.recentRoot,
    meta.recentRoot
  )
})

test('ephemeral send prepare returns bearer handoff metadata', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 18n, value: 10n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()

  const prepared = await client
    .send()
    .ephemeral({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 3n
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .prepare()

  assert.equal(prepared.result.payload.ephemeralPrivateKey, PRIVATE_KEY)
  assert.equal(
    prepared.result.payload.commitment,
    await noteCommitment(prepared.result.payload.note)
  )
  assert.equal(
    prepared.result.payload.note.owner,
    await ephemeralOwner(PRIVATE_KEY)
  )
})

test('ephemeral send uses signer-backed account for key generation', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 28n, value: 10n }),
    false
  )
  const embeddedAccount = {
    privacyAddress: env.privacyAddress,
    signer: env.signer
  }
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(failingGlobalSigner())
    .privacy()

  const prepared = await client
    .send()
    .ephemeral({
      privacyAccount: embeddedAccount,
      token: TOKEN,
      amount: 3n
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .prepare()

  assert.equal(prepared.result.payload.ephemeralPrivateKey, PRIVATE_KEY)
})

test('direct send rejects malformed recipient privacy address as typed error', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 29n, value: 10n }),
    false
  )
  const malformedRecipient = { bytes: `0x40${'00'.repeat(31)}` }
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()

  await assert.rejects(
    client
      .send()
      .to({
        privacyAccount: env.account,
        token: TOKEN,
        amount: 3n,
        recipient: malformedRecipient
      })
      .withCheckpoint({
        privacyAccount: env.privacyAddress,
        token: TOKEN,
        ownedNote: owned,
        checkedBlock: 7n
      })
      .prepare(),
    { code: 'invalid_privacy_address' }
  )
})

test('claim prepare supports direct, direct-link, and ephemeral bearer paths', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const incoming = note({ owner, nonce: 0n, psi: 13n, value: 4n })
  const txHash = await env.addIncomingNote(incoming, false)
  const incomingNote = {
    note: incoming,
    commitment: await noteCommitment(incoming),
    nullifier: await noteNullifier(incoming),
    sourcePosition: {
      blockNumber: 1n,
      transactionIndex: 0,
      logIndex: 0
    },
    sourceTxHash: txHash,
    sourceBridgeTxHash: txHash,
    status: 'claimable'
  }
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()

  const direct = await client
    .claim()
    .account(env.account)
    .note(incomingNote)
    .prepare()
  const directLink = await client
    .claim()
    .account(env.account)
    .link(
      encodeDirectClaimLink({
        recipient: env.privacyAddress,
        note: incoming,
        commitment: await noteCommitment(incoming)
      })
    )
    .prepare()
  const ephemeralNote = note({
    owner: await ephemeralOwner(EPHEMERAL_CLAIM_PRIVATE_KEY),
    nonce: 0n,
    psi: 14n,
    value: 3n
  })
  await env.addIncomingNote(ephemeralNote, false)
  const transfer = {
    note: ephemeralNote,
    commitment: await noteCommitment(ephemeralNote),
    ephemeralPrivateKey: EPHEMERAL_CLAIM_PRIVATE_KEY
  }
  const ephemeral = await client
    .claim()
    .account(env.account)
    .ephemeral(transfer)
    .prepare()
  const ephemeralLink = await client
    .claim()
    .account(env.account)
    .link(encodeEphemeralClaimLink(transfer))
    .prepare()
  await assert.rejects(client.claim().ephemeral(transfer).prepare(), {
    code: 'missing_privacy_signer'
  })
  await assert.rejects(
    client.claim().link(encodeEphemeralClaimLink(transfer)).prepare(),
    { code: 'missing_privacy_signer' }
  )
  const ephemeralPublicKey = await publicKeyFromPrivateKey(
    EPHEMERAL_CLAIM_PRIVATE_KEY
  )
  const recipientPublicKey = publicKeyFromPrivacyAddress(env.privacyAddress)
  const ephemeralClaimInputs = env.prover.calls[2].inputs

  assert.equal(direct.result.preparedCall.operation, 'transfer_claim')
  assert.equal(directLink.result.payload.claimSourceKind, 'direct')
  assert.equal(
    ephemeral.result.payload.ephemeralPrivateKey,
    EPHEMERAL_CLAIM_PRIVATE_KEY
  )
  assert.equal(ephemeralLink.result.payload.claimSourceKind, 'ephemeral')
  assert.equal(
    ephemeralClaimInputs.recipient_signature.public_key_x,
    recipientPublicKey.x.toString()
  )
  assert.equal(
    ephemeralClaimInputs.incoming_note_signature.public_key_x,
    BigInt(ephemeralPublicKey.x).toString()
  )
  assert.notEqual(
    ephemeralClaimInputs.recipient_signature.public_key_x,
    ephemeralClaimInputs.incoming_note_signature.public_key_x
  )
})

test('claim validation rejects tampered, manual-mismatched, and unpublished sources', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const incoming = note({ owner, nonce: 0n, psi: 19n, value: 4n })
  const txHash = await env.addIncomingNote(incoming, false)
  const mismatch = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 20n, value: 1n }),
    false
  )
  const unpublished = note({ owner, nonce: 0n, psi: 21n, value: 4n })
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const incomingNote = {
    note: incoming,
    commitment: await noteCommitment(incoming),
    nullifier: await noteNullifier(incoming),
    sourcePosition: {
      blockNumber: 1n,
      transactionIndex: 0,
      logIndex: 0
    },
    sourceTxHash: txHash,
    sourceBridgeTxHash: txHash,
    status: 'claimable'
  }
  const malformedTransfer = {
    note: { ...incoming, kind: 2n },
    commitment: word(404n),
    ephemeralPrivateKey: PRIVATE_KEY
  }

  const unpublishedCommitment = await noteCommitment(unpublished)
  await assert.rejects(
    client
      .claim()
      .account(env.account)
      .note({ ...incomingNote, note: { ...incoming, kind: 2n } })
      .prepare(),
    { code: 'invalid_incoming_transfer' }
  )
  await assert.rejects(
    client
      .claim()
      .account(env.account)
      .note({ ...incomingNote, note: { ...incoming, value: 1n << 240n } })
      .prepare(),
    { code: 'value_out_of_range' }
  )
  await assert.rejects(
    client
      .claim()
      .account(env.account)
      .note({
        ...incomingNote,
        note: { ...incoming, psi: BN254_SCALAR_MODULUS }
      })
      .prepare(),
    { code: 'field_out_of_range' }
  )
  await assert.rejects(
    client.claim().account(env.account).ephemeral(malformedTransfer).prepare(),
    { code: 'invalid_incoming_transfer' }
  )
  await assert.rejects(
    client
      .claim()
      .account(env.account)
      .note({ ...incomingNote, commitment: word(303n) })
      .prepare(),
    { code: 'commitment_mismatch' }
  )
  await assert.rejects(
    client
      .claim()
      .account(env.account)
      .note(incomingNote)
      .withClaimInputs({
        ownedInput: {
          kind: 'padding',
          data: {
            recentRoot: mismatch.recentRoot
          }
        },
        incomingInput: {
          ownedNote: mismatch,
          merklePath: zeroMerklePath(),
          recentRoot: mismatch.recentRoot
        }
      })
      .prepare(),
    { code: 'commitment_mismatch' }
  )
  await assert.rejects(
    client
      .claim()
      .account(env.account)
      .note({
        ...incomingNote,
        note: unpublished,
        commitment: unpublishedCommitment,
        nullifier: await noteNullifier(unpublished)
      })
      .prepare(),
    (error) =>
      error.code === 'commitment_not_found'
      && error.data?.commitment === unpublishedCommitment
  )
})

test('direct send can be discovered and prepared for claim', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 23n, value: 10n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const preparedSend = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 4n,
      recipient: env.privacyAddress
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .prepare()
  const txHash = await env.addIncomingNote(
    preparedSend.result.payload.note,
    false
  )
  env.spent.add(owned.nullifier)
  env.logs.push(
    log({
      txHash,
      blockNumber: 8n,
      transactionIndex: 0,
      logIndex: 0,
      prefixTopic: env.prefixTopic
    })
  )

  const incoming = await client.incoming().list({
    privacyAccount: env.account,
    fromBlock: 0n
  })
  const preparedClaim = await client
    .claim()
    .account(env.account)
    .note(incoming[0])
    .prepare()

  assert.equal(incoming.length, 1)
  assert.equal(incoming[0].commitment, preparedSend.result.payload.commitment)
  assert.equal(preparedClaim.result.preparedCall.operation, 'transfer_claim')
})

test('ephemeral handoff and link claims prepare from published transfer notes', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 24n, value: 10n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const ephemeralSend = await client
    .send()
    .ephemeral({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 2n
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .link('handoff')
  await env.addIncomingNote(ephemeralSend.result.payload[0].note, false)
  env.spent.add(owned.nullifier)

  const fromHandoff = await client
    .claim()
    .account(env.account)
    .ephemeral(ephemeralSend.result.payload[0])
    .prepare()
  const fromLink = await client
    .claim()
    .account(env.account)
    .link(ephemeralSend.result.payload[1])
    .prepare()

  assert.equal(fromHandoff.result.preparedCall.operation, 'transfer_claim')
  assert.equal(fromLink.result.payload.claimSourceKind, 'ephemeral')
  assert.equal(fromLink.result.payload.message, 'handoff')
})

test('direct link claim prepares after the linked note is published', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 25n, value: 10n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const linked = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 2n,
      recipient: env.privacyAddress
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .link('direct')
  await env.addIncomingNote(linked.result.payload[0].note, false)
  env.spent.add(owned.nullifier)

  const preparedClaim = await client
    .claim()
    .account(env.account)
    .link(linked.result.payload[1])
    .prepare()

  assert.equal(preparedClaim.result.payload.claimSourceKind, 'direct')
  assert.equal(preparedClaim.result.payload.message, 'direct')
  assert.equal(preparedClaim.result.preparedCall.operation, 'transfer_claim')
})

test('manual explicit claim inputs prepare without owned-note lookup', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const incoming = note({ owner, nonce: 0n, psi: 26n, value: 3n })
  const incomingMeta = await ownedNote(incoming)
  const txHash = await env.addIncomingNote(incoming, false)
  const incomingOwned = {
    note: incoming,
    commitment: incomingMeta.commitment,
    nullifier: incomingMeta.nullifier,
    nonceHash: await firstNonceHash(
      incoming.kind,
      incoming.token,
      incoming.owner
    ),
    sourceBlock: 8n,
    sourceBridgeTxHash: txHash
  }
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const incomingNote = {
    note: incoming,
    commitment: incomingOwned.commitment,
    nullifier: incomingOwned.nullifier,
    sourcePosition: {
      blockNumber: 8n,
      transactionIndex: 0,
      logIndex: 0
    },
    sourceTxHash: txHash,
    sourceBridgeTxHash: txHash,
    status: 'claimable'
  }

  const prepared = await client
    .claim()
    .account(env.account)
    .note(incomingNote)
    .withClaimInputs({
      ownedInput: {
        kind: 'padding',
        data: {
          recentRoot: incomingMeta.recentRoot
        }
      },
      incomingInput: {
        ownedNote: incomingOwned,
        merklePath: zeroMerklePath(),
        recentRoot: incomingMeta.recentRoot
      }
    })
    .prepare()

  assert.equal(prepared.result.preparedCall.operation, 'transfer_claim')
  assert.equal(env.prover.calls.at(-1).inputs.input_note_own.kind, '0')
})

test('claim-link parsing rejects malformed and out-of-range payloads', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const client = createPayyClient({
    publicClient: env.readClient
  })

  await assert.rejects(client.links().parse('/not-s#abc'), {
    code: 'invalid_claim_link'
  })
  await assert.rejects(
    client
      .links()
      .parse(
        invalidDirectLink(
          note({ owner, nonce: 0n, psi: 1n, value: 1n << 240n })
        )
      ),
    { code: 'invalid_claim_link' }
  )
})

test('account-scoped direct claim rejects mismatched privacy account', async () => {
  const env = await createEnv()
  const other = await privacyAccount(
    '0x0202020202020202020202020202020202020202020202020202020202020202'
  )
  const owner = await privacyAddressOwner(env.privacyAddress)
  const incoming = note({ owner, nonce: 0n, psi: 15n, value: 4n })
  const txHash = await env.addIncomingNote(incoming, false)
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()

  await assert.rejects(
    client
      .claim()
      .account(other)
      .note({
        note: incoming,
        commitment: await noteCommitment(incoming),
        nullifier: await noteNullifier(incoming),
        sourcePosition: {
          blockNumber: 1n,
          transactionIndex: 0,
          logIndex: 0
        },
        sourceTxHash: txHash,
        sourceBridgeTxHash: txHash,
        status: 'claimable'
      })
      .prepare(),
    { code: 'privacy_account_mismatch' }
  )
})

test('zero amount prepare rejects before proving', async () => {
  const env = await createEnv()
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()

  await assert.rejects(
    client
      .send()
      .to({
        privacyAccount: env.account,
        token: TOKEN,
        amount: 0n,
        recipient: env.privacyAddress
      })
      .prepare(),
    { code: 'amount_zero' }
  )
  assert.equal(env.prover.calls.length, 0)
})

test('submitAndWait returns submitted and confirmed operation identity', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 16n, value: 10n }),
    false
  )
  const sourceTxHash = word(123n)
  const sent = []
  const client = createPayyClient({
    publicClient: env.readClient,
    evmSubmitter: {
      getChainId: async () => payyNetworks.dev.chainId,
      getAddress: async () => EVM_ACCOUNT,
      sendTransaction: async (request) => {
        sent.push(request)
        return sourceTxHash
      }
    },
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const prepared = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 1n,
      recipient: env.privacyAddress
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .prepare()

  const expectedSourceBridgeTxHash = client
    .bridge()
    .computeTxHash(
      prepared.result.preparedCall.verificationKeyHash,
      prepared.result.preparedCall.proof,
      prepared.result.preparedCall.publicInputs
    )
  const confirmed = await prepared.submitAndWait()

  assert.equal(sent.length, 1)
  assert.equal(confirmed.sourceTxHash, sourceTxHash)
  assert.equal(confirmed.payload.sourceTxHash, sourceTxHash)
  assert.equal(confirmed.payload.sourceBridgeTxHash, expectedSourceBridgeTxHash)
  assert.equal(confirmed.receipt.status, 'success')
})

test('submitAndWait does not treat unconfirmed output notes as cached latest state', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 22n, value: 10n }),
    false
  )
  const client = createPayyClient({
    publicClient: env.readClient,
    evmSubmitter: {
      getChainId: async () => payyNetworks.dev.chainId,
      getAddress: async () => EVM_ACCOUNT,
      sendTransaction: async () => word(501n)
    },
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  await client.setCheckpoint({
    privacyAccount: env.privacyAddress,
    token: TOKEN,
    ownedNote: owned,
    checkedBlock: 7n
  })
  const prepared = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 1n,
      recipient: env.privacyAddress
    })
    .prepare()

  await prepared.submitAndWait()
  env.spent.add(owned.nullifier)
  const state = await client.notes().get({
    privacyAccount: env.account,
    token: TOKEN
  })

  assert.equal(state.ownedNote, null)
})

test('submit rejects mismatched submitter chain id before broadcast', async () => {
  const env = await createEnv()
  const owner = await privacyAddressOwner(env.privacyAddress)
  const owned = await env.addOwnedNote(
    note({ owner, nonce: 0n, psi: 17n, value: 10n }),
    false
  )
  let sent = false
  const client = createPayyClient({
    publicClient: env.readClient,
    evmSubmitter: {
      getChainId: async () => payyNetworks.dev.chainId + 1,
      getAddress: async () => EVM_ACCOUNT,
      sendTransaction: async () => {
        sent = true
        return word(1n)
      }
    },
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()
  const prepared = await client
    .send()
    .to({
      privacyAccount: env.account,
      token: TOKEN,
      amount: 1n,
      recipient: env.privacyAddress
    })
    .withCheckpoint({
      privacyAccount: env.privacyAddress,
      token: TOKEN,
      ownedNote: owned,
      checkedBlock: 7n
    })
    .prepare()

  await assert.rejects(prepared.submit(), { code: 'chain_id_mismatch' })
  assert.equal(sent, false)
})

test('withEvmPrivateKey without raw submitter does not reuse delegated submitter', async () => {
  const env = await createEnv()
  let sent = false
  const client = createPayyClient({
    publicClient: env.readClient,
    evmSubmitter: {
      getChainId: async () => payyNetworks.dev.chainId,
      getAddress: async () => EVM_ACCOUNT,
      sendTransaction: async () => {
        sent = true
        return word(601n)
      }
    },
    provingBackend: env.prover
  })
    .withEvmPrivateKey(PRIVATE_KEY)
    .privacy()
  const privacyAccount = await client.defaultAccount()

  const prepared = await client
    .mint({
      privacyAccount,
      evmAccount: EVM_ACCOUNT,
      token: TOKEN,
      amount: 1n
    })
    .prepare()

  await assert.rejects(prepared.submit(), { code: 'missing_evm_submitter' })
  assert.equal(sent, false)
})

test('mint submit uses signer-backed EvmAccount submitter', async () => {
  const env = await createEnv()
  const sourceTxHash = word(301n)
  let sent = null
  const submitter = {
    getChainId: async () => payyNetworks.dev.chainId,
    getAddress: async () => EVM_ACCOUNT,
    sendTransaction: async (request) => {
      sent = request
      return sourceTxHash
    }
  }
  const client = createPayyClient({
    publicClient: env.readClient,
    provingBackend: env.prover
  })
    .privacySigner(env.signer)
    .privacy()

  const prepared = await client
    .mint({
      privacyAccount: env.account,
      evmAccount: {
        address: EVM_ACCOUNT,
        submitter
      },
      token: TOKEN,
      amount: 5n
    })
    .prepare()
  const submitted = await prepared.submit()

  assert.equal(env.prover.calls[0].operation, 'mint')
  assert.equal(env.prover.calls[0].inputs.input_note.kind, '0')
  assert.equal(sent.from, EVM_ACCOUNT)
  assert.equal(submitted.sourceTxHash, sourceTxHash)
})

test('mint submit rejects mismatched or unknown delegated sender before broadcast', async () => {
  const env = await createEnv()
  let mismatchedSent = false
  let unknownSent = false
  const base = {
    publicClient: env.readClient,
    provingBackend: env.prover
  }
  const mismatched = createPayyClient({
    ...base,
    evmSubmitter: {
      getChainId: async () => payyNetworks.dev.chainId,
      getAddress: async () => '0x00000000000000000000000000000000000000bb',
      sendTransaction: async () => {
        mismatchedSent = true
        return word(1n)
      }
    }
  })
    .privacySigner(env.signer)
    .privacy()
  const unknown = createPayyClient({
    ...base,
    evmSubmitter: {
      getChainId: async () => payyNetworks.dev.chainId,
      getAddress: async () => null,
      sendTransaction: async () => {
        unknownSent = true
        return word(1n)
      }
    }
  })
    .privacySigner(env.signer)
    .privacy()

  await assert.rejects(
    (
      await mismatched
        .mint({
          privacyAccount: env.account,
          evmAccount: EVM_ACCOUNT,
          token: TOKEN,
          amount: 1n
        })
        .prepare()
    ).submit(),
    { code: 'evm_account_mismatch' }
  )
  await assert.rejects(
    (
      await unknown
        .mint({
          privacyAccount: env.account,
          evmAccount: EVM_ACCOUNT,
          token: TOKEN,
          amount: 1n
        })
        .prepare()
    ).submit(),
    { code: 'evm_account_mismatch' }
  )

  assert.equal(mismatchedSent, false)
  assert.equal(unknownSent, false)
})

test('viem and ethers raw submitters construct and broadcast local transactions', async () => {
  const rawTxHashes = [word(201n), word(202n)]
  const viemNonceRequests = []
  const ethersNonceRequests = []
  const viemRaw = viemRawTransactionSubmitter({
    getChainId: async () => payyNetworks.dev.chainId,
    getBlockNumber: async () => 1n,
    call: async () => ({ data: zeroHash() }),
    getLogs: async () => [],
    getTransactionReceipt: async () => null,
    waitForTransactionReceipt: async () => ({
      transactionHash: rawTxHashes[0],
      blockNumber: 1n,
      status: 'success'
    }),
    getTransactionCount: async (args) => {
      viemNonceRequests.push(args)
      return 1
    },
    estimateGas: async () => 21000n,
    estimateFeesPerGas: async () => ({
      maxFeePerGas: 2n,
      maxPriorityFeePerGas: 1n
    }),
    sendRawTransaction: async ({ serializedTransaction }) => {
      assert.match(serializedTransaction, /^0x/)
      return rawTxHashes[0]
    }
  })
  const ethersRaw = ethersRawTransactionSubmitter({
    getNetwork: async () => ({ chainId: BigInt(payyNetworks.dev.chainId) }),
    getBlockNumber: async () => 1,
    call: async () => zeroHash(),
    getLogs: async () => [],
    getTransactionReceipt: async () => null,
    waitForTransaction: async () => ({
      hash: rawTxHashes[1],
      blockNumber: 1,
      status: 1
    }),
    getTransactionCount: async (address, blockTag) => {
      ethersNonceRequests.push({ address, blockTag })
      return 1
    },
    estimateGas: async () => 21000n,
    getFeeData: async () => ({
      maxFeePerGas: 2n,
      maxPriorityFeePerGas: 1n
    }),
    broadcastTransaction: async (rawTransaction) => {
      assert.match(rawTransaction, /^0x/)
      return { hash: rawTxHashes[1] }
    }
  })
  const request = {
    to: payyNetworks.dev.privacyBridge,
    data: '0x',
    value: 0n
  }

  assert.equal(
    await viemRaw.sendLocalTransaction(PRIVATE_KEY, request),
    rawTxHashes[0]
  )
  assert.equal(
    await ethersRaw.sendLocalTransaction(PRIVATE_KEY, request),
    rawTxHashes[1]
  )
  assert.deepEqual(
    viemNonceRequests.map((item) => item.blockTag),
    ['pending']
  )
  assert.deepEqual(
    ethersNonceRequests.map((item) => item.blockTag),
    ['pending']
  )
})

async function createEnv() {
  let bridge
  const localAccount = await privacyAccount(PRIVATE_KEY)
  const privacyAddress = localAccount.privacyAddress
  const chainPublicKey = publicKeyFromPrivacyAddress(privacyAddress)
  const prefixTopic = topicFromPrefix(
    await privacyAddressPrefix(privacyAddress)
  )
  const state = {
    blockNumber: 12n,
    logs: [],
    spent: new Set(),
    txnByNonceHash: new Map(),
    txnByCommitment: new Map(),
    txnData: new Map(),
    notesByTxnHash: new Map()
  }
  const readClient = {
    getChainId: async () => payyNetworks.dev.chainId,
    getBlockNumber: async () => state.blockNumber,
    getLogs: async (filter) =>
      state.logs.filter(
        (entry) =>
          entry.blockNumber >= filter.fromBlock
          && entry.blockNumber <= filter.toBlock
          && filter.topics.every(
            (topic, index) => topic === null || entry.topics[index] === topic
          )
      ),
    getTransactionReceipt: async () => null,
    waitForTransactionReceipt: async (args) => ({
      transactionHash: args.hash,
      blockNumber: state.blockNumber,
      status: 'success'
    }),
    readContract: async ({ data }) => {
      if (data === bridge.encodeGetRootCall()) {
        return state.root ?? zeroHash()
      }
      if (
        data.startsWith(selector(bridge.encodeGetMerklePathCall(zeroHash())))
      ) {
        return encodeMerklePath(
          state.merklePathRoot ?? state.root ?? zeroHash(),
          zeroMerklePath()
        )
      }
      if (
        data.startsWith(selector(bridge.encodeElementExistsCall(zeroHash())))
      ) {
        return word(state.spent.has(argumentWord(data)) ? 1n : 0n)
      }
      if (
        data.startsWith(
          selector(bridge.encodeGetTxnHashByNonceHashCall(zeroHash()))
        )
      ) {
        return state.txnByNonceHash.get(argumentWord(data)) ?? zeroHash()
      }
      if (
        data.startsWith(
          selector(bridge.encodeGetTxnHashByCommitmentCall(zeroHash()))
        )
      ) {
        return state.txnByCommitment.get(argumentWord(data)) ?? zeroHash()
      }
      if (data.startsWith(selector(bridge.encodeGetTxnDataCall(zeroHash())))) {
        return (
          state.txnData.get(argumentWord(data)) ?? encodeTxnData(zeroHash())
        )
      }
      if (data === bridge.encodeGetChainPublicKeyCall()) {
        return `${bigIntToB256(chainPublicKey.x)}${bigIntToB256(chainPublicKey.y).slice(2)}`
      }
      return zeroHash()
    }
  }
  const client = createPayyClient({ publicClient: readClient })
  bridge = client.bridge()
  const signer = {
    accounts: async () => [account],
    signTxCommitment: async () => {
      const key = publicKeyFromPrivacyAddress(privacyAddress)
      return {
        publicKeyX: bigIntToB256(key.x),
        publicKeyY: bigIntToB256(key.y),
        signature: `0x${'01'.repeat(64)}`
      }
    },
    decryptSenderNote: async ({ txnData }) =>
      state.notesByTxnHash.get(txnData.memo) ?? null,
    decryptRecipientNote: async ({ txnData }) =>
      state.notesByTxnHash.get(txnData.memo) ?? null,
    generateEphemeralKey: async () => ({
      privateKey: PRIVATE_KEY,
      privacyAddress: account.privacyAddress
    })
  }
  const account = {
    privacyAddress,
    signer
  }
  const prover = {
    calls: [],
    prove: async (operation, inputs) => {
      prover.calls.push({ operation, inputs })
      return {
        proof: PROOF,
        publicInputs: Array.from({ length: 33 }, zeroHash),
        verificationKeyHash: word(99n)
      }
    }
  }
  return {
    ...state,
    account,
    privacyAddress: account.privacyAddress,
    prefixTopic,
    readClient,
    signer,
    prover,
    setRoot: (root) => {
      state.root = root
    },
    setMerklePathRoot: (root) => {
      state.merklePathRoot = root
    },
    addOwnedNote: async (inputNote, spent, nonceHashOverride) => {
      const owned = await ownedNote(inputNote)
      const nonceHash =
        nonceHashOverride
        ?? (inputNote.nonce === 0n
          ? await firstNonceHash(
              inputNote.kind,
              inputNote.token,
              inputNote.owner
            )
          : await nextNonceHash(
              inputNote.kind,
              inputNote.token,
              inputNote.owner,
              inputNote.nonce,
              inputNote.psi
            ))
      const txHash = word(BigInt(state.txnData.size + 1))
      state.txnByNonceHash.set(nonceHash, txHash)
      state.txnByCommitment.set(owned.commitment, txHash)
      state.txnData.set(txHash, encodeTxnData(txHash))
      state.notesByTxnHash.set(txHash, inputNote)
      if (spent) {
        state.spent.add(owned.nullifier)
      }
      state.root = owned.recentRoot
      return {
        note: inputNote,
        commitment: owned.commitment,
        nullifier: owned.nullifier,
        nonceHash,
        sourceBlock: 1n,
        sourceBridgeTxHash: txHash
      }
    },
    addIncomingNote: async (inputNote, spent) => {
      const owned = await ownedNote(inputNote)
      const txHash = word(BigInt(state.txnData.size + 1))
      state.txnByCommitment.set(owned.commitment, txHash)
      state.txnData.set(txHash, encodeTxnData(txHash))
      state.notesByTxnHash.set(txHash, inputNote)
      if (spent) {
        state.spent.add(owned.nullifier)
      }
      state.root = owned.recentRoot
      return txHash
    }
  }
}

async function privacyAccount(privateKey) {
  const { createLocalPrivacySignerFromGrumpkinPrivateKey } = require('../dist')
  const signer = createLocalPrivacySignerFromGrumpkinPrivateKey(privateKey)
  return (await signer.accounts())[0]
}

function failingGlobalSigner() {
  const fail = async () => {
    throw new Error('global privacy signer should not be used')
  }
  return {
    accounts: async () => [],
    signTxCommitment: fail,
    decryptSenderNote: fail,
    decryptRecipientNote: fail,
    generateEphemeralKey: fail
  }
}

function note(overrides) {
  return {
    kind: 1n,
    token: BigInt(TOKEN),
    nonce: 0n,
    psi: 1n,
    owner: 1n,
    value: 1n,
    ...overrides
  }
}

async function ownedNote(inputNote) {
  const commitment = await noteCommitment(inputNote)
  const nullifier = await noteNullifier(inputNote)
  const recentRoot = bigIntToB256(
    await computeMerkleRoot(
      BigInt(commitment),
      BigInt(commitment),
      zeroMerklePath().map(BigInt)
    )
  )
  return {
    commitment,
    nullifier,
    recentRoot
  }
}

function log({ txHash, blockNumber, transactionIndex, logIndex, prefixTopic }) {
  return {
    address: payyNetworks.dev.privacyBridge,
    blockNumber,
    transactionIndex,
    logIndex,
    transactionHash: txHash,
    topics: [externalTransferTopic(), prefixTopic, txHash],
    data: '0x'
  }
}

function topicFromPrefix(prefix) {
  return `0x${prefix.bytes.slice(2).padEnd(64, '0')}`
}

function selector(callData) {
  return callData.slice(0, 10)
}

function argumentWord(callData) {
  return `0x${callData.slice(-64)}`
}

function word(value) {
  return bigIntToB256(value)
}

function encodeMerklePath(root, path) {
  return `${root}${word(64n).slice(2)}${word(BigInt(path.length)).slice(2)}${path
    .map((item) => item.slice(2))
    .join('')}`
}

function encodeTxnData(memo) {
  const words = [
    zeroHash(),
    ...Array.from({ length: 5 }, zeroHash),
    ...Array.from({ length: 5 }, zeroHash),
    ...Array.from({ length: 3 }, zeroHash),
    ...Array.from({ length: 3 }, zeroHash),
    ...Array.from({ length: 4 }, zeroHash),
    ...Array.from({ length: 4 }, zeroHash),
    memo
  ]
  return `0x${words.map((item) => item.slice(2)).join('')}`
}

function zeroMerklePath() {
  return Array.from({ length: 160 }, zeroHash)
}

function invalidDirectLink(inputNote) {
  const payload = concatBytes([Uint8Array.from([3, 0]), noteBytes(inputNote)])
  return `/s#${(bs58.default ?? bs58).encode(payload)}`
}

function noteBytes(inputNote) {
  return concatBytes([
    Uint8Array.from([Number(inputNote.kind)]),
    wordBytes(inputNote.token).slice(12),
    compactField(inputNote.nonce),
    wordBytes(inputNote.psi),
    wordBytes(inputNote.owner),
    compactField(inputNote.value)
  ])
}

function compactField(value) {
  const bytes = wordBytes(value)
  const first = bytes.findIndex((byte) => byte !== 0)
  const start = first === -1 ? 32 : first
  return concatBytes([Uint8Array.from([start]), bytes.slice(start)])
}

function wordBytes(value) {
  return Uint8Array.from(Buffer.from(word(value).slice(2), 'hex'))
}

function concatBytes(chunks) {
  const out = new Uint8Array(
    chunks.reduce((length, chunk) => length + chunk.length, 0)
  )
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.length
  }
  return out
}
