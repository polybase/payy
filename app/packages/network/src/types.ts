export type Element = string

export interface MerklePath {
  siblings: Element[]
}

export interface Note {
  address: Element
  psi: Element
  value: Element
  source: Element
  token: string
}

export interface StoredNote {
  note: Note
  commitment: Element
}

export interface InputNote {
  note: Note
  secret_key: Element
  merkle_path: MerklePath
}

export interface SnarkWitness {
  V1: {
    proof: string
    instances: Element[][]
  }
}

export type WalletActivityResultStatus =
  | null
  | 'success'
  | 'error'
  | 'cancelled'

export interface WalletActivity {
  parentId?: string
  kind: string
  stage: string
  result: WalletActivityResultStatus
  timestamp: number
  userCancel: boolean
  error: string | null
  errorCycles: number
  attempts: number
  okCycles: number
  data: any
}

export interface WalletState {
  version: string
  last_update: string | null
  address: Element | null
  invalid_notes: Record<Element, StoredNote>
  unspent_notes: Record<Element, StoredNote>
  spent_notes: Record<Element, StoredNote>
  activity: Record<string, any>
}

export interface WalletActivityTxn extends WalletActivity {
  data: {
    snark: SnarkWitness | null
    root: Element
    nullifiers: Element[]
    outputs: StoredNote[]
    inputs: StoredNote[]
    error: string | null
  }
}
