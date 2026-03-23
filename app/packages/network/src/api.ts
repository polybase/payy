import axios from 'axios'

export interface Block {
  hash: string
  block: {
    content: BlockContent
    signature: number[]
  }
  time: number
}

export interface BlockContent {
  header: {
    height: number
  }
  state: {
    root_hash: string
    txns: Txn[]
  }
  last_block_hash: number[]
}

export interface Txn {
  block_height: number
  time: number
  hash: string
  proof: TxnProof
  index_in_block: number
}

export interface TxnProof {
  proof: string
  public_inputs: {
    input_commitments: [string, string]
    output_commitments: [string, string]
    messages: string[]
  }
}

export interface BlocksResponse {
  blocks: Block[]
  cursor: {
    before: string | null
    after: string | null
  }
}

export interface TxnsResponse {
  txns: Txn[]
  cursor: {
    before: string | null
    after: string | null
  }
}

export interface StatsResponse {
  last_7_days_txns: StatsDay[]
}

export interface StatsDay {
  date: string
  count: number
}

export interface TxnResponse {
  txn: Txn
}

export interface ElementRepsonse {
  height: number
  element: string
}

export async function getBlocks(): Promise<BlocksResponse> {
  const res = await axios.get(
    `${process.env.NEXT_PUBLIC_ROLLUP_URL}/blocks?limit=10&skip_empty=true`
  )
  return res.data
}

export async function getBlock(hash: string): Promise<Block> {
  const res = await axios.get(
    `${process.env.NEXT_PUBLIC_ROLLUP_URL}/blocks/${hash}`
  )
  return res.data
}

export interface ListTxnParams {
  limit: number
}

export async function getTxn(hash: string): Promise<TxnResponse> {
  const res = await axios.get(
    `${process.env.NEXT_PUBLIC_ROLLUP_URL}/transactions/${hash}`
  )
  return res.data
}

export async function getTxns(): Promise<TxnsResponse> {
  const res = await axios.get(
    `${process.env.NEXT_PUBLIC_ROLLUP_URL}/transactions?limit=10`
  )
  return res.data
}

export async function getStats(): Promise<StatsResponse> {
  const res = await axios.get(`${process.env.NEXT_PUBLIC_ROLLUP_URL}/stats`)
  return res.data
}

export async function getElement(hash: string): Promise<ElementRepsonse> {
  const res = await axios.get(
    `${process.env.NEXT_PUBLIC_ROLLUP_URL}/elements/${hash}`
  )
  return res.data
}
