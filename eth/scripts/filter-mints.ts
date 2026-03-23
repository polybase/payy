import { createPublicClient, http, type Hex, type PublicClient } from 'viem'
import {
  ROLLUP_ABI,
  loadMints,
  mapParallel,
  normalizeHex,
  requireEnv,
  saveMints,
  withRetry,
  type MintEntry
} from './lib/migration-utils'

export interface FilterConfig {
  sourceRpcUrl: string
  sourceAddress: Hex
  inputFile: string
  outputFile: string
  concurrency?: number
  retryAttempts?: number
  client?: PublicClient
}

interface OnChainMint {
  note_kind: Hex
  amount: bigint
  spent: boolean
}

export async function filterMints(config: FilterConfig) {
  console.log(`Loading mints from ${config.inputFile}...`)
  const mints = await loadMints(config.inputFile)
  const allMints = Object.values(mints)

  const candidates = allMints.filter((m) => {
    return !m.spent && m.value !== undefined && m.noteKind !== undefined
  })

  if (candidates.length === 0) {
    console.log('No unspent candidates found in input file.')
    return
  }

  console.log(`Verifying ${candidates.length} candidates against source chain...`)

  const sourceClient = config.client ?? createPublicClient({ transport: http(config.sourceRpcUrl) })

  const validMints: Record<string, MintEntry> = {}
  let verifiedCount = 0
  let invalidCount = 0

  await mapParallel(candidates, config.concurrency ?? 10, async (mint) => {
    try {
      const result = await withRetry(`getMint(${mint.mintHash})`, config.retryAttempts ?? 3, async () =>
        await sourceClient.readContract({
          address: config.sourceAddress,
          abi: ROLLUP_ABI,
          functionName: 'getMint',
          args: [mint.mintHash]
        })
      ) as unknown as OnChainMint

      const onChainAmount = result.amount
      const onChainSpent = result.spent
      const onChainKind = result.note_kind

      if (onChainAmount === 0n) {
        console.warn(`Mint ${mint.mintHash} does not exist on chain (amount 0)`)
        invalidCount++
        return
      }

      if (onChainSpent) {
        console.warn(`Mint ${mint.mintHash} is marked spent on chain`)
        invalidCount++
        return
      }

      if (onChainAmount.toString() !== mint.value) {
        console.warn(`Mint ${mint.mintHash} amount mismatch: local=${mint.value} chain=${onChainAmount}`)
        invalidCount++
        return
      }

      if (onChainKind !== mint.noteKind) {
        console.warn(`Mint ${mint.mintHash} kind mismatch: local=${mint.noteKind} chain=${onChainKind}`)
        invalidCount++
        return
      }

      validMints[mint.mintHash] = mint
      verifiedCount++
    } catch (e) {
      console.error(`Failed to verify mint ${mint.mintHash}:`, e)
      invalidCount++
    }
  })

  console.log(`Verification complete. Valid: ${verifiedCount}, Invalid: ${invalidCount}`)
  await saveMints(config.outputFile, validMints)
  console.log(`Saved valid mints to ${config.outputFile}`)
}

async function main() {
  const sourceRpcUrl = requireEnv('SOURCE_RPC_URL')
  const sourceAddress = normalizeHex(requireEnv('SOURCE_CONTRACT_ADDRESS'), 'SOURCE_CONTRACT_ADDRESS')
  const inputFile = process.env.INPUT_FILE ?? 'mints.json'
  const outputFile = process.env.OUTPUT_FILE ?? 'filtered-mints.json'
  const concurrency = process.env.RPC_CONCURRENCY !== undefined ? Number(process.env.RPC_CONCURRENCY) : undefined
  const retryAttempts = process.env.RPC_RETRY_ATTEMPTS !== undefined ? Number(process.env.RPC_RETRY_ATTEMPTS) : undefined

  await filterMints({
    sourceRpcUrl,
    sourceAddress,
    inputFile,
    outputFile,
    concurrency,
    retryAttempts
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
