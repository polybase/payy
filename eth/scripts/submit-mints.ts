import {
  createPublicClient,
  createWalletClient,
  encodeFunctionData,
  http,
  toHex,
  type Hex,
  type PublicClient,
  type WalletClient
} from 'viem'
import { privateKeyToAccount } from 'viem/accounts'
import {
  ROLLUP_ABI,
  loadMints,
  mapParallel,
  normalizeHex,
  requireEnv,
  resolveChain,
  withRetry,
  type MintEntry
} from './lib/migration-utils'

export interface SubmitConfig {
  targetRpcUrl: string
  targetAddress: Hex
  privateKey: Hex
  inputFile: string
  concurrency?: number
  retryAttempts?: number
  dryRun?: boolean
  printTx?: boolean
  publicClient?: PublicClient
  walletClient?: WalletClient
}

interface MigratableMint extends MintEntry {
  value: string
  noteKind: Hex
}

interface TxFeeOverrides {
  maxFeePerGas: bigint
  maxPriorityFeePerGas: bigint
}

function isMigratable(m: MintEntry): m is MigratableMint {
  if (m.spent) return false
  if (m.value === undefined || m.noteKind === undefined) {
    console.warn(`Skipping unspent mint ${m.mintHash} due to missing data (value/noteKind)`)
    return false
  }
  return true
}

async function getDoubledRecommendedFees(publicClient: PublicClient, retryAttempts: number): Promise<TxFeeOverrides> {
  const estimatedFees = await withRetry('estimateFeesPerGas', retryAttempts, async () => await publicClient.estimateFeesPerGas())
  if (estimatedFees.maxFeePerGas === undefined || estimatedFees.maxPriorityFeePerGas === undefined) {
    throw new Error('Target chain does not expose EIP-1559 fee suggestions')
  }

  return {
    maxFeePerGas: estimatedFees.maxFeePerGas * 2n,
    maxPriorityFeePerGas: estimatedFees.maxPriorityFeePerGas * 2n
  }
}

export async function submitMints(config: SubmitConfig) {
  console.log(`Loading mints from ${config.inputFile}...`)
  const mints = await loadMints(config.inputFile)
  const allMints = Object.values(mints)

  const candidates = allMints.filter(isMigratable)

  if (candidates.length === 0) {
    console.log('No candidates for migration found.')
    return
  }

  console.log(`Found ${candidates.length} unspent mints. Checking target chain for existence...`)

  const targetPublicClient = config.publicClient ?? createPublicClient({ transport: http(config.targetRpcUrl) })
  const chainId = await targetPublicClient.getChainId()
  const targetChain = resolveChain(chainId, config.targetRpcUrl)

  const account = privateKeyToAccount(config.privateKey)
  const targetWalletClient = config.walletClient ?? createWalletClient({
    account,
    chain: targetChain,
    transport: http(config.targetRpcUrl)
  })

  const candidateHashes = candidates.map((c) => c.mintHash)
  const existingMints = new Set<Hex>()

  await mapParallel(candidateHashes, config.concurrency ?? 5, async (hash) => {
    const mint = await withRetry(`getMint(${hash})`, config.retryAttempts ?? 3, async () =>
      await targetPublicClient.readContract({
        address: config.targetAddress,
        abi: ROLLUP_ABI,
        functionName: 'getMint',
        args: [hash]
      })
    )

    if (mint.amount > 0n) {
      existingMints.add(hash)
    }
  })

  const toMigrate = candidates.filter((c) => !existingMints.has(c.mintHash))

  if (toMigrate.length === 0) {
    console.log('All candidates already exist on target.')
    return
  }

  console.log(`Migrating ${toMigrate.length} mints...`)

  if (config.dryRun === true && config.printTx !== true) {
    console.log('Dry run enabled. Exiting.')
    return
  }

  const feeOverrides = config.printTx === true ? undefined : await getDoubledRecommendedFees(targetPublicClient, config.retryAttempts ?? 3)
  if (feeOverrides !== undefined) {
    console.log(
      `Using 2x recommended EIP-1559 fees: maxFeePerGas=${feeOverrides.maxFeePerGas}, maxPriorityFeePerGas=${feeOverrides.maxPriorityFeePerGas}`
    )
  }

  const accountAddress = targetWalletClient.account?.address ?? account.address
  let nonce = await targetPublicClient.getTransactionCount({
    address: accountAddress,
    blockTag: 'latest'
  })

  console.log(`Starting migration from nonce ${nonce}`)

  const successfulSubmissions: { mintHash: Hex; txHash: Hex }[] = []

  for (const [index, mint] of toMigrate.entries()) {
    const valueBytes32 = toHex(BigInt(mint.value), { size: 32 })

    if (config.printTx === true) {
      const data = encodeFunctionData({
        abi: ROLLUP_ABI,
        functionName: 'mint',
        args: [mint.mintHash, valueBytes32, mint.noteKind]
      })
      console.log(`\n--- Mint ${index + 1}/${toMigrate.length} ---`)
      console.log(`Target: ${config.targetAddress}`)
      console.log(`Mint Hash: ${mint.mintHash}`)
      console.log(`Value: ${mint.value} (${valueBytes32})`)
      console.log(`Data: ${data}`)
      console.log(`Nonce: ${nonce}`)
      nonce++
      continue
    }

    if (feeOverrides === undefined) {
      throw new Error('EIP-1559 fees are unavailable while submitting transactions')
    }

    const txHash = await withRetry('mint', config.retryAttempts ?? 3, async () =>
      await targetWalletClient.writeContract({
        account: targetWalletClient.account ?? account,
        chain: targetChain,
        address: config.targetAddress,
        abi: ROLLUP_ABI,
        functionName: 'mint',
        args: [mint.mintHash, valueBytes32, mint.noteKind],
        nonce,
        maxFeePerGas: feeOverrides.maxFeePerGas,
        maxPriorityFeePerGas: feeOverrides.maxPriorityFeePerGas
      })
    )
    successfulSubmissions.push({ mintHash: mint.mintHash, txHash })
    nonce++
  }

  if (config.printTx === true) return

  console.log(`Submitted ${successfulSubmissions.length} transactions. Waiting for receipts...`)

  let migratedCount = 0

  await mapParallel(successfulSubmissions, config.concurrency ?? 5, async ({ mintHash, txHash }) => {
    console.log(`Waiting for receipt: ${txHash}`)
    const receipt = await withRetry('waitForReceipt', config.retryAttempts ?? 3, async () =>
      await targetPublicClient.waitForTransactionReceipt({ hash: txHash })
    )

    if (receipt.status !== 'success') {
      throw new Error(`Transaction ${txHash} reverted`)
    }

    migratedCount++
    console.log(`Migrated ${migratedCount}/${toMigrate.length} (${mintHash})`)
  })
}

async function main() {
  const targetRpcUrl = requireEnv('TARGET_RPC_URL')
  const targetAddress = normalizeHex(requireEnv('TARGET_CONTRACT_ADDRESS'), 'TARGET_CONTRACT_ADDRESS')
  const privateKey = normalizeHex(requireEnv('PRIVATE_KEY'), 'PRIVATE_KEY')
  const inputFile = process.env.INPUT_FILE ?? 'filtered-mints.json'
  const concurrency = process.env.RPC_CONCURRENCY !== undefined ? Number(process.env.RPC_CONCURRENCY) : undefined
  const retryAttempts = process.env.RPC_RETRY_ATTEMPTS !== undefined ? Number(process.env.RPC_RETRY_ATTEMPTS) : undefined
  const dryRun = process.env.DRY_RUN === 'true'
  const printTx = process.env.PRINT_TX === 'true'

  await submitMints({
    targetRpcUrl,
    targetAddress,
    privateKey,
    inputFile,
    concurrency,
    retryAttempts,
    dryRun,
    printTx
  })
}

if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error(error)
      process.exit(1)
    })
}
