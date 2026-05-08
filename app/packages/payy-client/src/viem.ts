import type {
  Address,
  B256,
  Hex,
  PayyEvmLog,
  PayyEvmLogFilter,
  PayyEvmReadClient,
  PayyEvmSubmitter,
  PayyEvmTransactionRequest,
  PayyTransactionCountArgs,
  PayyRawFeeData,
  PayyRawTransactionSubmitter,
  PayyTransactionReceipt,
  PreparedOperationResult,
  PreparedPrivacyCall
} from './types'
import { PayyClientError } from './errors'
import { payyNetworks } from './network'
import { defineChain, type Account, type Chain } from 'viem'
import { privateKeyToAccount } from 'viem/accounts'

const PAYY_DEV_RPC_URL = 'http://127.0.0.1:8546'
const PAYY_TESTNET_RPC_URL = 'https://rpc.testnet.payy.network'
const PAYY_TESTNET_EXPLORER_URL = 'https://blockscout.testnet.payy.network'

export const chains = {
  payy: {
    dev: defineChain({
      id: payyNetworks.dev.chainId,
      name: 'Payy Dev',
      nativeCurrency: {
        name: 'PUSD',
        symbol: 'PUSD',
        decimals: 16
      },
      rpcUrls: {
        default: { http: [PAYY_DEV_RPC_URL] }
      }
    }),
    testnet: defineChain({
      id: payyNetworks.testnet.chainId,
      name: 'Payy Testnet',
      nativeCurrency: {
        name: 'PUSD',
        symbol: 'PUSD',
        decimals: 16
      },
      rpcUrls: {
        default: { http: [PAYY_TESTNET_RPC_URL] }
      },
      blockExplorers: {
        default: {
          name: 'Payy Blockscout',
          url: PAYY_TESTNET_EXPLORER_URL
        }
      }
    })
  }
} as const

type ViemPublicClient = {
  getChainId(): Promise<number>
  getBlockNumber(): Promise<bigint>
  call(args: {
    readonly to: Address
    readonly data: Hex
    readonly blockNumber?: bigint
  }): Promise<{ readonly data?: Hex }>
  getLogs(args: {
    readonly address: Address
    readonly fromBlock: bigint
    readonly toBlock: bigint
    readonly topics: readonly (B256 | null)[]
  }): Promise<
    readonly {
      readonly blockNumber?: bigint
      readonly transactionIndex?: number
      readonly logIndex?: number
      readonly transactionHash?: B256
      readonly address?: Address
      readonly topics?: readonly B256[]
      readonly data?: Hex
    }[]
  >
  getTransactionReceipt(args: { readonly hash: B256 }): Promise<{
    readonly transactionHash: B256
    readonly blockNumber: bigint
    readonly status: 'success' | 'reverted'
  } | null>
  waitForTransactionReceipt(args: {
    readonly hash: B256
    readonly confirmations?: number
    readonly timeout?: number
    readonly pollingInterval?: number
  }): Promise<{
    readonly transactionHash: B256
    readonly blockNumber: bigint
    readonly status: 'success' | 'reverted'
  }>
  getTransactionCount(args: PayyTransactionCountArgs): Promise<number>
  estimateGas(request: PayyEvmTransactionRequest): Promise<bigint>
  estimateFeesPerGas(): Promise<PayyRawFeeData>
  sendRawTransaction(args: {
    readonly serializedTransaction: Hex
  }): Promise<B256>
}

type ViemWalletClient = {
  getChainId(): Promise<number>
  account?: { readonly address: Address }
  sendTransaction(request: PayyEvmTransactionRequest): Promise<B256>
}

type ViemTransactionAccount = Address | Account

type PreparedTransactionInput =
  | PreparedPrivacyCall
  | PreparedOperationResult<unknown>
  | { readonly result: PreparedOperationResult<unknown> }

export type ViemTransactionOptions = {
  readonly chain?: Chain
  readonly account?: ViemTransactionAccount
}

export type ViemTransaction = {
  readonly account?: ViemTransactionAccount
  readonly chain?: Chain
  readonly to: Address
  readonly data: Hex
  readonly value: bigint
  readonly gas?: bigint
}

export function viemPublicClientAdapter(
  publicClient: ViemPublicClient
): PayyEvmReadClient {
  return {
    getChainId: () => publicClient.getChainId(),
    getBlockNumber: () => publicClient.getBlockNumber(),
    readContract: async (args) =>
      (
        await publicClient.call({
          to: args.to,
          data: args.data,
          blockNumber: args.blockNumber
        })
      ).data ?? '0x',
    getLogs: async (filter) =>
      (await publicClient.getLogs(normalizeLogFilter(filter))).map(
        normalizeLog
      ),
    getTransactionReceipt: async (hash) => {
      const receipt = await publicClient.getTransactionReceipt({ hash })
      return receipt === null ? null : normalizeReceipt(receipt)
    },
    waitForTransactionReceipt: async (args) =>
      normalizeReceipt(
        await publicClient.waitForTransactionReceipt({
          hash: args.hash,
          confirmations: args.confirmations,
          timeout: args.timeoutMs,
          pollingInterval: args.pollIntervalMs
        })
      )
  }
}

export function viemWalletSubmitter(
  walletClient: ViemWalletClient
): PayyEvmSubmitter {
  return {
    getChainId: () => walletClient.getChainId(),
    getAddress: async () => walletClient.account?.address ?? null,
    sendTransaction: (request) =>
      walletClient.sendTransaction({
        ...request,
        from: request.from ?? walletClient.account?.address
      })
  }
}

export function toViemTransaction(
  prepared: PreparedTransactionInput,
  options: ViemTransactionOptions = {}
): ViemTransaction {
  const call = preparedCall(prepared)
  if (options.chain !== undefined && options.chain.id !== call.chainId) {
    throw new PayyClientError(
      'chain_id_mismatch',
      'viem transaction chain id mismatch',
      {
        expected: call.chainId,
        actual: options.chain.id
      }
    )
  }
  const request = call.bridgeRequest
  const account = options.account ?? request.from
  if (
    request.from !== undefined
    && account !== undefined
    && accountAddress(account).toLowerCase() !== request.from.toLowerCase()
  ) {
    throw new PayyClientError(
      'evm_account_mismatch',
      'viem transaction account does not match request.from'
    )
  }
  return {
    ...(account === undefined ? {} : { account }),
    ...(options.chain === undefined ? {} : { chain: options.chain }),
    to: request.to,
    data: request.data,
    value: request.value ?? 0n,
    ...(request.gasLimit === undefined ? {} : { gas: request.gasLimit })
  }
}

export function viemRawTransactionSubmitter(
  publicClient: ViemPublicClient
): PayyRawTransactionSubmitter {
  return {
    getChainId: () => publicClient.getChainId(),
    getTransactionCount: async (args) =>
      BigInt(await publicClient.getTransactionCount(args)),
    estimateGas: (request) => publicClient.estimateGas(request),
    getFeeData: () => publicClient.estimateFeesPerGas(),
    sendRawTransaction: (rawTransaction) =>
      publicClient.sendRawTransaction({
        serializedTransaction: rawTransaction
      }),
    getLocalAddress: async (evmPrivateKey) =>
      privateKeyToAccount(evmPrivateKey).address,
    sendLocalTransaction: async (evmPrivateKey, request) => {
      const account = privateKeyToAccount(evmPrivateKey)
      if (
        request.from !== undefined
        && request.from.toLowerCase() !== account.address.toLowerCase()
      ) {
        throw new PayyClientError(
          'evm_account_mismatch',
          'viem local account does not match request.from'
        )
      }
      const nonce = await publicClient.getTransactionCount({
        address: account.address,
        blockTag: 'pending'
      })
      const gas =
        request.gasLimit
        ?? (await publicClient.estimateGas({
          ...request,
          from: request.from ?? account.address
        }))
      const fee = await publicClient.estimateFeesPerGas()
      const serializedTransaction = await account.signTransaction({
        chainId: await publicClient.getChainId(),
        to: request.to,
        data: request.data,
        value: request.value ?? 0n,
        gas,
        nonce,
        maxFeePerGas: fee.maxFeePerGas,
        maxPriorityFeePerGas: fee.maxPriorityFeePerGas,
        type: 'eip1559'
      })
      return publicClient.sendRawTransaction({ serializedTransaction })
    }
  }
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
  readonly address?: Address
  readonly blockNumber?: bigint
  readonly transactionIndex?: number
  readonly logIndex?: number
  readonly transactionHash?: B256
  readonly topics?: readonly B256[]
  readonly data?: Hex
}): PayyEvmLog {
  return {
    address: requireLogField(log.address, 'address'),
    blockNumber: requireLogField(log.blockNumber, 'blockNumber'),
    transactionIndex: requireLogField(log.transactionIndex, 'transactionIndex'),
    logIndex: requireLogField(log.logIndex, 'logIndex'),
    transactionHash: requireLogField(log.transactionHash, 'transactionHash'),
    topics: requireLogField(log.topics, 'topics'),
    data: requireLogField(log.data, 'data')
  }
}

function requireLogField<T>(value: T | null | undefined, field: string): T {
  if (value === null || value === undefined) {
    throw new PayyClientError(
      'missing_log_metadata',
      'viem log missing metadata',
      { field }
    )
  }
  return value
}

function normalizeReceipt(receipt: {
  readonly transactionHash: B256
  readonly blockNumber: bigint
  readonly status: 'success' | 'reverted'
}): PayyTransactionReceipt {
  return {
    transactionHash: receipt.transactionHash,
    blockNumber: receipt.blockNumber,
    status: receipt.status
  }
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

function accountAddress(account: ViemTransactionAccount): Address {
  if (typeof account === 'string') {
    return account
  }
  return account.address
}
