const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const test = require('node:test')

const {
  createLocalPrivacySignerFromGrumpkinPrivateKey,
  createPayyClient,
  deriveGrumpkinPrivateKey,
  encodeDirectClaimLink,
  encodeEphemeralClaimLink,
  hexToBytes,
  noteCommitment,
  OperationBuilder,
  payyNetworks,
  privacyAddress,
  privacyAddressOwner,
  privacyAddressPrefix,
  zeroHash
} = require('../dist')

function readClient() {
  return {
    getChainId: async () => payyNetworks.dev.chainId,
    getBlockNumber: async () => 9n,
    readContract: async () => `0x${'0'.repeat(63)}7`,
    getLogs: async () => [],
    getTransactionReceipt: async () => null,
    waitForTransactionReceipt: async (args) => ({
      transactionHash: args.hash,
      blockNumber: 9n,
      status: 'success'
    })
  }
}

function uppercaseHex(hex) {
  return `0x${hex.slice(2).toUpperCase()}`
}

test('base client exposes bridge reads and transactions namespace', async () => {
  const client = createPayyClient({
    publicClient: readClient()
  })

  assert.equal(await client.bridge().getRoot(), zeroHash().replace(/0$/, '7'))
  assert.equal(client.privacy, undefined)
  assert.deepEqual(client.transactions(), {})
})

test('base client accepts ethereum-style public client config', async () => {
  const client = createPayyClient({
    publicClient: readClient()
  })

  assert.equal(await client.bridge().getRoot(), zeroHash().replace(/0$/, '7'))
})

test('bridge elementExists requires canonical bool returns', async () => {
  function clientForReturn(response) {
    return createPayyClient({
      publicClient: {
        ...readClient(),
        readContract: async () => response
      }
    })
  }

  assert.equal(
    await clientForReturn(zeroHash()).bridge().elementExists(zeroHash()),
    false
  )
  assert.equal(
    await clientForReturn(`0x${'0'.repeat(63)}1`)
      .bridge()
      .elementExists(zeroHash()),
    true
  )
  await assert.rejects(
    clientForReturn(`0x${'0'.repeat(63)}2`)
      .bridge()
      .elementExists(zeroHash()),
    { code: 'contract_return_malformed' }
  )
})

test('base client exposes explicit local private key builders', async () => {
  async function defaultAddress(client) {
    const account = await client.privacy().defaultAccount()
    assert.notEqual(account, null)
    return account.privacyAddress?.bytes ?? account.bytes
  }

  const baseClient = createPayyClient({
    publicClient: readClient()
  })
  const evmPrivateKey = `0x${'11'.repeat(32)}`
  const evmAddress = await defaultAddress(
    baseClient.withEvmPrivateKey(evmPrivateKey)
  )

  assert.equal(
    await defaultAddress(baseClient.withSecp256k1PrivateKey(evmPrivateKey)),
    evmAddress
  )

  const fixture = JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '../../../../fixtures/payy-evm-client/v3.json'),
      'utf8'
    )
  )
  const signer = createLocalPrivacySignerFromGrumpkinPrivateKey(
    fixture.grumpkin_private_key
  )
  const expected = (await signer.accounts())[0].privacyAddress.bytes

  assert.equal(
    await defaultAddress(
      baseClient.withGrumpkinPrivateKey(fixture.grumpkin_private_key)
    ),
    expected
  )
})

test('privacy client keeps privacy surface when adding evm signer', async () => {
  const signer = createLocalPrivacySignerFromGrumpkinPrivateKey(
    `0x${'11'.repeat(32)}`
  )
  const privacyPrepClient = createPayyClient({
    publicClient: readClient()
  }).privacySigner(signer)
  const checkpoint = {
    privacyAccount: (await signer.accounts())[0].privacyAddress,
    token: '0x0000000000000000000000000000000000000001',
    ownedNote: null,
    checkedBlock: 9n
  }
  await privacyPrepClient.privacy().setCheckpoint(checkpoint)

  const privacyClient = privacyPrepClient.evmSigner({
    getChainId: async () => payyNetworks.dev.chainId,
    getAddress: async () => '0x00000000000000000000000000000000000000aa',
    sendTransaction: async () => zeroHash()
  })

  const privacy = privacyClient.privacy()
  assert.equal(typeof privacy.accounts, 'function')
  assert.equal(typeof privacy.send, 'function')
  assert.deepEqual(await privacy.accounts(), await signer.accounts())
  assert.equal(
    privacyClient.inner.checkpoints,
    privacyPrepClient.inner.checkpoints
  )
  assert.equal(
    privacyClient.inner.checkpoints.get(
      `${checkpoint.privacyAccount.bytes}:${checkpoint.token}`
    ),
    checkpoint
  )
})

test('local grumpkin signer validates scalar range', () => {
  const modulus =
    21888242871839275222246405745257275088548364400416034343698204186575808495617n
  const modulusHex = `0x${modulus.toString(16).padStart(64, '0')}`
  const aboveModulusHex = `0x${(modulus + 1n).toString(16).padStart(64, '0')}`

  assert.throws(
    () => createLocalPrivacySignerFromGrumpkinPrivateKey(zeroHash()),
    { code: 'field_out_of_range' }
  )
  assert.throws(
    () => createLocalPrivacySignerFromGrumpkinPrivateKey(modulusHex),
    { code: 'field_out_of_range' }
  )
  assert.throws(
    () => createLocalPrivacySignerFromGrumpkinPrivateKey(aboveModulusHex),
    { code: 'field_out_of_range' }
  )
})

test('hex helpers reject malformed and short private key values', () => {
  assert.throws(() => hexToBytes('0xgg'), { code: 'invalid_hex' })
  assert.throws(() => hexToBytes('0x123'), { code: 'invalid_hex' })
  assert.throws(() => deriveGrumpkinPrivateKey('0x11'), {
    code: 'invalid_hex'
  })
  assert.throws(
    () =>
      createLocalPrivacySignerFromGrumpkinPrivateKey(`0x${'11'.repeat(31)}`),
    { code: 'invalid_hex' }
  )
})

test('public hex identifiers are canonicalized before identity checks', async () => {
  const signer = createLocalPrivacySignerFromGrumpkinPrivateKey(
    `0x${'11'.repeat(32)}`
  )
  const account = (await signer.accounts())[0].privacyAddress
  const token = `0x${'ab'.repeat(20)}`
  const prefixedReadClient = {
    ...readClient(),
    readContract: async () => zeroHash()
  }
  const client = createPayyClient({
    publicClient: prefixedReadClient
  })
    .privacySigner(signer)
    .privacy()
  const checkpoint = {
    privacyAccount: { bytes: uppercaseHex(account.bytes) },
    token: uppercaseHex(token),
    ownedNote: null,
    checkedBlock: 9n
  }

  assert.equal(privacyAddress(uppercaseHex(account.bytes)).bytes, account.bytes)
  await assert.doesNotReject(
    signer.signTxCommitment({
      privacyAccount: { bytes: uppercaseHex(account.bytes) },
      txCommitment: uppercaseHex(`0x${'ab'.repeat(32)}`)
    })
  )
  await client.setCheckpoint(checkpoint)

  const state = await client.notes().get({
    privacyAccount: { bytes: account.bytes },
    token
  })
  const prefix = await privacyAddressPrefix(account)
  await assert.doesNotReject(
    client.incoming().list({
      privacyAccount: { bytes: uppercaseHex(account.bytes) },
      privacyAddressPrefix: { bytes: uppercaseHex(prefix.bytes) }
    })
  )

  assert.equal(state.privacyAccount.bytes, account.bytes)
  assert.equal(state.token, token)
  assert.equal(
    client.inner.checkpoints.get(`${account.bytes}:${token}`).token,
    token
  )
})

test('withOwnedInput replaces previous owned input override', () => {
  const first = {
    kind: 'padding',
    data: { recentRoot: `0x${'11'.repeat(32)}` }
  }
  const second = {
    kind: 'padding',
    data: { recentRoot: `0x${'22'.repeat(32)}` }
  }

  const builder = new OperationBuilder({}, 'mint', {})
    .withOwnedInput(first)
    .withOwnedInput(second)

  assert.deepEqual(builder.resolvedInputs, [second])
})

test('local grumpkin signer reports account mismatch as typed error', async () => {
  const signer = createLocalPrivacySignerFromGrumpkinPrivateKey(
    `0x${'11'.repeat(32)}`
  )
  const wrongAccount = privacyAddress(zeroHash())
  const encryptedNote = Array.from({ length: 5 }, zeroHash)
  const encryptedKey = Array.from({ length: 4 }, zeroHash)
  const txnData = {
    verificationKeyHash: zeroHash(),
    senderEncryptedNote: encryptedNote,
    recipientEncryptedNote: encryptedNote,
    senderChainEncryptedKey: Array.from({ length: 3 }, zeroHash),
    recipientChainEncryptedKey: Array.from({ length: 3 }, zeroHash),
    userEncryptedKey: encryptedKey,
    recipientEncryptedKey: encryptedKey,
    memo: zeroHash()
  }

  await assert.rejects(
    signer.signTxCommitment({
      privacyAccount: wrongAccount,
      txCommitment: zeroHash()
    }),
    {
      code: 'privacy_account_mismatch'
    }
  )
  await assert.rejects(
    signer.decryptSenderNote({ privacyAccount: wrongAccount, txnData }),
    {
      code: 'privacy_account_mismatch'
    }
  )
})

test('privacy address helpers report malformed addresses as typed errors', async () => {
  const malformed = privacyAddress(`0x40${'00'.repeat(31)}`)

  await assert.rejects(privacyAddressOwner(malformed), {
    code: 'invalid_privacy_address'
  })
  await assert.rejects(privacyAddressPrefix(malformed), {
    code: 'invalid_privacy_address'
  })
})

test('only send builders expose link generation', () => {
  const client = createPayyClient({
    publicClient: readClient()
  })
    .privacySigner({
      accounts: async () => [],
      signTxCommitment: async () => {
        throw new Error('unused')
      },
      decryptSenderNote: async () => null,
      decryptRecipientNote: async () => null,
      generateEphemeralKey: async () => {
        throw new Error('unused')
      }
    })
    .privacy()
  const account = privacyAddress(zeroHash())
  const token = `0x${'11'.repeat(20)}`
  const note = {
    kind: 1n,
    token: 1n,
    nonce: 0n,
    psi: 1n,
    owner: 1n,
    value: 1n
  }
  const incomingNote = {
    note,
    commitment: zeroHash(),
    nullifier: zeroHash(),
    sourcePosition: {
      blockNumber: 1n,
      transactionIndex: 0,
      logIndex: 0
    },
    sourceTxHash: zeroHash(),
    sourceBridgeTxHash: zeroHash(),
    status: 'claimable'
  }

  assert.equal(
    typeof client.mint({
      privacyAccount: account,
      evmAccount: token,
      token,
      amount: 1n
    }).link,
    'undefined'
  )
  assert.equal(
    typeof client.burn({
      privacyAccount: account,
      token,
      amount: 1n,
      recipient: token
    }).link,
    'undefined'
  )
  assert.equal(typeof client.claim().note(incomingNote).link, 'undefined')
  assert.equal(
    typeof client.send().to({
      privacyAccount: account,
      token,
      amount: 1n,
      recipient: account
    }).link,
    'function'
  )
  assert.equal(
    typeof client.send().ephemeral({
      privacyAccount: account,
      token,
      amount: 1n
    }).link,
    'function'
  )
})

test('base bridge client exposes raw reads without transaction request helper', () => {
  const client = createPayyClient({
    publicClient: readClient()
  })

  assert.equal(client.bridge().transactionRequest, undefined)
})

test('base client parses v3 direct links', async () => {
  const note = {
    kind: 1n,
    token: 1n,
    nonce: 0n,
    psi: 1n,
    owner: 1n,
    value: 1n
  }
  const link = encodeDirectClaimLink({
    recipient: privacyAddress(zeroHash()),
    note,
    commitment: zeroHash()
  })
  const client = createPayyClient({
    publicClient: readClient()
  })

  assert.equal(
    (await client.links().parse(link.value)).claimSourceKind,
    'direct'
  )
})

test('shared v3 fixture matches npm crypto and links', async () => {
  const fixture = JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '../../../../fixtures/payy-evm-client/v3.json'),
      'utf8'
    )
  )
  const signer = createLocalPrivacySignerFromGrumpkinPrivateKey(
    fixture.grumpkin_private_key
  )
  const address = (await signer.accounts())[0].privacyAddress
  const client = createPayyClient({
    publicClient: readClient()
  })

  assert.equal(payyNetworks.dev.chainId, fixture.network_presets.dev.chain_id)
  assert.equal(
    payyNetworks.dev.privacyBridge,
    fixture.network_presets.dev.privacy_bridge
  )
  assert.equal(
    payyNetworks.testnet.chainId,
    fixture.network_presets.testnet.chain_id
  )
  assert.equal(
    payyNetworks.testnet.privacyBridge,
    fixture.network_presets.testnet.privacy_bridge
  )

  assert.equal(address.bytes, fixture.privacy_address.bytes)
  assert.equal(
    `0x${(await privacyAddressOwner(address)).toString(16).padStart(64, '0')}`,
    fixture.privacy_address.owner
  )
  assert.equal(
    (await privacyAddressPrefix(address)).bytes,
    fixture.privacy_address.prefix6
  )

  const note = Object.fromEntries(
    Object.entries(fixture.note)
      .filter(([key]) => !['commitment', 'nullifier'].includes(key))
      .map(([key, value]) => [key, BigInt(value)])
  )
  const leadingZeroSigner = createLocalPrivacySignerFromGrumpkinPrivateKey(
    fixture.leading_zero_prefix.grumpkin_private_key
  )
  const leadingZeroAddress = (await leadingZeroSigner.accounts())[0]
    .privacyAddress

  assert.equal(await noteCommitment(note), fixture.note.commitment)
  assert.equal(
    leadingZeroAddress.bytes,
    fixture.leading_zero_prefix.privacy_address
  )
  assert.equal(
    `0x${(await privacyAddressOwner(leadingZeroAddress)).toString(16).padStart(64, '0')}`,
    fixture.leading_zero_prefix.owner
  )
  assert.equal(
    (await privacyAddressPrefix(leadingZeroAddress)).bytes,
    fixture.leading_zero_prefix.prefix6
  )
  assert.equal(
    encodeDirectClaimLink(
      {
        recipient: address,
        note,
        commitment: fixture.note.commitment
      },
      'hello world'
    ).value,
    fixture.direct_send_delivery.link
  )
  assert.equal(
    encodeEphemeralClaimLink(
      {
        note,
        commitment: fixture.note.commitment,
        ephemeralPrivateKey: fixture.incoming_transfer.ephemeral_private_key
      },
      'handoff'
    ).value,
    fixture.incoming_transfer.link
  )
  assert.equal(
    (await client.links().parse(fixture.direct_send_delivery.link))
      .claimSourceKind,
    'direct'
  )
  assert.equal(
    (await client.links().parse(fixture.incoming_transfer.link))
      .claimSourceKind,
    'ephemeral'
  )
  assert.deepEqual(fixture.prepared_call_public_fields, [
    'operation',
    'chainId',
    'bridgeRequest',
    'verificationKeyHash',
    'proof',
    'publicInputs',
    'txCommitment',
    'statePreview'
  ])
})
