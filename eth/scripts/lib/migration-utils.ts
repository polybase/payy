import { readFile, writeFile } from 'fs/promises'
import {
  defineChain,
  parseAbi,
  type Chain,
  type Hex
} from 'viem'
import {
  goerli,
  mainnet,
  polygon,
  polygonAmoy,
  polygonMumbai,
  sepolia
} from 'viem/chains'

export const ROLLUP_ABI = parseAbi([
  'event MintAdded(bytes32 indexed mint_hash, uint256 value, bytes32 note_kind)',
  'event Minted(bytes32 indexed hash, bytes32 value, bytes32 note_kind)',
  'function mint(bytes32 mint_hash, bytes32 value, bytes32 note_kind)',
  'struct Mint { bytes32 note_kind; uint256 amount; bool spent; }',
  'function getMint(bytes32 hash) view returns (Mint)'
])

export interface MintEntry {
  mintHash: Hex
  value?: string
  noteKind?: Hex
  blockNumber?: string
  spent: boolean
}

export const requireEnv = (name: string): string => {
  const value = process.env[name]
  if (value === undefined || value === '') throw new Error(`${name} is required`)
  return value
}

export const normalizeHex = (value: string | undefined, label: string): Hex => {
  if (value === undefined || value === '') throw new Error(`${label} is required`)

  const normalized = value.startsWith('0x') ? value : `0x${value}`
  if (!/^0x[0-9a-fA-F]+$/.test(normalized)) {
    throw new Error(`${label} must be a hex string`)
  }

  return normalized as Hex
}

const chainById = new Map<number, Chain>([
  [mainnet.id, mainnet],
  [goerli.id, goerli],
  [sepolia.id, sepolia],
  [polygon.id, polygon],
  [polygonMumbai.id, polygonMumbai],
  [polygonAmoy.id, polygonAmoy]
])

export const resolveChain = (chainId: number, rpcUrl: string): Chain => {
  const known = chainById.get(chainId)
  if (known !== undefined) return known

  return defineChain({
    id: chainId,
    name: `chain-${chainId}`,
    nativeCurrency: { name: 'Native', symbol: 'ETH', decimals: 18 },
    rpcUrls: {
      default: {
        http: [rpcUrl]
      }
    }
  })
}

export const sleep = async (ms: number): Promise<void> => {
  await new Promise((resolve) => setTimeout(resolve, ms))
}

export const withRetry = async <T>(
  label: string,
  attempts: number,
  fn: () => Promise<T>
): Promise<T> => {
  let lastError: unknown

  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      return await fn()
    } catch (error) {
      lastError = error
      console.warn(`${label} failed (attempt ${attempt}/${attempts}):`, error)
      if (attempt < attempts) {
        await sleep(1000 * attempt)
      }
    }
  }

  throw lastError
}

export const chunkArray = <T>(items: T[], size: number): T[][] => {
  if (size <= 0) throw new Error('chunk size must be > 0')

  const chunks: T[][] = []
  for (let i = 0; i < items.length; i += size) {
    chunks.push(items.slice(i, i + size))
  }

  return chunks
}

export const mapParallel = async <T, R>(
  items: T[],
  concurrency: number,
  fn: (item: T) => Promise<R>
): Promise<R[]> => {
  const results = new Array<R>(items.length)
  let nextIndex = 0

  const worker = async () => {
    while (nextIndex < items.length) {
      const index = nextIndex++
      results[index] = await fn(items[index])
    }
  }

  const workers = Array.from({ length: Math.min(concurrency, items.length) }, worker)
  await Promise.all(workers)

  return results
}

export const saveMints = async (path: string, mints: Record<string, MintEntry>) => {
  await writeFile(path, JSON.stringify(mints, null, 2))
}

export const loadMints = async (path: string): Promise<Record<string, MintEntry>> => {
  try {
    const content = await readFile(path, 'utf8')
    return JSON.parse(content)
  } catch {
    return {}
  }
}
