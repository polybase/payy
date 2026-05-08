import type {
  Address,
  B256,
  Hex,
  PayyEvmLog,
  PayyEvmLogFilter,
  PayyEvmReadClient,
  PayyEvmSubmitter,
  PayyEvmTransactionRequest,
  PayyBlockTag,
  PayyRawFeeData,
  PayyRawTransactionSubmitter,
  PayyTransactionReceipt,
  PreparedOperationResult,
  PreparedPrivacyCall
} from './types'
import { PayyClientError } from './errors'
import { Wallet } from 'ethers'

const DEFAULT_RECEIPT_TIMEOUT_MS = 60_000
const DEFAULT_RECEIPT_POLL_INTERVAL_MS = 1_000

type EthersProvider = {
  getNetwork(): Promise<{ readonly chainId: bigint }>
  getBlockNumber(): Promise<number>
  call(
    args: { readonly to: Address; readonly data: Hex },
    blockTag?: bigint
  ): Promise<Hex>
  getLogs(args: {
    readonly address: Address
    readonly fromBlock: bigint
    readonly toBlock: bigint
    readonly topics: readonly (B256 | null)[]
  }): Promise<
    readonly {
      readonly blockNumber?: number
      readonly transactionIndex?: number
      readonly index?: number
      readonly logIndex?: number
      readonly transactionHash?: B256
      readonly address?: Address
      readonly topics?: readonly B256[]
      readonly data?: Hex
    }[]
  >
  getTransactionReceipt(hash: B256): Promise<{
    readonly hash: B256
    readonly blockNumber: number
    readonly status: number | null
  } | null>
  waitForTransaction(hash: B256): Promise<{
    readonly hash: B256
    readonly blockNumber: number
    readonly status: number | null
  } | null>
  getTransactionCount(
    address: Address,
    blockTag?: PayyBlockTag
  ): Promise<number>
  estimateGas(request: PayyEvmTransactionRequest): Promise<bigint>
  getFeeData(): Promise<EthersFeeData>
  broadcastTransaction(rawTransaction: Hex): Promise<{ readonly hash: B256 }>
}

type EthersFeeData = {
  readonly maxFeePerGas: bigint | null
  readonly maxPriorityFeePerGas: bigint | null
}

type EthersSigner = {
  provider?: EthersProvider
  getAddress(): Promise<Address>
  sendTransaction(
    request: PayyEvmTransactionRequest
  ): Promise<{ readonly hash: B256 }>
}

type EthersTransactionFrom = Address | { readonly address: Address }

type PreparedTransactionInput =
  | PreparedPrivacyCall
  | PreparedOperationResult<unknown>
  | { readonly result: PreparedOperationResult<unknown> }

export type EthersTransactionOptions = {
  readonly chainId?: number
  readonly from?: EthersTransactionFrom
}

export type EthersTransaction = {
  readonly chainId: number
  readonly from?: Address
  readonly to: Address
  readonly data: Hex
  readonly value: bigint
  readonly gasLimit?: bigint
}

export function ethersProviderAdapter(
  provider: EthersProvider
): PayyEvmReadClient {
  return {
    getChainId: async () => Number((await provider.getNetwork()).chainId),
    getBlockNumber: async () => BigInt(await provider.getBlockNumber()),
    readContract: (args) =>
      provider.call({ to: args.to, data: args.data }, args.blockNumber),
    getLogs: async (filter) =>
      (await provider.getLogs(normalizeLogFilter(filter))).map(normalizeLog),
    getTransactionReceipt: async (hash) => {
      const receipt = await provider.getTransactionReceipt(hash)
      return receipt === null ? null : normalizeReceipt(receipt)
    },
    waitForTransactionReceipt: async (args) => {
      const timeoutMs = args.timeoutMs ?? DEFAULT_RECEIPT_TIMEOUT_MS
      const pollIntervalMs =
        args.pollIntervalMs ?? DEFAULT_RECEIPT_POLL_INTERVAL_MS
      const deadline = Date.now() + timeoutMs
      for (;;) {
        const receipt = await provider.getTransactionReceipt(args.hash)
        if (receipt !== null) {
          const latestBlock =
            confirmationsRequired(args.confirmations) <= 1
              ? BigInt(receipt.blockNumber)
              : BigInt(await provider.getBlockNumber())
          if (
            receiptConfirmed(
              BigInt(receipt.blockNumber),
              latestBlock,
              args.confirmations
            )
          ) {
            return normalizeReceipt(receipt)
          }
        }
        if (Date.now() >= deadline) {
          throw new PayyClientError(
            'receipt_timeout',
            'ethers waitForTransactionReceipt timeout',
            { hash: args.hash, timeoutMs }
          )
        }
        await sleep(pollIntervalMs)
      }
    }
  }
}

export function ethersSignerSubmitter(signer: EthersSigner): PayyEvmSubmitter {
  return {
    getChainId: async () => {
      if (signer.provider === undefined) {
        throw new PayyClientError(
          'missing_evm_provider',
          'ethers signer provider required'
        )
      }
      return Number((await signer.provider.getNetwork()).chainId)
    },
    getAddress: () => signer.getAddress(),
    sendTransaction: async (request) => {
      const signerAddress = await signer.getAddress()
      if (
        request.from !== undefined
        && request.from.toLowerCase() !== signerAddress.toLowerCase()
      ) {
        throw new PayyClientError(
          'evm_account_mismatch',
          'ethers signer does not match request.from'
        )
      }
      return (await signer.sendTransaction(request)).hash
    }
  }
}

export function toEthersTransaction(
  prepared: PreparedTransactionInput,
  options: EthersTransactionOptions = {}
): EthersTransaction {
  const call = preparedCall(prepared)
  if (options.chainId !== undefined && options.chainId !== call.chainId) {
    throw new PayyClientError(
      'chain_id_mismatch',
      'ethers transaction chain id mismatch',
      {
        expected: call.chainId,
        actual: options.chainId
      }
    )
  }
  const request = call.bridgeRequest
  const from = options.from ?? request.from
  if (
    request.from !== undefined
    && from !== undefined
    && fromAddress(from).toLowerCase() !== request.from.toLowerCase()
  ) {
    throw new PayyClientError(
      'evm_account_mismatch',
      'ethers transaction from does not match request.from'
    )
  }
  return {
    chainId: call.chainId,
    ...(from === undefined ? {} : { from: fromAddress(from) }),
    to: request.to,
    data: request.data,
    value: request.value ?? 0n,
    ...(request.gasLimit === undefined ? {} : { gasLimit: request.gasLimit })
  }
}

export function ethersRawTransactionSubmitter(
  provider: EthersProvider
): PayyRawTransactionSubmitter {
  return {
    getChainId: async () => Number((await provider.getNetwork()).chainId),
    getTransactionCount: async (args) =>
      BigInt(await provider.getTransactionCount(args.address, args.blockTag)),
    estimateGas: (request) => provider.estimateGas(request),
    getFeeData: async () => normalizeFeeData(await provider.getFeeData()),
    sendRawTransaction: async (rawTransaction) =>
      (await provider.broadcastTransaction(rawTransaction)).hash,
    getLocalAddress: async (evmPrivateKey) =>
      new Wallet(evmPrivateKey).address as Address,
    sendLocalTransaction: async (evmPrivateKey, request) => {
      const wallet = new Wallet(evmPrivateKey)
      const address = (await wallet.getAddress()) as Address
      if (
        request.from !== undefined
        && request.from.toLowerCase() !== address.toLowerCase()
      ) {
        throw new PayyClientError(
          'evm_account_mismatch',
          'ethers local account does not match request.from'
        )
      }
      const fee = normalizeFeeData(await provider.getFeeData())
      const gasRequest = {
        ...request,
        from: request.from ?? address
      }
      const rawTransaction = await wallet.signTransaction({
        type: 2,
        chainId: Number((await provider.getNetwork()).chainId),
        to: request.to,
        data: request.data,
        value: request.value ?? 0n,
        gasLimit: request.gasLimit ?? (await provider.estimateGas(gasRequest)),
        nonce: await provider.getTransactionCount(address, 'pending'),
        maxFeePerGas: fee.maxFeePerGas,
        maxPriorityFeePerGas: fee.maxPriorityFeePerGas
      })
      return (await provider.broadcastTransaction(rawTransaction as Hex)).hash
    }
  }
}

function normalizeReceipt(receipt: {
  readonly hash: B256
  readonly blockNumber: number
  readonly status: number | null
}): PayyTransactionReceipt {
  if (receipt.status !== 0 && receipt.status !== 1) {
    throw new PayyClientError(
      'receipt_status_unknown',
      'ethers receipt status unavailable',
      { hash: receipt.hash }
    )
  }
  return {
    transactionHash: receipt.hash,
    blockNumber: BigInt(receipt.blockNumber),
    status: receipt.status === 0 ? 'reverted' : 'success'
  }
}

function normalizeFeeData(fee: EthersFeeData): PayyRawFeeData {
  if (fee.maxFeePerGas === null) {
    throw new PayyClientError(
      'fee_data_unavailable',
      'ethers maxFeePerGas unavailable',
      { field: 'maxFeePerGas' }
    )
  }
  if (fee.maxPriorityFeePerGas === null) {
    throw new PayyClientError(
      'fee_data_unavailable',
      'ethers maxPriorityFeePerGas unavailable',
      { field: 'maxPriorityFeePerGas' }
    )
  }
  return {
    maxFeePerGas: fee.maxFeePerGas,
    maxPriorityFeePerGas: fee.maxPriorityFeePerGas
  }
}

function receiptConfirmed(
  receiptBlock: bigint,
  latestBlock: bigint,
  confirmations?: number
): boolean {
  const required = BigInt(confirmationsRequired(confirmations))
  return latestBlock >= receiptBlock + required - 1n
}

function confirmationsRequired(confirmations?: number): number {
  return Math.max(confirmations ?? 1, 1)
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function normalizeLogFilter(filter: PayyEvmLogFilter): {
  readonly address: Address
  readonly fromBlock: bigint
  readonly toBlock: bigint
  readonly topics: readonly (B256 | null)[]
} {
  return filter
}

function normalizeLog(log: {
  readonly blockNumber?: number
  readonly transactionIndex?: number
  readonly index?: number
  readonly logIndex?: number
  readonly transactionHash?: B256
  readonly address?: Address
  readonly topics?: readonly B256[]
  readonly data?: Hex
}): PayyEvmLog {
  return {
    address: requireLogField(log.address, 'address'),
    blockNumber: BigInt(requireLogField(log.blockNumber, 'blockNumber')),
    transactionIndex: requireLogField(log.transactionIndex, 'transactionIndex'),
    logIndex: requireLogField(log.logIndex ?? log.index, 'logIndex'),
    transactionHash: requireLogField(log.transactionHash, 'transactionHash'),
    topics: requireLogField(log.topics, 'topics'),
    data: requireLogField(log.data, 'data')
  }
}

function requireLogField<T>(value: T | null | undefined, field: string): T {
  if (value === null || value === undefined) {
    throw new PayyClientError(
      'missing_log_metadata',
      'ethers log missing metadata',
      { field }
    )
  }
  return value
}

function preparedCall(prepared: PreparedTransactionInput): PreparedPrivacyCall {
  if ('result' in prepared) {
    return prepared.result.preparedCall
  }
  if ('preparedCall' in prepared) {
    return prepared.preparedCall
  }
  return prepared
}

function fromAddress(from: EthersTransactionFrom): Address {
  if (typeof from === 'string') {
    return from
  }
  return from.address
}
