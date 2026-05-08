import { BridgeClient, externalTransferTopic } from './bridge'
import { PayyClientError } from './errors'
import {
  LinksClient,
  encodeDirectClaimLink,
  encodeEphemeralClaimLink
} from './links'
import {
  createLocalPrivacySigner,
  createLocalPrivacySignerFromGrumpkinPrivateKey
} from './localSigner'
import {
  BbJsProvingBackend,
  type NoirCircuitInput,
  type ProvingBackend
} from './proving'
import { defaultPrivacyBridge } from './network'
import {
  computeMerkleRoot,
  encryptChainKey,
  encryptKeyForPublicKey,
  encryptedKeyHash,
  encryptedNote,
  bigIntToB256,
  ephemeralOwner,
  fieldMod,
  firstNonceHash,
  nextNonceHash,
  noteCommitment,
  noteNullifier,
  publicKeyFromPrivacyAddress,
  privacyAddressOwner,
  privacyAddressPrefix,
  randomNonZeroField,
  randomU240,
  receivePrefix,
  txCommitment,
  userEncryptedKeyHash,
  zeroHash
} from './crypto'
import type {
  Address,
  B256,
  BridgeTransactionRequest,
  ClaimLink,
  ConfirmedOperationResult,
  DirectSendDelivery,
  EvmAccount,
  Hex,
  IncomingNote,
  IncomingTransfer,
  Note,
  OwnerSignature,
  OwnedNoteState,
  ParsedClaimLink,
  PayyEvmLogFilter,
  PayyEvmReadClient,
  PayyEvmSubmitter,
  PayyEvmTransactionRequest,
  PayyRawTransactionSubmitter,
  PreparedOperationResult,
  PrivacyAccount,
  PrivacyAddress,
  PrivacyAddressPrefix,
  PrivacyOperationKind,
  PrivacySigner,
  PrivateBalanceState,
  ClaimResolvedInputs,
  OwnedNote,
  ResolvedInputNote,
  SubmittedOperationResult
} from './types'
import {
  addressToBigInt,
  b256ToBigInt,
  canonicalAddress,
  canonicalHex,
  ensureAmountNonZero,
  evmAccountAddress,
  hexToBytes,
  privacyAddress,
  privacyAccountAddress
} from './utils'

export type PayyClientConfig = {
  readonly publicClient: PayyEvmReadClient
  readonly privacyBridge?: Address
  readonly evmSubmitter?: PayyEvmSubmitter
  readonly rawTransactionSubmitter?: PayyRawTransactionSubmitter
  readonly provingBackend?: ProvingBackend
}

type ResolvedPrivacyAccount = {
  readonly privacyAddress: PrivacyAddress
  readonly privacyAccount: PrivacyAccount
  readonly signer: PrivacySigner
}

export type OwnedNoteGetParams = {
  readonly privacyAccount: PrivacyAccount
  readonly token: Address
}

export type IncomingListParams = {
  readonly privacyAccount: PrivacyAccount
  readonly privacyAddressPrefix?: PrivacyAddressPrefix
  readonly fromBlock?: bigint
  readonly toBlock?: bigint
  readonly includeSpent?: boolean
  readonly pollIntervalMs?: number
}

export type IncomingWatchResult = {
  readonly nextFromBlock: bigint
  readonly error?: unknown
}

export type MintParams = {
  readonly privacyAccount: PrivacyAccount
  readonly evmAccount: EvmAccount
  readonly token: Address
  readonly amount: bigint
}

export type BurnParams = {
  readonly privacyAccount: PrivacyAccount
  readonly token: Address
  readonly amount: bigint
  readonly evmRecipient: Address
}

export type DirectSendParams = {
  readonly privacyAccount: PrivacyAccount
  readonly token: Address
  readonly amount: bigint
  readonly recipient: PrivacyAddress
  readonly bridgeMemo?: B256
}

export type EphemeralSendParams = {
  readonly privacyAccount: PrivacyAccount
  readonly token: Address
  readonly amount: bigint
  readonly bridgeMemo?: B256
}

type ClientInner = {
  privacyBridge: Address
  readClient: PayyEvmReadClient
  evmSubmitter?: PayyEvmSubmitter
  rawTransactionSubmitter?: PayyRawTransactionSubmitter
  privacySigner?: PrivacySigner
  provingBackend: ProvingBackend
  checkpoints: Map<string, OwnedNoteState>
}

type SpendInput = {
  readonly note: Note
  readonly merklePath: readonly B256[]
  readonly recentRoot: B256
  readonly commitment: B256
  readonly nullifier: B256
}

type OwnedSpendInput =
  | {
      readonly kind: 'real'
      readonly input: SpendInput
    }
  | {
      readonly kind: 'padding'
      readonly merklePath: readonly B256[]
      readonly recentRoot: B256
    }

type ClaimInputs = {
  readonly own: OwnedSpendInput
  readonly incoming: SpendInput
}

type ProvenCall = {
  readonly chainId: number
  readonly verificationKeyHash: B256
  readonly proof: Hex
  readonly publicInputs: readonly B256[]
  readonly txCommitment: B256
  readonly recentRoot: B256
  readonly inputCommitments: readonly B256[]
  readonly inputNullifiers: readonly B256[]
  readonly outputCommitments: readonly B256[]
  readonly userEncryptedKey: readonly [B256, B256, B256, B256]
  readonly recipientEncryptedKey: readonly [B256, B256, B256, B256]
}

const BN254_SCALAR_MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n
const ADDRESS_FIELD_BOUND = 1n << 160n
const MAX_NOTE_VALUE = 1n << 240n

export class BasePayyClient {
  inner: ClientInner

  constructor(config: PayyClientConfig) {
    this.inner = {
      privacyBridge: config.privacyBridge ?? defaultPrivacyBridge,
      readClient: config.publicClient,
      evmSubmitter: config.evmSubmitter,
      rawTransactionSubmitter: config.rawTransactionSubmitter,
      provingBackend: config.provingBackend ?? new BbJsProvingBackend({}),
      checkpoints: new Map()
    }
  }

  bridge(): BridgeClient {
    return new BridgeClient(this.inner.privacyBridge, this.inner.readClient)
  }

  links(): LinksClient {
    return new LinksClient()
  }

  transactions(): Record<string, never> {
    return {}
  }

  privacySigner(signer: PrivacySigner): PrivacyPayyClient {
    return new PrivacyPayyClient({
      ...this.inner,
      privacySigner: signer
    })
  }

  evmSigner(submitter: PayyEvmSubmitter): EvmPayyClient {
    return new EvmPayyClient({
      ...this.inner,
      evmSubmitter: submitter
    })
  }

  withEvmPrivateKey(evmPrivateKey: B256): PrivacyPayyClient {
    const privacySigner = createLocalPrivacySigner(evmPrivateKey)
    const evmSubmitter =
      this.inner.rawTransactionSubmitter === undefined
        ? undefined
        : new RawLocalEvmSubmitter(
            evmPrivateKey,
            this.inner.rawTransactionSubmitter
          )
    return new PrivacyPayyClient({
      ...this.inner,
      privacySigner,
      evmSubmitter
    })
  }

  withSecp256k1PrivateKey(evmPrivateKey: B256): PrivacyPayyClient {
    return this.withEvmPrivateKey(evmPrivateKey)
  }

  withGrumpkinPrivateKey(grumpkinPrivateKey: B256): PrivacyPayyClient {
    return this.privacySigner(
      createLocalPrivacySignerFromGrumpkinPrivateKey(grumpkinPrivateKey)
    )
  }
}

export class EvmPayyClient extends BasePayyClient {
  constructor(inner: ClientInner) {
    super({
      publicClient: inner.readClient,
      privacyBridge: inner.privacyBridge,
      evmSubmitter: inner.evmSubmitter,
      rawTransactionSubmitter: inner.rawTransactionSubmitter,
      provingBackend: inner.provingBackend
    })
    this.inner = inner
  }
}

export class PrivacyPayyClient extends BasePayyClient {
  constructor(inner: ClientInner) {
    super({
      publicClient: inner.readClient,
      privacyBridge: inner.privacyBridge,
      evmSubmitter: inner.evmSubmitter,
      rawTransactionSubmitter: inner.rawTransactionSubmitter,
      provingBackend: inner.provingBackend
    })
    this.inner = inner
  }

  privacy(): PrivacyNamespace {
    return new PrivacyNamespace(this.inner)
  }

  evmSigner(submitter: PayyEvmSubmitter): PrivacyPayyClient {
    return new PrivacyPayyClient({
      ...this.inner,
      evmSubmitter: submitter
    })
  }
}

export class PrivacyNamespace {
  inner: ClientInner

  constructor(inner: ClientInner) {
    this.inner = inner
  }

  bridge(): BridgeClient {
    return new BridgeClient(this.inner.privacyBridge, this.inner.readClient)
  }

  links(): LinksClient {
    return new LinksClient()
  }

  async accounts(): Promise<readonly PrivacyAccount[]> {
    return this.privacySignerRequired().accounts()
  }

  async defaultAccount(): Promise<PrivacyAccount | null> {
    return (await this.accounts())[0] ?? null
  }

  async setCheckpoint(checkpoint: OwnedNoteState): Promise<void> {
    const normalized = normalizeOwnedNoteState(checkpoint)
    await validateCheckpoint(normalized)
    this.inner.checkpoints.set(
      cacheKey(normalized.privacyAccount, normalized.token),
      normalized
    )
  }

  notes(): NotesClient {
    return new NotesClient(this)
  }

  balances(): BalancesClient {
    return new BalancesClient(this)
  }

  incoming(): IncomingClient {
    return new IncomingClient(this)
  }

  mint(params: MintParams): OperationBuilder<void> {
    return new OperationBuilder(this, 'mint', params)
  }

  burn(params: BurnParams): OperationBuilder<void> {
    return new OperationBuilder(this, 'burn', params)
  }

  send(): SendClient {
    return new SendClient(this)
  }

  claim(): ClaimClient {
    return new ClaimClient(this)
  }

  privacySignerRequired(): PrivacySigner {
    if (this.inner.privacySigner === undefined) {
      throw new PayyClientError(
        'missing_privacy_signer',
        'privacy signer required'
      )
    }
    return this.inner.privacySigner
  }

  async resolveOwnedNote(
    params: OwnedNoteGetParams,
    checkpoint?: OwnedNoteState
  ): Promise<OwnedNoteState> {
    const resolved = resolvePrivacyAccount(this, params.privacyAccount)
    const token = canonicalAddress(params.token)
    const selected =
      checkpoint
      ?? this.inner.checkpoints.get(cacheKey(resolved.privacyAddress, token))
    if (selected !== undefined) {
      const normalized = normalizeOwnedNoteState(selected)
      if (
        normalized.privacyAccount.bytes !== resolved.privacyAddress.bytes
        || normalized.token !== token
      ) {
        throw new PayyClientError(
          'checkpoint_mismatch',
          'checkpoint key mismatch'
        )
      }
      try {
        await validateCheckpoint(normalized)
      } catch (error) {
        if (isCheckpointShapeError(error)) {
          return this.fullOwnedNoteLookup(resolved, token)
        }
        throw error
      }
      return this.validateAndRefreshCheckpoint(normalized, resolved)
    }
    return this.fullOwnedNoteLookup(resolved, token)
  }

  private async fullOwnedNoteLookup(
    resolved: ResolvedPrivacyAccount,
    token: Address
  ): Promise<OwnedNoteState> {
    const owner = await privacyAddressOwner(resolved.privacyAddress)
    const nonceHash = await firstNonceHash(1n, addressToBigInt(token), owner)
    const state = {
      privacyAccount: resolved.privacyAddress,
      token,
      ownedNote: await this.scanNonceChain(resolved, token, nonceHash),
      checkedBlock: await this.inner.readClient.getBlockNumber()
    }
    await this.setCheckpoint(state)
    return state
  }

  private async scanNonceChain(
    resolved: ResolvedPrivacyAccount,
    token: Address,
    nonceHash: B256
  ): Promise<OwnedNote | null> {
    let cursor = nonceHash
    for (;;) {
      const note = await this.lookupOneNote(resolved, token, cursor)
      if (note === null) {
        return null
      }
      if (!(await this.bridge().elementExists(note.nullifier))) {
        return note
      }
      cursor = await nextNonceHash(
        note.note.kind,
        note.note.token,
        note.note.owner,
        note.note.nonce + 1n,
        note.note.psi
      )
    }
  }

  private async lookupOneNote(
    resolved: ResolvedPrivacyAccount,
    token: Address,
    nonceHash: B256
  ): Promise<OwnedNote | null> {
    const sourceBridgeTxHash =
      await this.bridge().getTxnHashByNonceHash(nonceHash)
    if (sourceBridgeTxHash === null) {
      return null
    }
    const note = await resolved.signer.decryptSenderNote({
      privacyAccount: resolved.privacyAccount,
      txnData: await this.bridge().getTxnData(sourceBridgeTxHash)
    })
    if (note === null) {
      return null
    }
    if (
      note.token !== addressToBigInt(token)
      || note.owner !== (await privacyAddressOwner(resolved.privacyAddress))
    ) {
      return null
    }
    return {
      note,
      commitment: await noteCommitment(note),
      nullifier: await noteNullifier(note),
      nonceHash,
      sourceBridgeTxHash
    }
  }

  private async validateAndRefreshCheckpoint(
    checkpoint: OwnedNoteState,
    resolved: ResolvedPrivacyAccount
  ): Promise<OwnedNoteState> {
    if (checkpoint.ownedNote === null) {
      return this.fullOwnedNoteLookup(resolved, checkpoint.token)
    }
    const refreshed = await this.lookupOneNote(
      resolved,
      checkpoint.token,
      checkpoint.ownedNote.nonceHash
    )
    if (
      refreshed === null
      || refreshed.commitment !== checkpoint.ownedNote.commitment
      || refreshed.nullifier !== checkpoint.ownedNote.nullifier
      || !sameNote(refreshed.note, checkpoint.ownedNote.note)
    ) {
      return this.fullOwnedNoteLookup(resolved, checkpoint.token)
    }
    if (!(await this.bridge().elementExists(checkpoint.ownedNote.nullifier))) {
      const fresh = {
        ...checkpoint,
        checkedBlock: await this.inner.readClient.getBlockNumber()
      }
      await this.setCheckpoint(fresh)
      return fresh
    }
    const next = await nextNonceHash(
      checkpoint.ownedNote.note.kind,
      checkpoint.ownedNote.note.token,
      checkpoint.ownedNote.note.owner,
      checkpoint.ownedNote.note.nonce + 1n,
      checkpoint.ownedNote.note.psi
    )
    const fresh = {
      privacyAccount: checkpoint.privacyAccount,
      token: checkpoint.token,
      ownedNote: await this.scanNonceChain(resolved, checkpoint.token, next),
      checkedBlock: await this.inner.readClient.getBlockNumber()
    }
    await this.setCheckpoint(fresh)
    return fresh
  }
}

export class NotesClient {
  private readonly client: PrivacyNamespace
  private readonly checkpoint?: OwnedNoteState

  constructor(client: PrivacyNamespace, checkpoint?: OwnedNoteState) {
    this.client = client
    this.checkpoint = checkpoint
  }

  withCheckpoint(checkpoint: OwnedNoteState): NotesClient {
    return new NotesClient(this.client, checkpoint)
  }

  async get(params: OwnedNoteGetParams): Promise<OwnedNoteState> {
    return this.client.resolveOwnedNote(params, this.checkpoint)
  }
}

export class BalancesClient {
  private readonly client: PrivacyNamespace
  private readonly checkpoint?: OwnedNoteState

  constructor(client: PrivacyNamespace, checkpoint?: OwnedNoteState) {
    this.client = client
    this.checkpoint = checkpoint
  }

  withCheckpoint(checkpoint: OwnedNoteState): BalancesClient {
    return new BalancesClient(this.client, checkpoint)
  }

  async get(params: OwnedNoteGetParams): Promise<PrivateBalanceState> {
    const ownedNoteState = await this.client.resolveOwnedNote(
      params,
      this.checkpoint
    )
    return {
      ownedNoteState,
      balance:
        ownedNoteState.ownedNote === null
          ? null
          : {
              privacyAccount: ownedNoteState.privacyAccount,
              token: ownedNoteState.token,
              spendable: ownedNoteState.ownedNote.note.value
            }
    }
  }
}

export class IncomingClient {
  private readonly client: PrivacyNamespace

  constructor(client: PrivacyNamespace) {
    this.client = client
  }

  async list(params: IncomingListParams): Promise<readonly IncomingNote[]> {
    const resolved = resolvePrivacyAccount(this.client, params.privacyAccount)
    const prefix = await privacyAddressPrefix(resolved.privacyAddress)
    if (
      params.privacyAddressPrefix !== undefined
      && canonicalHex(params.privacyAddressPrefix.bytes) !== prefix.bytes
    ) {
      throw new PayyClientError('prefix_mismatch', 'privacy prefix mismatch')
    }
    const filter: PayyEvmLogFilter = {
      address: this.client.inner.privacyBridge,
      fromBlock: params.fromBlock ?? 0n,
      toBlock:
        params.toBlock ?? (await this.client.inner.readClient.getBlockNumber()),
      topics: [externalTransferTopic(), prefixToTopic(prefix)]
    }
    const logs = await this.client.inner.readClient.getLogs(filter)
    const notes: IncomingNote[] = []
    const owner = await privacyAddressOwner(resolved.privacyAddress)
    for (const log of logs) {
      const sourceBridgeTxHash = log.topics[2] ?? log.transactionHash
      const txnData = await this.client.bridge().getTxnData(sourceBridgeTxHash)
      const note = await resolved.signer.decryptRecipientNote({
        privacyAccount: resolved.privacyAccount,
        txnData
      })
      if (note === null) {
        continue
      }
      if (note.owner !== owner) {
        continue
      }
      const nullifier = await noteNullifier(note)
      const spent = await this.client.bridge().elementExists(nullifier)
      if (spent && params.includeSpent !== true) {
        continue
      }
      notes.push({
        note,
        commitment: await noteCommitment(note),
        nullifier,
        sourcePosition: {
          blockNumber: log.blockNumber,
          transactionIndex: log.transactionIndex,
          logIndex: log.logIndex
        },
        sourceTxHash: log.transactionHash,
        sourceBridgeTxHash,
        status: spent ? 'spent' : 'claimable'
      })
    }
    return notes.sort(compareIncomingNotes)
  }

  async watch(
    params: IncomingListParams,
    callback: (note: IncomingNote) => Promise<void> | void
  ): Promise<IncomingWatchResult> {
    let fromBlock = params.fromBlock ?? 0n
    let fullyProcessed = fromBlock - 1n
    const pollIntervalMs = params.pollIntervalMs ?? 3000
    for (;;) {
      const head = await this.client.inner.readClient.getBlockNumber()
      const toBlock =
        params.toBlock === undefined
          ? head
          : params.toBlock < head
            ? params.toBlock
            : head
      if (fromBlock <= toBlock) {
        let notes: readonly IncomingNote[]
        try {
          notes = await this.list({
            ...params,
            fromBlock,
            toBlock
          })
        } catch (error) {
          return { nextFromBlock: fromBlock, error }
        }
        for (const note of notes) {
          try {
            await callback(note)
          } catch (error) {
            return {
              nextFromBlock: note.sourcePosition.blockNumber,
              error
            }
          }
        }
        fullyProcessed = toBlock
        fromBlock = toBlock + 1n
      }
      if (params.toBlock !== undefined && fromBlock > params.toBlock) {
        return { nextFromBlock: fullyProcessed + 1n }
      }
      await sleep(pollIntervalMs)
    }
  }
}

export class SendClient {
  private readonly client: PrivacyNamespace

  constructor(client: PrivacyNamespace) {
    this.client = client
  }

  to(params: DirectSendParams): SendOperationBuilder<DirectSendDelivery> {
    return new SendOperationBuilder(this.client, 'transfer_send', params)
  }

  ephemeral(
    params: EphemeralSendParams
  ): SendOperationBuilder<IncomingTransfer> {
    return new SendOperationBuilder(this.client, 'transfer_send', params)
  }
}

export class ClaimClient {
  private readonly client: PrivacyNamespace
  private readonly selectedAccount?: PrivacyAccount

  constructor(client: PrivacyNamespace, selectedAccount?: PrivacyAccount) {
    this.client = client
    this.selectedAccount = selectedAccount
  }

  account(privacyAccountValue: PrivacyAccount): ClaimClient {
    return new ClaimClient(this.client, privacyAccountValue)
  }

  note(note: IncomingNote): OperationBuilder<IncomingNote> {
    return new OperationBuilder(this.client, 'transfer_claim', {
      kind: 'note',
      note,
      account: this.selectedAccount
    })
  }

  ephemeral(transfer: IncomingTransfer): OperationBuilder<IncomingTransfer> {
    return new OperationBuilder(this.client, 'transfer_claim', {
      kind: 'ephemeral',
      transfer,
      account: this.selectedAccount
    })
  }

  link(link: ClaimLink): OperationBuilder<ParsedClaimLink> {
    return new OperationBuilder(this.client, 'transfer_claim', {
      kind: 'link',
      link,
      account: this.selectedAccount
    })
  }
}

export class OperationBuilder<TPayload> {
  protected readonly client: PrivacyNamespace
  protected readonly operation: PrivacyOperationKind
  protected readonly params: unknown
  protected readonly checkpoint?: OwnedNoteState
  protected readonly resolvedInputs: readonly ResolvedInputNote[]
  protected readonly claimInputs?: ClaimResolvedInputs

  constructor(
    client: PrivacyNamespace,
    operation: PrivacyOperationKind,
    params: unknown,
    checkpoint?: OwnedNoteState,
    resolvedInputs: readonly ResolvedInputNote[] = [],
    claimInputs?: ClaimResolvedInputs
  ) {
    this.client = client
    this.operation = operation
    this.params = params
    this.checkpoint = checkpoint
    this.resolvedInputs = resolvedInputs
    this.claimInputs = claimInputs
  }

  withCheckpoint(checkpoint: OwnedNoteState): OperationBuilder<TPayload> {
    return new OperationBuilder(
      this.client,
      this.operation,
      this.params,
      checkpoint,
      this.resolvedInputs,
      this.claimInputs
    )
  }

  withOwnedInput(input: ResolvedInputNote): OperationBuilder<TPayload> {
    return new OperationBuilder(
      this.client,
      this.operation,
      this.params,
      this.checkpoint,
      [input],
      this.claimInputs
    )
  }

  withClaimInputs(inputs: ClaimResolvedInputs): OperationBuilder<TPayload> {
    return new OperationBuilder(
      this.client,
      this.operation,
      this.params,
      this.checkpoint,
      this.resolvedInputs,
      inputs
    )
  }

  async prepare(): Promise<Prepared<TPayload>> {
    if (this.operation === 'mint') {
      const params = this.params as MintParams
      const submitter =
        typeof params.evmAccount === 'string'
          ? undefined
          : params.evmAccount.submitter
      return new Prepared(this.client, await this.prepareMint(), submitter)
    }
    if (this.operation === 'burn') {
      return new Prepared(this.client, await this.prepareBurn())
    }
    if ('recipient' in (this.params as DirectSendParams)) {
      return new Prepared(this.client, await this.prepareDirectSend())
    }
    if ('amount' in (this.params as EphemeralSendParams)) {
      return new Prepared(this.client, await this.prepareEphemeralSend())
    }
    return new Prepared(this.client, await this.prepareClaim())
  }

  private async prepareMint(): Promise<PreparedOperationResult<TPayload>> {
    const params = this.params as MintParams
    ensureAmountNonZero(params.amount)
    ensureU240(params.amount)
    const input = await this.resolveOwnedSpendInput(
      params.privacyAccount,
      params.token,
      true
    )
    const inputNote = ownedInputNote(input)
    const evmAddress = evmAccountAddress(params.evmAccount)
    const outputNote: Note = {
      kind: 1n,
      token: addressToBigInt(params.token),
      nonce: inputNote.kind === 0n ? 0n : inputNote.nonce + 1n,
      psi: await randomNonZeroField(),
      owner: await privacyAddressOwner(
        privacyAccountAddress(params.privacyAccount)
      ),
      value: inputNote.value + params.amount
    }
    ensureU240(outputNote.value)
    const encrypted = await this.encryptionForAccount(
      outputNote,
      params.privacyAccount
    )
    const chainId = await this.client.inner.readClient.getChainId()
    const signed = await this.mintTxCommitment(
      chainId,
      input,
      outputNote,
      evmAddress,
      encrypted.userKey
    )
    const ownerSignature = await this.signOwner(params.privacyAccount, signed)
    const call = await this.proveCircuit(
      'mint',
      {
        input_note: noteInput(inputNote),
        input_merkle_path: merklePathInput(ownedMerklePath(input)),
        output_note: noteInput(outputNote),
        owner_signature: ownerSignatureInput(ownerSignature),
        symmetric_key: field(encrypted.symmetricKey),
        ...this.commonPublicInputs(
          chainId,
          ownedRecentRoot(input),
          [ownedNullifier(input), zeroHash()],
          [await noteCommitment(outputNote), zeroHash()],
          await outputNonceHash(inputNote, outputNote),
          userEncryptedKeyHash(encrypted.userKey),
          0n,
          encrypted.encryptedNote,
          zeroWordArray5(),
          encrypted.chainKey,
          zeroWordArray3(),
          await this.chainPublicKeyInput()
        ),
        token: field(outputNote.token),
        burn_recipient: field(0n),
        value: field(outputNote.value - inputNote.value),
        mint_from: field(addressToBigInt(evmAddress)),
        receive_prefix: field(0n)
      },
      chainId,
      signed,
      ownedRecentRoot(input),
      [ownedCommitment(input)],
      [ownedNullifier(input)],
      [await noteCommitment(outputNote)],
      encrypted.userKey,
      zeroWordArray4()
    )
    const request = bridgeTransactionRequest(
      this.client.inner.privacyBridge,
      this.client
        .bridge()
        .encodeMintCall(
          call.verificationKeyHash,
          call.proof,
          call.publicInputs,
          call.userEncryptedKey
        ),
      evmAddress
    )
    return this.preparedResult(
      'mint',
      params.privacyAccount,
      params.token,
      request,
      call,
      undefined as TPayload
    )
  }

  private async prepareBurn(): Promise<PreparedOperationResult<TPayload>> {
    const params = this.params as BurnParams
    ensureAmountNonZero(params.amount)
    ensureU240(params.amount)
    if (addressToBigInt(params.evmRecipient) === 0n) {
      throw new PayyClientError('evm_recipient_zero', 'EVM recipient is zero')
    }
    const input = await this.resolveOwnedSpendInput(
      params.privacyAccount,
      params.token,
      false
    )
    if (input.kind !== 'real') {
      throw new PayyClientError('missing_owned_note', 'owned note required')
    }
    ensureBalance(input.input.note, params.amount)
    const outputNote: Note = {
      kind: 1n,
      token: input.input.note.token,
      nonce: input.input.note.nonce + 1n,
      psi: await randomNonZeroField(),
      owner: input.input.note.owner,
      value: input.input.note.value - params.amount
    }
    const encrypted = await this.encryptionForAccount(
      outputNote,
      params.privacyAccount
    )
    const chainId = await this.client.inner.readClient.getChainId()
    const signed = await this.burnTxCommitment(
      chainId,
      input.input,
      outputNote,
      params.evmRecipient,
      encrypted.userKey
    )
    const ownerSignature = await this.signOwner(params.privacyAccount, signed)
    const outputCommitment = await noteCommitment(outputNote)
    const call = await this.proveCircuit(
      'burn',
      {
        input_note: noteInput(input.input.note),
        input_merkle_path: merklePathInput(input.input.merklePath),
        output_note: noteInput(outputNote),
        owner_signature: ownerSignatureInput(ownerSignature),
        burn_recipient_private: field(addressToBigInt(params.evmRecipient)),
        symmetric_key: field(encrypted.symmetricKey),
        ...this.commonPublicInputs(
          chainId,
          input.input.recentRoot,
          [input.input.nullifier, zeroHash()],
          [outputCommitment, zeroHash()],
          await outputNonceHash(input.input.note, outputNote),
          userEncryptedKeyHash(encrypted.userKey),
          0n,
          encrypted.encryptedNote,
          zeroWordArray5(),
          encrypted.chainKey,
          zeroWordArray3(),
          await this.chainPublicKeyInput()
        ),
        token: field(input.input.note.token),
        burn_recipient: field(addressToBigInt(params.evmRecipient)),
        value: field(params.amount),
        mint_from: field(0n),
        receive_prefix: field(0n)
      },
      chainId,
      signed,
      input.input.recentRoot,
      [input.input.commitment],
      [input.input.nullifier],
      [outputCommitment],
      encrypted.userKey,
      zeroWordArray4()
    )
    const request = bridgeTransactionRequest(
      this.client.inner.privacyBridge,
      this.client
        .bridge()
        .encodeBurnCall(
          call.verificationKeyHash,
          call.proof,
          call.publicInputs,
          call.userEncryptedKey
        )
    )
    return this.preparedResult(
      'burn',
      params.privacyAccount,
      params.token,
      request,
      call,
      undefined as TPayload
    )
  }

  private async prepareDirectSend(): Promise<
    PreparedOperationResult<TPayload>
  > {
    const params = this.params as DirectSendParams
    ensureAmountNonZero(params.amount)
    ensureU240(params.amount)
    const input = await this.resolveOwnedSpendInput(
      params.privacyAccount,
      params.token,
      false
    )
    if (input.kind !== 'real') {
      throw new PayyClientError('missing_owned_note', 'owned note required')
    }
    ensureBalance(input.input.note, params.amount)
    const outputNoteSelf: Note = {
      kind: 1n,
      token: input.input.note.token,
      nonce: input.input.note.nonce + 1n,
      psi: await randomNonZeroField(),
      owner: input.input.note.owner,
      value: input.input.note.value - params.amount
    }
    const note = await makeNote(
      params.token,
      params.amount,
      await privacyAddressOwner(params.recipient)
    )
    const call = await this.prepareTransferSendCall(
      params.privacyAccount,
      input.input,
      outputNoteSelf,
      note,
      params.recipient
    )
    const request = bridgeTransactionRequest(
      this.client.inner.privacyBridge,
      this.client
        .bridge()
        .encodeTransferCall(
          call.verificationKeyHash,
          call.proof,
          call.publicInputs,
          params.bridgeMemo ?? zeroHash(),
          call.userEncryptedKey,
          call.recipientEncryptedKey
        )
    )
    const payload = {
      recipient: params.recipient,
      note,
      commitment: await noteCommitment(note),
      sourceBridgeTxHash: this.client
        .bridge()
        .computeTxHash(call.verificationKeyHash, call.proof, call.publicInputs)
    } as TPayload
    return this.preparedResult(
      'transfer_send',
      params.privacyAccount,
      params.token,
      request,
      call,
      payload
    )
  }

  private async prepareEphemeralSend(): Promise<
    PreparedOperationResult<TPayload>
  > {
    const params = this.params as EphemeralSendParams
    ensureAmountNonZero(params.amount)
    ensureU240(params.amount)
    const input = await this.resolveOwnedSpendInput(
      params.privacyAccount,
      params.token,
      false
    )
    if (input.kind !== 'real') {
      throw new PayyClientError('missing_owned_note', 'owned note required')
    }
    ensureBalance(input.input.note, params.amount)
    const key = await this.signerForAccount(
      params.privacyAccount
    ).generateEphemeralKey()
    const outputNoteSelf: Note = {
      kind: 1n,
      token: input.input.note.token,
      nonce: input.input.note.nonce + 1n,
      psi: await randomNonZeroField(),
      owner: input.input.note.owner,
      value: input.input.note.value - params.amount
    }
    const note = await makeNote(
      params.token,
      params.amount,
      await ephemeralOwner(key.privateKey)
    )
    const call = await this.prepareTransferSendCall(
      params.privacyAccount,
      input.input,
      outputNoteSelf,
      note,
      key.privacyAddress
    )
    const request = bridgeTransactionRequest(
      this.client.inner.privacyBridge,
      this.client
        .bridge()
        .encodeTransferCall(
          call.verificationKeyHash,
          call.proof,
          call.publicInputs,
          params.bridgeMemo ?? zeroHash(),
          call.userEncryptedKey,
          call.recipientEncryptedKey
        )
    )
    const payload = {
      note,
      commitment: await noteCommitment(note),
      ephemeralPrivateKey: key.privateKey,
      sourceBridgeTxHash: this.client
        .bridge()
        .computeTxHash(call.verificationKeyHash, call.proof, call.publicInputs)
    } as TPayload
    return this.preparedResult(
      'transfer_send',
      params.privacyAccount,
      params.token,
      request,
      call,
      payload
    )
  }

  private async prepareClaim(): Promise<PreparedOperationResult<TPayload>> {
    const params = this.params as {
      readonly kind: 'note' | 'ephemeral' | 'link'
      readonly note?: IncomingNote
      readonly transfer?: IncomingTransfer
      readonly link?: ClaimLink
      readonly account?: PrivacyAccount
    }
    const payload = await this.claimPayload()
    let account: PrivacyAccount
    let incomingNote: Note
    let incomingSignature: OwnerSignature
    if (isIncomingNote(payload)) {
      account = await this.selectDirectClaimAccount(
        payload.note.owner,
        params.account
      )
      incomingNote = payload.note
    } else if (isIncomingTransfer(payload)) {
      if (params.account === undefined) {
        throw new PayyClientError(
          'missing_privacy_signer',
          'claim account required'
        )
      }
      account = params.account
      incomingNote = payload.note
    } else if (isParsedClaimLink(payload)) {
      if (payload.directNote !== undefined) {
        account = await this.selectDirectClaimAccount(
          payload.directNote.note.owner,
          params.account
        )
        incomingNote = payload.directNote.note
      } else if (payload.incomingTransfer !== undefined) {
        if (params.account === undefined) {
          throw new PayyClientError(
            'missing_privacy_signer',
            'claim account required'
          )
        }
        account = params.account
        incomingNote = payload.incomingTransfer.note
      } else {
        throw new PayyClientError('invalid_claim_link', 'invalid claim source')
      }
    } else {
      throw new PayyClientError('invalid_claim_link', 'invalid claim source')
    }
    const inputs = await this.claimInputSet(account, incomingNote)
    const ownNote = ownedInputNote(inputs.own)
    const outputNote: Note = {
      kind: 1n,
      token: incomingNote.token,
      nonce: ownNote.kind === 0n ? 0n : ownNote.nonce + 1n,
      psi: await randomNonZeroField(),
      owner: await privacyAddressOwner(privacyAccountAddress(account)),
      value: ownNote.value + incomingNote.value
    }
    ensureU240(outputNote.value)
    const encrypted = await this.encryptionForAccount(outputNote, account)
    const chainId = await this.client.inner.readClient.getChainId()
    const signed = await this.transferClaimTxCommitment(
      chainId,
      inputs.own,
      inputs.incoming,
      outputNote,
      encrypted.userKey
    )
    const recipientSignature = await this.signOwner(account, signed)
    if (isIncomingTransfer(payload)) {
      incomingSignature = await this.signEphemeralOwner(
        payload.ephemeralPrivateKey,
        signed
      )
    } else if (
      isParsedClaimLink(payload)
      && payload.incomingTransfer !== undefined
    ) {
      incomingSignature = await this.signEphemeralOwner(
        payload.incomingTransfer.ephemeralPrivateKey,
        signed
      )
    } else {
      incomingSignature = await this.signOwner(account, signed)
    }
    const outputCommitment = await noteCommitment(outputNote)
    const recentRoot = inputs.incoming.recentRoot
    if (
      inputs.own.kind === 'real'
      && inputs.own.input.recentRoot !== recentRoot
    ) {
      throw new PayyClientError('commitment_mismatch', 'root mismatch')
    }
    const call = await this.proveCircuit(
      'transfer_claim',
      {
        input_note_own: noteInput(ownNote),
        input_note_incoming: noteInput(inputs.incoming.note),
        input_merkle_path_own: merklePathInput(ownedMerklePath(inputs.own)),
        input_merkle_path_incoming: merklePathInput(inputs.incoming.merklePath),
        output_note: noteInput(outputNote),
        recipient_signature: ownerSignatureInput(recipientSignature),
        incoming_note_signature: ownerSignatureInput(incomingSignature),
        sender_symmetric_key: field(encrypted.symmetricKey),
        ...this.commonPublicInputs(
          chainId,
          recentRoot,
          [ownedNullifier(inputs.own), inputs.incoming.nullifier],
          [outputCommitment, zeroHash()],
          await outputNonceHash(ownNote, outputNote),
          userEncryptedKeyHash(encrypted.userKey),
          0n,
          encrypted.encryptedNote,
          zeroWordArray5(),
          encrypted.chainKey,
          zeroWordArray3(),
          await this.chainPublicKeyInput()
        ),
        token: field(0n),
        burn_recipient: field(0n),
        value: field(0n),
        mint_from: field(0n),
        receive_prefix: field(0n)
      },
      chainId,
      signed,
      recentRoot,
      [ownedCommitment(inputs.own), inputs.incoming.commitment],
      [ownedNullifier(inputs.own), inputs.incoming.nullifier],
      [outputCommitment],
      encrypted.userKey,
      zeroWordArray4()
    )
    const request = bridgeTransactionRequest(
      this.client.inner.privacyBridge,
      this.client
        .bridge()
        .encodeTransferCall(
          call.verificationKeyHash,
          call.proof,
          call.publicInputs,
          zeroHash(),
          call.userEncryptedKey,
          zeroWordArray4()
        )
    )
    return this.preparedResult(
      'transfer_claim',
      account,
      addressFromField(incomingNote.token),
      request,
      call,
      payload
    )
  }

  private async prepareTransferSendCall(
    account: PrivacyAccount,
    input: SpendInput,
    outputNoteSelf: Note,
    outputNoteRecv: Note,
    recipient: PrivacyAddress
  ): Promise<ProvenCall> {
    const senderSymmetricKey = await randomU240()
    const recipientSymmetricKey = await randomU240()
    const senderEncryptedNote = await encryptedNote(
      outputNoteSelf,
      senderSymmetricKey
    )
    const recipientEncryptedNote = await encryptedNote(
      outputNoteRecv,
      recipientSymmetricKey
    )
    const chainKey = await this.chainPublicKey()
    const senderChainKey = await encryptChainKey(senderSymmetricKey, chainKey)
    const recipientChainKey = await encryptChainKey(
      recipientSymmetricKey,
      chainKey
    )
    const senderKey = await encryptKeyForPublicKey(
      senderSymmetricKey,
      privacyAddressPublicKey(privacyAccountAddress(account))
    )
    const recipientKey = await encryptKeyForPublicKey(
      recipientSymmetricKey,
      privacyAddressPublicKey(recipient)
    )
    const chainId = await this.client.inner.readClient.getChainId()
    const signed = await this.transferSendTxCommitment(
      chainId,
      input,
      outputNoteSelf,
      outputNoteRecv,
      senderKey,
      recipientKey
    )
    const signature = await this.signOwner(account, signed)
    const outputCommitment0 = await noteCommitment(outputNoteSelf)
    const outputCommitment1 = await noteCommitment(outputNoteRecv)
    return this.proveCircuit(
      'transfer_send',
      {
        input_note: noteInput(input.note),
        input_merkle_path: merklePathInput(input.merklePath),
        output_note_self: noteInput(outputNoteSelf),
        output_note_recv: noteInput(outputNoteRecv),
        owner_signature: ownerSignatureInput(signature),
        sender_symmetric_key: field(senderSymmetricKey),
        recipient_symmetric_key: field(recipientSymmetricKey),
        ...this.commonPublicInputs(
          chainId,
          input.recentRoot,
          [input.nullifier, zeroHash()],
          [outputCommitment0, outputCommitment1],
          await outputNonceHash(input.note, outputNoteSelf),
          userEncryptedKeyHash(senderKey),
          encryptedKeyHash(recipientKey),
          senderEncryptedNote,
          recipientEncryptedNote,
          senderChainKey,
          recipientChainKey,
          await this.chainPublicKeyInput()
        ),
        token: field(0n),
        burn_recipient: field(0n),
        value: field(0n),
        mint_from: field(0n),
        receive_prefix: field(receivePrefix(outputNoteRecv.owner))
      },
      chainId,
      signed,
      input.recentRoot,
      [input.commitment],
      [input.nullifier],
      [outputCommitment0, outputCommitment1],
      senderKey,
      recipientKey
    )
  }

  private async resolveOwnedSpendInput(
    account: PrivacyAccount,
    token: Address,
    allowPadding: boolean
  ): Promise<OwnedSpendInput> {
    if (this.resolvedInputs[0] !== undefined) {
      return this.resolvedSpendInputFromOverride(
        this.resolvedInputs[0],
        allowPadding
      )
    }
    const state = await this.client.resolveOwnedNote(
      { privacyAccount: account, token },
      this.checkpoint
    )
    if (state.ownedNote === null) {
      if (allowPadding) {
        return {
          kind: 'padding',
          merklePath: zeroMerklePath(),
          recentRoot: await this.client.bridge().getRoot()
        }
      }
      throw new PayyClientError('missing_owned_note', 'owned note required')
    }
    return {
      kind: 'real',
      input: await this.spendInputFromNote(
        state.ownedNote.note,
        state.ownedNote.commitment,
        state.ownedNote.nullifier
      )
    }
  }

  private async claimInputSet(
    account: PrivacyAccount,
    incomingNote: Note
  ): Promise<ClaimInputs> {
    if (this.claimInputs !== undefined) {
      const own = await this.resolvedSpendInputFromOverride(
        this.claimInputs.ownedInput,
        true
      )
      const incoming = await this.spendInputFromNote(
        this.claimInputs.incomingInput.ownedNote.note,
        this.claimInputs.incomingInput.ownedNote.commitment,
        this.claimInputs.incomingInput.ownedNote.nullifier,
        this.claimInputs.incomingInput.merklePath,
        this.claimInputs.incomingInput.recentRoot
      )
      if (!sameNote(incoming.note, incomingNote)) {
        throw new PayyClientError(
          'commitment_mismatch',
          'incoming override mismatch'
        )
      }
      return { own, incoming }
    }
    return {
      own: await this.resolveOwnedSpendInput(
        account,
        addressFromField(incomingNote.token),
        true
      ),
      incoming: await this.spendInputFromNote(
        incomingNote,
        await noteCommitment(incomingNote),
        await noteNullifier(incomingNote)
      )
    }
  }

  private async resolvedSpendInputFromOverride(
    input: ResolvedInputNote,
    allowPadding: boolean
  ): Promise<OwnedSpendInput> {
    if (input.kind === 'padding') {
      if (!allowPadding) {
        throw new PayyClientError('missing_owned_note', 'owned note required')
      }
      return {
        kind: 'padding',
        merklePath: zeroMerklePath(),
        recentRoot: input.data.recentRoot
      }
    }
    return {
      kind: 'real',
      input: await this.spendInputFromNote(
        input.data.ownedNote.note,
        input.data.ownedNote.commitment,
        input.data.ownedNote.nullifier,
        input.data.merklePath,
        input.data.recentRoot
      )
    }
  }

  private async spendInputFromNote(
    note: Note,
    commitment: B256,
    nullifier: B256,
    merklePath?: readonly B256[],
    recentRoot?: B256
  ): Promise<SpendInput> {
    if ((await noteCommitment(note)) !== commitment) {
      throw new PayyClientError('commitment_mismatch', 'note mismatch')
    }
    if ((await noteNullifier(note)) !== nullifier) {
      throw new PayyClientError('nullifier_mismatch', 'nullifier mismatch')
    }
    if (await this.client.bridge().elementExists(nullifier)) {
      throw new PayyClientError('note_spent', 'note is spent')
    }
    const resolvedPath =
      merklePath === undefined
        ? await this.client.bridge().getMerklePath(commitment)
        : undefined
    const path = resolvedPath?.siblings ?? merklePath ?? []
    const root =
      resolvedPath?.root ?? recentRoot ?? (await this.client.bridge().getRoot())
    const computed = await computeMerkleRoot(
      b256ToBigInt(commitment),
      b256ToBigInt(commitment),
      path.map(b256ToBigInt)
    )
    if (computed !== b256ToBigInt(root)) {
      throw new PayyClientError('commitment_mismatch', 'Merkle root mismatch')
    }
    return {
      note,
      merklePath: path,
      recentRoot: root,
      commitment,
      nullifier
    }
  }

  private async encryptionForAccount(
    note: Note,
    account: PrivacyAccount
  ): Promise<{
    readonly symmetricKey: bigint
    readonly encryptedNote: readonly [B256, B256, B256, B256, B256]
    readonly chainKey: readonly [B256, B256, B256]
    readonly userKey: readonly [B256, B256, B256, B256]
  }> {
    const symmetricKey = await randomU240()
    const chainKey = await encryptChainKey(
      symmetricKey,
      await this.chainPublicKey()
    )
    return {
      symmetricKey,
      encryptedNote: await encryptedNote(note, symmetricKey),
      chainKey,
      userKey: await encryptKeyForPublicKey(
        symmetricKey,
        privacyAddressPublicKey(privacyAccountAddress(account))
      )
    }
  }

  private async chainPublicKey(): Promise<{
    readonly x: B256
    readonly y: B256
  }> {
    const [x, y] = await this.client.bridge().getChainPublicKey()
    return { x, y }
  }

  private async chainPublicKeyInput(): Promise<readonly string[]> {
    const key = await this.chainPublicKey()
    return [field(BigInt(key.x)), field(BigInt(key.y))]
  }

  private async signOwner(
    account: PrivacyAccount,
    commitment: bigint
  ): Promise<OwnerSignature> {
    return this.signerForAccount(account).signTxCommitment({
      privacyAccount: account,
      txCommitment: bigIntToB256(commitment)
    })
  }

  private async signEphemeralOwner(
    privateKey: B256,
    commitment: bigint
  ): Promise<OwnerSignature> {
    const signer = createLocalPrivacySignerFromGrumpkinPrivateKey(privateKey)
    const account = (await signer.accounts())[0]
    if (account === undefined) {
      throw new PayyClientError(
        'missing_privacy_signer',
        'ephemeral signer missing'
      )
    }
    return signer.signTxCommitment({
      privacyAccount: account,
      txCommitment: bigIntToB256(commitment)
    })
  }

  private signerForAccount(account: PrivacyAccount): PrivacySigner {
    if ('signer' in account) {
      return account.signer
    }
    return this.client.privacySignerRequired()
  }

  private async selectDirectClaimAccount(
    owner: bigint,
    account?: PrivacyAccount
  ): Promise<PrivacyAccount> {
    if (account !== undefined) {
      if (
        (await privacyAddressOwner(privacyAccountAddress(account))) !== owner
      ) {
        throw new PayyClientError(
          'privacy_account_mismatch',
          'claim account does not own incoming note'
        )
      }
      return account
    }
    for (const candidate of await this.client.accounts()) {
      if (
        (await privacyAddressOwner(privacyAccountAddress(candidate))) === owner
      ) {
        return candidate
      }
    }
    throw new PayyClientError(
      'privacy_account_mismatch',
      'no signer account owns incoming note'
    )
  }

  private commonPublicInputs(
    chainId: number,
    recentRoot: B256,
    inputNullifiers: readonly [B256, B256],
    outputCommitments: readonly [B256, B256],
    nonceHash: B256,
    userKeyHash: bigint,
    recipientKeyHash: bigint,
    senderEncryptedNote: readonly [B256, B256, B256, B256, B256],
    recipientEncryptedNote: readonly [B256, B256, B256, B256, B256],
    senderChainKey: readonly [B256, B256, B256],
    recipientChainKey: readonly [B256, B256, B256],
    chainPublicKey: readonly string[]
  ): NoirCircuitInput {
    return {
      chain_id: field(BigInt(chainId)),
      bridge_address: field(addressToBigInt(this.client.inner.privacyBridge)),
      recent_root: field(BigInt(recentRoot)),
      input_nullifiers: inputNullifiers.map((value) => field(BigInt(value))),
      output_commitments: outputCommitments.map((value) =>
        field(BigInt(value))
      ),
      nonce_hash: field(BigInt(nonceHash)),
      user_encrypted_key_hash: field(userKeyHash),
      recipient_encrypted_key_hash: field(recipientKeyHash),
      sender_encrypted_note: senderEncryptedNote.map((value) =>
        field(BigInt(value))
      ),
      recipient_encrypted_note: recipientEncryptedNote.map((value) =>
        field(BigInt(value))
      ),
      sender_chain_encrypted_key: senderChainKey.map((value) =>
        field(BigInt(value))
      ),
      recipient_chain_encrypted_key: recipientChainKey.map((value) =>
        field(BigInt(value))
      ),
      chain_public_key: [...chainPublicKey]
    }
  }

  private async proveCircuit(
    operation: PrivacyOperationKind,
    inputs: NoirCircuitInput,
    chainId: number,
    txCommitmentValue: bigint,
    recentRoot: B256,
    inputCommitments: readonly B256[],
    inputNullifiers: readonly B256[],
    outputCommitments: readonly B256[],
    userEncryptedKey: readonly [B256, B256, B256, B256],
    recipientEncryptedKey: readonly [B256, B256, B256, B256]
  ): Promise<ProvenCall> {
    const proof = await this.client.inner.provingBackend.prove(
      operation,
      inputs
    )
    return {
      chainId,
      verificationKeyHash: proof.verificationKeyHash,
      proof: proof.proof,
      publicInputs: proof.publicInputs,
      txCommitment: bigIntToB256(txCommitmentValue),
      recentRoot,
      inputCommitments: inputCommitments.filter(
        (value) => value !== zeroHash()
      ),
      inputNullifiers: inputNullifiers.filter((value) => value !== zeroHash()),
      outputCommitments: outputCommitments.filter(
        (value) => value !== zeroHash()
      ),
      userEncryptedKey,
      recipientEncryptedKey
    }
  }

  private preparedResult(
    operation: PrivacyOperationKind,
    account: PrivacyAccount,
    token: Address,
    bridgeRequest: BridgeTransactionRequest,
    call: ProvenCall,
    payload: TPayload
  ): PreparedOperationResult<TPayload> {
    return {
      preparedCall: {
        operation,
        chainId: call.chainId,
        bridgeRequest,
        verificationKeyHash: call.verificationKeyHash,
        proof: call.proof,
        publicInputs: call.publicInputs,
        txCommitment: call.txCommitment,
        statePreview: {
          privacyAccount: privacyAccountAddress(account),
          token,
          recentRoot: call.recentRoot,
          inputCommitments: call.inputCommitments,
          inputNullifiers: call.inputNullifiers,
          outputCommitments: call.outputCommitments
        }
      },
      payload
    }
  }

  private async mintTxCommitment(
    chainId: number,
    input: OwnedSpendInput,
    outputNote: Note,
    mintFrom: Address,
    userKey: readonly B256[]
  ): Promise<bigint> {
    return txCommitment(
      BigInt(chainId),
      addressToBigInt(this.client.inner.privacyBridge),
      b256ToBigInt(ownedCommitment(input)),
      0n,
      BigInt(await noteCommitment(outputNote)),
      0n,
      0n,
      addressToBigInt(mintFrom),
      userEncryptedKeyHash(userKey),
      0n,
      0n
    )
  }

  private async burnTxCommitment(
    chainId: number,
    input: SpendInput,
    outputNote: Note,
    burnRecipient: Address,
    userKey: readonly B256[]
  ): Promise<bigint> {
    return txCommitment(
      BigInt(chainId),
      addressToBigInt(this.client.inner.privacyBridge),
      b256ToBigInt(input.commitment),
      0n,
      BigInt(await noteCommitment(outputNote)),
      0n,
      addressToBigInt(burnRecipient),
      0n,
      userEncryptedKeyHash(userKey),
      0n,
      0n
    )
  }

  private async transferSendTxCommitment(
    chainId: number,
    input: SpendInput,
    outputNoteSelf: Note,
    outputNoteRecv: Note,
    userKey: readonly B256[],
    recipientKey: readonly B256[]
  ): Promise<bigint> {
    return txCommitment(
      BigInt(chainId),
      addressToBigInt(this.client.inner.privacyBridge),
      b256ToBigInt(input.commitment),
      0n,
      BigInt(await noteCommitment(outputNoteSelf)),
      BigInt(await noteCommitment(outputNoteRecv)),
      0n,
      0n,
      userEncryptedKeyHash(userKey),
      encryptedKeyHash(recipientKey),
      receivePrefix(outputNoteRecv.owner)
    )
  }

  private async transferClaimTxCommitment(
    chainId: number,
    own: OwnedSpendInput,
    incoming: SpendInput,
    outputNote: Note,
    userKey: readonly B256[]
  ): Promise<bigint> {
    return txCommitment(
      BigInt(chainId),
      addressToBigInt(this.client.inner.privacyBridge),
      b256ToBigInt(ownedCommitment(own)),
      b256ToBigInt(incoming.commitment),
      BigInt(await noteCommitment(outputNote)),
      0n,
      0n,
      0n,
      userEncryptedKeyHash(userKey),
      0n,
      0n
    )
  }

  private async claimPayload(): Promise<TPayload> {
    const params = this.params as {
      readonly kind: 'note' | 'ephemeral' | 'link'
      readonly note?: IncomingNote
      readonly transfer?: IncomingTransfer
      readonly link?: ClaimLink
      readonly account?: PrivacyAccount
    }
    if (params.kind === 'note' && params.note !== undefined) {
      await validateIncomingNote(params.note)
      await this.validateClaimPublication(params.note.note)
      await this.resolveDirectClaimAccount(
        params.note.note.owner,
        params.account
      )
      return params.note as TPayload
    }
    if (params.kind === 'ephemeral' && params.transfer !== undefined) {
      if (params.account === undefined) {
        throw new PayyClientError(
          'missing_privacy_signer',
          'claim account required'
        )
      }
      await validateIncomingTransfer(params.transfer, 'direct')
      await this.validateClaimPublication(params.transfer.note)
      return params.transfer as TPayload
    }
    if (params.link !== undefined) {
      const parsed = await this.client.links().parse(params.link.value)
      if (
        parsed.claimSourceKind === 'ephemeral'
        && params.account === undefined
      ) {
        throw new PayyClientError(
          'missing_privacy_signer',
          'claim account required'
        )
      }
      if (parsed.incomingTransfer !== undefined) {
        await validateIncomingTransfer(parsed.incomingTransfer, 'link')
        await this.validateClaimPublication(parsed.incomingTransfer.note)
      }
      if (parsed.directNote !== undefined) {
        await this.validateClaimPublication(parsed.directNote.note)
        await this.resolveDirectClaimAccount(
          parsed.directNote.note.owner,
          params.account
        )
      }
      return parsed as TPayload
    }
    throw new PayyClientError('invalid_claim_link', 'invalid claim source')
  }

  private async validateClaimPublication(note: Note): Promise<void> {
    const commitment = await noteCommitment(note)
    if (
      (await this.client.bridge().getTxnHashByCommitment(commitment)) === null
    ) {
      throw new PayyClientError(
        'commitment_not_found',
        'claim source is not published',
        { commitment }
      )
    }
    if (await this.client.bridge().elementExists(await noteNullifier(note))) {
      throw new PayyClientError('note_spent', 'claim source is spent')
    }
  }

  private async resolveDirectClaimAccount(
    owner: bigint,
    account?: PrivacyAccount
  ): Promise<void> {
    if (account !== undefined) {
      if (
        (await privacyAddressOwner(privacyAccountAddress(account))) !== owner
      ) {
        throw new PayyClientError(
          'privacy_account_mismatch',
          'claim account does not own incoming note'
        )
      }
      return
    }
    for (const candidate of await this.client.accounts()) {
      if (
        (await privacyAddressOwner(privacyAccountAddress(candidate))) === owner
      ) {
        return
      }
    }
    throw new PayyClientError(
      'privacy_account_mismatch',
      'no signer account owns incoming note'
    )
  }
}

export class SendOperationBuilder<
  TPayload extends DirectSendDelivery | IncomingTransfer
> extends OperationBuilder<TPayload> {
  withCheckpoint(checkpoint: OwnedNoteState): SendOperationBuilder<TPayload> {
    return new SendOperationBuilder(
      this.client,
      this.operation,
      this.params,
      checkpoint,
      this.resolvedInputs,
      this.claimInputs
    )
  }

  withOwnedInput(input: ResolvedInputNote): SendOperationBuilder<TPayload> {
    return new SendOperationBuilder(
      this.client,
      this.operation,
      this.params,
      this.checkpoint,
      [input],
      this.claimInputs
    )
  }

  withClaimInputs(inputs: ClaimResolvedInputs): SendOperationBuilder<TPayload> {
    return new SendOperationBuilder(
      this.client,
      this.operation,
      this.params,
      this.checkpoint,
      this.resolvedInputs,
      inputs
    )
  }

  async link(message?: string): Promise<Prepared<[TPayload, ClaimLink]>> {
    const prepared = await this.prepare()
    const payload = prepared.result.payload
    const link = isIncomingTransfer(payload)
      ? await encodeEphemeralClaimLink(payload, message)
      : isDirectSendDelivery(payload)
        ? await encodeDirectClaimLink(payload, message)
        : unsupportedLinkPayload()
    return new Prepared(this.client, {
      preparedCall: prepared.result.preparedCall,
      payload: [payload, link] as [TPayload, ClaimLink]
    })
  }
}

export class Prepared<TPayload> {
  readonly result: PreparedOperationResult<TPayload>
  private readonly client: PrivacyNamespace
  private readonly submitter?: PayyEvmSubmitter

  constructor(
    client: PrivacyNamespace,
    result: PreparedOperationResult<TPayload>,
    submitter?: PayyEvmSubmitter
  ) {
    this.client = client
    this.result = result
    this.submitter = submitter
  }

  async submit(): Promise<SubmittedOperationResult<TPayload>> {
    const submitter = this.submitter ?? this.client.inner.evmSubmitter
    if (submitter === undefined) {
      throw new PayyClientError(
        'missing_evm_submitter',
        'EVM submitter required'
      )
    }
    const chainId = await submitter.getChainId()
    if (chainId !== this.result.preparedCall.chainId) {
      throw new PayyClientError(
        'chain_id_mismatch',
        'submitter chain id mismatch',
        {
          expected: this.result.preparedCall.chainId,
          actual: chainId
        }
      )
    }
    const from = this.result.preparedCall.bridgeRequest.from
    if (from !== undefined) {
      const address = await submitter.getAddress()
      if (address === null || address.toLowerCase() !== from.toLowerCase()) {
        throw new PayyClientError(
          'evm_account_mismatch',
          'submitter account mismatch'
        )
      }
    }
    const sourceTxHash = await submitter.sendTransaction(
      this.result.preparedCall.bridgeRequest
    )
    const payload =
      this.result.preparedCall.operation === 'transfer_send'
        ? withSourceMetadata(this.result.payload, { sourceTxHash })
        : this.result.payload
    return {
      ...this.result,
      payload,
      sourceTxHash
    }
  }

  async submitAndWait(): Promise<ConfirmedOperationResult<TPayload>> {
    const submitted = await this.submit()
    const receipt =
      await this.client.inner.readClient.waitForTransactionReceipt({
        hash: submitted.sourceTxHash
      })
    if (receipt.status === 'reverted') {
      throw new PayyClientError('transaction_reverted', 'transaction reverted')
    }
    return {
      ...submitted,
      receipt
    }
  }
}

function field(value: bigint): string {
  return fieldMod(value).toString()
}

function noteInput(note: Note): NoirCircuitInput {
  return {
    kind: field(note.kind),
    token: field(note.token),
    nonce: field(note.nonce),
    psi: field(note.psi),
    owner: field(note.owner),
    value: field(note.value)
  }
}

function merklePathInput(path: readonly B256[]): NoirCircuitInput {
  if (path.length > 160) {
    throw new PayyClientError(
      'merkle_path_invalid',
      'Merkle path exceeds circuit depth'
    )
  }
  return {
    path: [...path, ...zeroMerklePath()]
      .slice(0, 160)
      .map((value) => field(BigInt(value)))
  }
}

function ownerSignatureInput(signature: OwnerSignature): NoirCircuitInput {
  return {
    signature: Array.from(hexToBytes(signature.signature)),
    public_key_x: field(BigInt(signature.publicKeyX)),
    public_key_y: field(BigInt(signature.publicKeyY))
  }
}

function zeroMerklePath(): readonly B256[] {
  return Array.from({ length: 160 }, zeroHash)
}

function zeroWordArray3(): readonly [B256, B256, B256] {
  return [zeroHash(), zeroHash(), zeroHash()]
}

function zeroWordArray4(): readonly [B256, B256, B256, B256] {
  return [zeroHash(), zeroHash(), zeroHash(), zeroHash()]
}

function zeroWordArray5(): readonly [B256, B256, B256, B256, B256] {
  return [zeroHash(), zeroHash(), zeroHash(), zeroHash(), zeroHash()]
}

function ownedInputNote(input: OwnedSpendInput): Note {
  return input.kind === 'real' ? input.input.note : paddingNote()
}

function ownedCommitment(input: OwnedSpendInput): B256 {
  return input.kind === 'real' ? input.input.commitment : zeroHash()
}

function ownedNullifier(input: OwnedSpendInput): B256 {
  return input.kind === 'real' ? input.input.nullifier : zeroHash()
}

function ownedRecentRoot(input: OwnedSpendInput): B256 {
  return input.kind === 'real' ? input.input.recentRoot : input.recentRoot
}

function ownedMerklePath(input: OwnedSpendInput): readonly B256[] {
  return input.kind === 'real' ? input.input.merklePath : input.merklePath
}

function paddingNote(): Note {
  return {
    kind: 0n,
    token: 0n,
    nonce: 0n,
    psi: 0n,
    owner: 0n,
    value: 0n
  }
}

function privacyAddressPublicKey(address: PrivacyAddress): {
  readonly x: B256
  readonly y: B256
} {
  const publicKey = publicKeyFromPrivacyAddress(address)
  return {
    x: bigIntToB256(publicKey.x),
    y: bigIntToB256(publicKey.y)
  }
}

function addressFromField(value: bigint): Address {
  return `0x${bigIntToB256(value).slice(-40)}` as Address
}

async function outputNonceHash(input: Note, output: Note): Promise<B256> {
  if (input.kind === 0n) {
    return firstNonceHash(output.kind, output.token, output.owner)
  }
  return nextNonceHash(
    output.kind,
    output.token,
    output.owner,
    output.nonce,
    input.psi
  )
}

function ensureU240(value: bigint): void {
  if (value < 0n || value >= MAX_NOTE_VALUE) {
    throw new PayyClientError('value_out_of_range', 'value exceeds 240 bits')
  }
}

function ensureFieldElement(value: bigint): void {
  if (value < 0n || value >= BN254_SCALAR_MODULUS) {
    throw new PayyClientError(
      'field_out_of_range',
      'field element out of range'
    )
  }
}

function ensureBalance(note: Note, amount: bigint): void {
  if (note.value < amount) {
    throw new PayyClientError('insufficient_balance', 'insufficient balance')
  }
}

function sameNote(lhs: Note, rhs: Note): boolean {
  return (
    lhs.kind === rhs.kind
    && lhs.token === rhs.token
    && lhs.nonce === rhs.nonce
    && lhs.psi === rhs.psi
    && lhs.owner === rhs.owner
    && lhs.value === rhs.value
  )
}

export function createPayyClient(config: PayyClientConfig): BasePayyClient {
  return new BasePayyClient(config)
}

class RawLocalEvmSubmitter implements PayyEvmSubmitter {
  private readonly evmPrivateKey: B256
  private readonly rawSubmitter: PayyRawTransactionSubmitter

  constructor(evmPrivateKey: B256, rawSubmitter: PayyRawTransactionSubmitter) {
    this.evmPrivateKey = evmPrivateKey
    this.rawSubmitter = rawSubmitter
  }

  async getChainId(): Promise<number> {
    return this.rawSubmitter.getChainId()
  }

  async getAddress(): Promise<Address> {
    return this.rawSubmitter.getLocalAddress(this.evmPrivateKey)
  }

  async sendTransaction(request: PayyEvmTransactionRequest): Promise<B256> {
    return this.rawSubmitter.sendLocalTransaction(this.evmPrivateKey, request)
  }
}

async function makeNote(
  token: Address,
  value: bigint,
  owner: bigint
): Promise<Note> {
  return {
    kind: 1n,
    token: addressToBigInt(token),
    nonce: 0n,
    psi: await randomNonZeroField(),
    owner,
    value
  }
}

async function validateCheckpoint(checkpoint: OwnedNoteState): Promise<void> {
  validateCheckpointMetadata(checkpoint)
  if (checkpoint.ownedNote === null) {
    return
  }
  if (
    (await noteCommitment(checkpoint.ownedNote.note))
    !== checkpoint.ownedNote.commitment
  ) {
    throw new PayyClientError('commitment_mismatch', 'cached note mismatch')
  }
  if (
    (await noteNullifier(checkpoint.ownedNote.note))
    !== checkpoint.ownedNote.nullifier
  ) {
    throw new PayyClientError('nullifier_mismatch', 'cached nullifier mismatch')
  }
  if (checkpoint.ownedNote.note.nonce === 0n) {
    const expectedNonceHash = await firstNonceHash(
      checkpoint.ownedNote.note.kind,
      checkpoint.ownedNote.note.token,
      checkpoint.ownedNote.note.owner
    )
    if (expectedNonceHash !== checkpoint.ownedNote.nonceHash) {
      throw new PayyClientError(
        'nonce_hash_mismatch',
        'cached first-note nonce hash mismatch'
      )
    }
  }
}

function validateCheckpointMetadata(checkpoint: OwnedNoteState): void {
  if (
    checkpoint.ownedNote?.sourceBlock !== undefined
    && checkpoint.ownedNote.sourceBlock > checkpoint.checkedBlock
  ) {
    throw new PayyClientError(
      'checkpoint_mismatch',
      'checkpoint source block exceeds checked block'
    )
  }
}

function isCheckpointShapeError(error: unknown): boolean {
  return (
    error instanceof PayyClientError
    && (error.code === 'commitment_mismatch'
      || error.code === 'nullifier_mismatch'
      || error.code === 'nonce_hash_mismatch')
  )
}

async function validateIncomingNote(note: IncomingNote): Promise<void> {
  if (note.status === 'spent') {
    throw new PayyClientError('note_spent', 'incoming note is spent')
  }
  validateReceivedNoteShape(note.note, 'direct')
  if ((await noteCommitment(note.note)) !== note.commitment) {
    throw new PayyClientError(
      'commitment_mismatch',
      'incoming note commitment mismatch'
    )
  }
  if ((await noteNullifier(note.note)) !== note.nullifier) {
    throw new PayyClientError(
      'nullifier_mismatch',
      'incoming note nullifier mismatch'
    )
  }
}

async function validateIncomingTransfer(
  transfer: IncomingTransfer,
  source: 'direct' | 'link'
): Promise<void> {
  validateReceivedNoteShape(transfer.note, source)
  if ((await noteCommitment(transfer.note)) !== transfer.commitment) {
    throw new PayyClientError(
      'commitment_mismatch',
      'incoming transfer mismatch'
    )
  }
  if (
    (await ephemeralOwner(transfer.ephemeralPrivateKey)) !== transfer.note.owner
  ) {
    throw new PayyClientError(
      source === 'link' ? 'invalid_claim_link' : 'ephemeral_key_mismatch',
      'ephemeral owner mismatch'
    )
  }
}

function validateReceivedNoteShape(
  note: Note,
  source: 'direct' | 'link'
): void {
  const shapeCode =
    source === 'link' ? 'invalid_claim_link' : 'invalid_incoming_transfer'
  if (note.kind !== 1n) {
    throw new PayyClientError(shapeCode, 'invalid incoming note kind')
  }
  ensureFieldElement(note.token)
  ensureFieldElement(note.nonce)
  ensureFieldElement(note.psi)
  ensureFieldElement(note.owner)
  if (note.token >= ADDRESS_FIELD_BOUND || note.nonce !== 0n) {
    throw new PayyClientError(shapeCode, 'invalid incoming transfer note')
  }
  ensureU240(note.value)
  ensureAmountNonZero(note.value)
}

function cacheKey(privacyAccountValue: PrivacyAddress, token: Address): string {
  return `${privacyAddress(privacyAccountValue.bytes).bytes}:${canonicalAddress(token)}`
}

function normalizeOwnedNoteState(checkpoint: OwnedNoteState): OwnedNoteState {
  const normalizedPrivacyAccount = privacyAddress(
    checkpoint.privacyAccount.bytes
  )
  const normalizedToken = canonicalAddress(checkpoint.token)
  if (
    normalizedPrivacyAccount.bytes === checkpoint.privacyAccount.bytes
    && normalizedToken === checkpoint.token
  ) {
    return checkpoint
  }
  return {
    ...checkpoint,
    privacyAccount: normalizedPrivacyAccount,
    token: normalizedToken
  }
}

function normalizePrivacyAccount(account: PrivacyAccount): PrivacyAccount {
  if ('privacyAddress' in account) {
    return {
      ...account,
      privacyAddress: privacyAddress(account.privacyAddress.bytes)
    }
  }
  return privacyAddress(account.bytes)
}

function resolvePrivacyAccount(
  client: PrivacyNamespace,
  account: PrivacyAccount
): ResolvedPrivacyAccount {
  const normalized = normalizePrivacyAccount(account)
  return {
    privacyAddress: privacyAccountAddress(normalized),
    privacyAccount: normalized,
    signer: signerForPrivacyAccount(client, normalized)
  }
}

function signerForPrivacyAccount(
  client: PrivacyNamespace,
  account: PrivacyAccount
): PrivacySigner {
  if ('signer' in account) {
    return account.signer
  }
  return client.privacySignerRequired()
}

function prefixToTopic(prefix: PrivacyAddressPrefix): B256 {
  return `0x${prefix.bytes.slice(2).padEnd(64, '0')}` as B256
}

function compareIncomingNotes(a: IncomingNote, b: IncomingNote): number {
  if (a.sourcePosition.blockNumber !== b.sourcePosition.blockNumber) {
    return a.sourcePosition.blockNumber < b.sourcePosition.blockNumber ? -1 : 1
  }
  if (a.sourcePosition.transactionIndex !== b.sourcePosition.transactionIndex) {
    return a.sourcePosition.transactionIndex - b.sourcePosition.transactionIndex
  }
  return a.sourcePosition.logIndex - b.sourcePosition.logIndex
}

async function sleep(ms: number): Promise<void> {
  await new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function isIncomingTransfer(value: unknown): value is IncomingTransfer {
  return (
    typeof value === 'object'
    && value !== null
    && 'ephemeralPrivateKey' in value
  )
}

function isDirectSendDelivery(value: unknown): value is DirectSendDelivery {
  return typeof value === 'object' && value !== null && 'recipient' in value
}

function withSourceMetadata<TPayload>(
  payload: TPayload,
  metadata: {
    readonly sourceTxHash?: B256
    readonly sourceBridgeTxHash?: B256
  }
): TPayload {
  if (isIncomingTransfer(payload)) {
    return { ...payload, ...metadata } as TPayload
  }
  if (isDirectSendDelivery(payload)) {
    return { ...payload, ...metadata } as TPayload
  }
  if (isLinkedSendPayload(payload)) {
    return [withSourceMetadata(payload[0], metadata), payload[1]] as TPayload
  }
  return payload
}

function isLinkedSendPayload(
  value: unknown
): value is [DirectSendDelivery | IncomingTransfer, ClaimLink] {
  return (
    Array.isArray(value)
    && value.length === 2
    && (isIncomingTransfer(value[0]) || isDirectSendDelivery(value[0]))
  )
}

function unsupportedLinkPayload(): never {
  throw new PayyClientError(
    'invalid_claim_link',
    'link generation is only supported for send operations'
  )
}

function isIncomingNote(value: unknown): value is IncomingNote {
  return typeof value === 'object' && value !== null && 'status' in value
}

function isParsedClaimLink(value: unknown): value is ParsedClaimLink {
  return (
    typeof value === 'object' && value !== null && 'claimSourceKind' in value
  )
}

function bridgeTransactionRequest(
  privacyBridge: Address,
  data: Hex,
  from?: Address
): BridgeTransactionRequest {
  return {
    from,
    to: privacyBridge,
    data,
    value: 0n
  }
}
