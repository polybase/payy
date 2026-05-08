export type Hex = `0x${string}`
export type Address = Hex
export type B256 = Hex

export type PayyNetworkConfig = {
  readonly chainId: number
  readonly name: string
  readonly privacyBridge: Address
}

export type ReadContractArgs = {
  readonly to: Address
  readonly data: Hex
  readonly blockNumber?: bigint
}

export type PayyEvmCallRequest = ReadContractArgs

export type PayyEvmLogFilter = {
  readonly address: Address
  readonly fromBlock: bigint
  readonly toBlock: bigint
  readonly topics: readonly (B256 | null)[]
}

export type PayyEvmLog = {
  readonly address: Address
  readonly blockNumber: bigint
  readonly transactionIndex: number
  readonly logIndex: number
  readonly transactionHash: B256
  readonly topics: readonly B256[]
  readonly data: Hex
}

export type PayyEvmTransactionRequest = {
  readonly from?: Address
  readonly to: Address
  readonly data: Hex
  readonly value?: bigint
  readonly gasLimit?: bigint
}

export type BridgeTransactionRequest = {
  readonly from?: Address
  readonly to: Address
  readonly data: Hex
  readonly value: bigint
  readonly gasLimit?: bigint
}

export type PayyTransactionStatus = 'success' | 'reverted'

export type PayyTransactionReceipt = {
  readonly transactionHash: B256
  readonly blockNumber: bigint
  readonly status: PayyTransactionStatus
}

export type PayyWaitForTransactionReceiptArgs = {
  readonly hash: B256
  readonly confirmations?: number
  readonly timeoutMs?: number
  readonly pollIntervalMs?: number
}

export type PayyEvmReadClient = {
  getChainId(): Promise<number>
  getBlockNumber(): Promise<bigint>
  readContract(args: ReadContractArgs): Promise<Hex>
  getLogs(filter: PayyEvmLogFilter): Promise<readonly PayyEvmLog[]>
  getTransactionReceipt(hash: B256): Promise<PayyTransactionReceipt | null>
  waitForTransactionReceipt(
    args: PayyWaitForTransactionReceiptArgs
  ): Promise<PayyTransactionReceipt>
}

export type PayyEvmSubmitter = {
  getChainId(): Promise<number>
  getAddress(): Promise<Address | null>
  sendTransaction(request: PayyEvmTransactionRequest): Promise<B256>
}

export type PayyRawFeeData = {
  readonly maxFeePerGas: bigint
  readonly maxPriorityFeePerGas: bigint
}

export type PayyEvmFeeData = PayyRawFeeData

export type PayyBlockTag = 'latest' | 'pending'

export type PayyTransactionCountArgs = {
  readonly address: Address
  readonly blockTag?: PayyBlockTag
}

export type PayyRawTransactionSubmitter = {
  getChainId(): Promise<number>
  getTransactionCount(args: PayyTransactionCountArgs): Promise<bigint>
  estimateGas(request: PayyEvmTransactionRequest): Promise<bigint>
  getFeeData(): Promise<PayyRawFeeData>
  sendRawTransaction(rawTransaction: Hex): Promise<B256>
  getLocalAddress(evmPrivateKey: B256): Promise<Address>
  sendLocalTransaction(
    evmPrivateKey: B256,
    request: PayyEvmTransactionRequest
  ): Promise<B256>
}

export type PrivacyAddress = {
  readonly bytes: B256
}

export type PrivacyAddressPrefix = {
  readonly bytes: Hex
}

export type OwnerSignature = {
  readonly publicKeyX: B256
  readonly publicKeyY: B256
  readonly signature: Hex
}

export type TxnData = {
  readonly verificationKeyHash: B256
  readonly senderEncryptedNote: readonly [B256, B256, B256, B256, B256]
  readonly recipientEncryptedNote: readonly [B256, B256, B256, B256, B256]
  readonly senderChainEncryptedKey: readonly [B256, B256, B256]
  readonly recipientChainEncryptedKey: readonly [B256, B256, B256]
  readonly userEncryptedKey: readonly [B256, B256, B256, B256]
  readonly recipientEncryptedKey: readonly [B256, B256, B256, B256]
  readonly memo: B256
}

export type PayyMerklePath = {
  readonly root: B256
  readonly siblings: readonly B256[]
}

export type Note = {
  readonly kind: bigint
  readonly token: bigint
  readonly nonce: bigint
  readonly psi: bigint
  readonly owner: bigint
  readonly value: bigint
}

export type EphemeralKeyPair = {
  readonly privateKey: B256
  readonly privacyAddress: PrivacyAddress
}

export type PrivacySigner = {
  accounts(): Promise<readonly PrivacyAccount[]>
  signTxCommitment(args: {
    readonly privacyAccount: PrivacyAccount
    readonly txCommitment: B256
  }): Promise<OwnerSignature>
  decryptSenderNote(args: {
    readonly privacyAccount: PrivacyAccount
    readonly txnData: TxnData
  }): Promise<Note | null>
  decryptRecipientNote(args: {
    readonly privacyAccount: PrivacyAccount
    readonly txnData: TxnData
  }): Promise<Note | null>
  generateEphemeralKey(): Promise<EphemeralKeyPair>
}

export type PrivacySignerAccount = {
  readonly privacyAddress: PrivacyAddress
  readonly signer: PrivacySigner
}

export type PrivacyAccount = PrivacyAddress | PrivacySignerAccount

export type EvmSignerAccount = {
  readonly address: Address
  readonly submitter: PayyEvmSubmitter
}

export type EvmAccount = Address | EvmSignerAccount

export type OwnedNote = {
  readonly note: Note
  readonly commitment: B256
  readonly nullifier: B256
  readonly nonceHash: B256
  readonly sourceBlock?: bigint
  readonly sourceTxHash?: B256
  readonly sourceBridgeTxHash?: B256
}

export type OwnedNoteState = {
  readonly privacyAccount: PrivacyAddress
  readonly token: Address
  readonly ownedNote: OwnedNote | null
  readonly checkedBlock: bigint
}

export type PrivateBalance = {
  readonly privacyAccount: PrivacyAddress
  readonly token: Address
  readonly spendable: bigint
}

export type PrivateBalanceState = {
  readonly balance: PrivateBalance | null
  readonly ownedNoteState: OwnedNoteState
}

export type SourceChainPosition = {
  readonly blockNumber: bigint
  readonly transactionIndex: number
  readonly logIndex: number
}

export type IncomingNoteStatus = 'claimable' | 'spent'

export type IncomingNote = {
  readonly note: Note
  readonly commitment: B256
  readonly nullifier: B256
  readonly sourcePosition: SourceChainPosition
  readonly sourceTxHash: B256
  readonly sourceBridgeTxHash: B256
  readonly status: IncomingNoteStatus
}

export type IncomingTransfer = {
  readonly note: Note
  readonly commitment: B256
  readonly ephemeralPrivateKey: B256
  readonly sourceTxHash?: B256
  readonly sourceBridgeTxHash?: B256
}

export type DirectSendDelivery = {
  readonly recipient: PrivacyAddress
  readonly note: Note
  readonly commitment: B256
  readonly sourceTxHash?: B256
  readonly sourceBridgeTxHash?: B256
}

export type DirectLinkedNote = {
  readonly note: Note
  readonly commitment: B256
}

export type RealInputNote = {
  readonly ownedNote: OwnedNote
  readonly merklePath: readonly B256[]
  readonly recentRoot: B256
}

export type PaddingInputNote = {
  readonly recentRoot: B256
}

export type ResolvedInputNote =
  | {
      readonly kind: 'real'
      readonly data: RealInputNote
    }
  | {
      readonly kind: 'padding'
      readonly data: PaddingInputNote
    }

export type ClaimResolvedInputs = {
  readonly ownedInput: ResolvedInputNote
  readonly incomingInput: RealInputNote
}

export type PrivacyOperationKind =
  | 'mint'
  | 'burn'
  | 'transfer_send'
  | 'transfer_claim'

export type PrivacyStatePreview = {
  readonly privacyAccount: PrivacyAddress
  readonly token: Address
  readonly recentRoot: B256
  readonly inputCommitments: readonly B256[]
  readonly inputNullifiers: readonly B256[]
  readonly outputCommitments: readonly B256[]
}

export type PreparedPrivacyCall = {
  readonly operation: PrivacyOperationKind
  readonly chainId: number
  readonly bridgeRequest: BridgeTransactionRequest
  readonly verificationKeyHash: B256
  readonly proof: Hex
  readonly publicInputs: readonly B256[]
  readonly txCommitment: B256
  readonly statePreview: PrivacyStatePreview
}

export type PreparedOperationResult<TPayload> = {
  readonly preparedCall: PreparedPrivacyCall
  readonly payload: TPayload
}

export type SubmittedOperationResult<TPayload> =
  PreparedOperationResult<TPayload> & {
    readonly sourceTxHash: B256
  }

export type ConfirmedOperationResult<TPayload> =
  SubmittedOperationResult<TPayload> & {
    readonly receipt: PayyTransactionReceipt
  }

export type ClaimLink = {
  readonly value: string
}

export type ClaimSourceKind = 'direct' | 'ephemeral'

export type ParsedClaimLink = {
  readonly claimSourceKind: ClaimSourceKind
  readonly message: string | null
  readonly directNote?: DirectLinkedNote
  readonly incomingTransfer?: IncomingTransfer
}
