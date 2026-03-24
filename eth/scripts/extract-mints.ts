import {
  createPublicClient,
  decodeEventLog,
  encodeEventTopics,
  http,
  type Hex,
  type PublicClient
} from 'viem'
import {
  ROLLUP_ABI,
  chunkArray,
  loadMints,
  normalizeHex,
  requireEnv,
  saveMints,
  withRetry,
  type MintEntry
} from './lib/migration-utils'

export interface ExtractConfig {
  sourceRpcUrl: string
  sourceAddress: Hex
  outputFile: string
  startBlock?: bigint
  endBlock?: bigint
  blockBatchSize?: bigint
  concurrency?: number
  retryAttempts?: number
  client?: PublicClient
}

export async function extractMints(config: ExtractConfig) {
  console.log(`Loading state from ${config.outputFile}...`)
  const mints = await loadMints(config.outputFile)

  const sourceClient = config.client ?? createPublicClient({ transport: http(config.sourceRpcUrl) })
  const latestBlock = await sourceClient.getBlockNumber()

  const startBlock = config.startBlock ?? 0n
  const endBlock = config.endBlock ?? latestBlock

  console.log(`Extracting mints from ${startBlock} to ${endBlock} using ${config.sourceRpcUrl}`)

  const ranges: Array<{ from: bigint, to: bigint }> = []
  const batchSize = config.blockBatchSize ?? 10_000n

  for (let from = startBlock; from <= endBlock; from += batchSize) {
    const to = from + batchSize - 1n > endBlock ? endBlock : from + batchSize - 1n
    ranges.push({ from, to })
  }

  const chunks = chunkArray(ranges, config.concurrency ?? 5)

  const mintAddedTopic = encodeEventTopics({ abi: ROLLUP_ABI, eventName: 'MintAdded' })[0]
  const mintedTopic = encodeEventTopics({ abi: ROLLUP_ABI, eventName: 'Minted' })[0]

  for (const chunk of chunks) {
    await Promise.all(
      chunk.map(async (range) => {
        console.log(`Scanning blocks ${range.from} to ${range.to}`)
        const logs = await withRetry(`getLogs(${range.from}-${range.to})`, config.retryAttempts ?? 3, async () =>
          await sourceClient.getLogs({
            address: config.sourceAddress,
            fromBlock: range.from,
            toBlock: range.to,
            topics: [[mintAddedTopic, mintedTopic]]
          } as any)
        )

        for (const log of logs) {
          if (log.topics[0] === mintAddedTopic) {
            const decoded = decodeEventLog({
              abi: ROLLUP_ABI,
              eventName: 'MintAdded',
              data: log.data,
              topics: log.topics
            })
            const args = decoded.args
            const mintHash = args.mint_hash

            mints[mintHash] = {
              mintHash,
              value: args.value.toString(),
              noteKind: args.note_kind,
              blockNumber: (log.blockNumber ?? 0n).toString(),
              spent: mints[mintHash]?.spent ?? false
            }
          } else if (log.topics[0] === mintedTopic) {
            const decoded = decodeEventLog({
              abi: ROLLUP_ABI,
              eventName: 'Minted',
              data: log.data,
              topics: log.topics
            })
            const args = decoded.args
            const hash = args.hash

            if (mints[hash] !== undefined) {
              mints[hash].spent = true
            } else {
              const entry: MintEntry = {
                mintHash: hash,
                spent: true
              }
              mints[hash] = entry
            }
          }
        }
      })
    )

    console.log(`Saving ${Object.keys(mints).length} mints to ${config.outputFile}...`)
    await saveMints(config.outputFile, mints)
  }

  console.log('Extraction complete.')
}

async function main() {
  const sourceRpcUrl = requireEnv('SOURCE_RPC_URL')
  const sourceAddress = normalizeHex(requireEnv('SOURCE_CONTRACT_ADDRESS'), 'SOURCE_CONTRACT_ADDRESS')
  const startBlock = process.env.START_BLOCK !== undefined ? BigInt(process.env.START_BLOCK) : undefined
  const endBlock = process.env.END_BLOCK !== undefined ? BigInt(process.env.END_BLOCK) : undefined
  const outputFile = process.env.OUTPUT_FILE ?? 'mints.json'
  const blockBatchSize = process.env.BLOCK_BATCH_SIZE !== undefined ? BigInt(process.env.BLOCK_BATCH_SIZE) : undefined
  const concurrency = process.env.RPC_CONCURRENCY !== undefined ? Number(process.env.RPC_CONCURRENCY) : undefined
  const retryAttempts = process.env.RPC_RETRY_ATTEMPTS !== undefined ? Number(process.env.RPC_RETRY_ATTEMPTS) : undefined

  await extractMints({
    sourceRpcUrl,
    sourceAddress,
    outputFile,
    startBlock,
    endBlock,
    blockBatchSize,
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
