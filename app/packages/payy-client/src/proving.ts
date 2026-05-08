import { Noir, type CompiledCircuit, type InputMap } from '@noir-lang/noir_js'
import { PayyClientError } from './errors'
import type { B256, Hex, PrivacyOperationKind } from './types'
import { bytesToHex, hexToBytes } from './utils'

export type NoirCircuitInput = InputMap

export type ProvingBackend = {
  prove(
    operation: PrivacyOperationKind,
    inputs: NoirCircuitInput
  ): Promise<{
    readonly proof: Hex
    readonly publicInputs: readonly B256[]
    readonly verificationKeyHash: B256
  }>
}

type CircuitCatalog = Partial<Record<PrivacyOperationKind, CompiledCircuit>>

type ProofData = {
  readonly proof: Uint8Array
  readonly publicInputs: readonly string[]
}

type UltraHonkBackendLike = {
  generateProof(
    compressedWitness: Uint8Array,
    options?: { readonly keccak?: boolean }
  ): Promise<ProofData>
  destroy?(): Promise<void>
}

type BbJsModule = {
  readonly UltraHonkBackend: new (
    bytecode: string,
    backendOptions?: { readonly threads?: number },
    circuitOptions?: { readonly recursive?: boolean }
  ) => UltraHonkBackendLike
}

const VERIFICATION_KEY_HASHES = {
  mint: '0x018ab6045d8d028222d44bf35d371c951420aa4a600573a4157853d7b69a619a',
  burn: '0x2dca74dc2fd7c4403022fbf3fb389dd205c8d4c241fd4a3fa6b97a0bc87ec9f5',
  transfer_send:
    '0x2e1ddce51b98d5291bacf99f1c417d6fb3ba850f9082446f9814fa63c48fa8cc',
  transfer_claim:
    '0x1bd7966ba80f33f8218fd13c8fc7b521f274b1e0593458cceb1e2ca1198b8026'
} satisfies Record<PrivacyOperationKind, B256>

export class BbJsProvingBackend implements ProvingBackend {
  private readonly circuits: CircuitCatalog

  constructor(circuits: Partial<CircuitCatalog> = {}) {
    this.circuits = { ...circuits }
  }

  async prove(
    operation: PrivacyOperationKind,
    inputs: NoirCircuitInput
  ): Promise<{
    readonly proof: Hex
    readonly publicInputs: readonly B256[]
    readonly verificationKeyHash: B256
  }> {
    const circuit = await this.circuit(operation)
    const noir = new Noir(circuit)
    const { witness } = await noir.execute(inputs)
    const { UltraHonkBackend } = (await import('@aztec/bb.js')) as BbJsModule
    const backend = new UltraHonkBackend(
      circuit.bytecode,
      { threads: 1 },
      { recursive: false }
    )
    try {
      const proofData = await backend.generateProof(witness, { keccak: false })
      return {
        proof: bytesToHex(proofData.proof),
        publicInputs: proofData.publicInputs.map(normalizeProvingField),
        verificationKeyHash: VERIFICATION_KEY_HASHES[operation]
      }
    } finally {
      await backend.destroy?.()
    }
  }

  private async circuit(
    operation: PrivacyOperationKind
  ): Promise<CompiledCircuit> {
    const configured = this.circuits[operation]
    if (configured !== undefined) {
      return configured
    }
    const loaded = await loadDefaultCircuit(operation)
    this.circuits[operation] = loaded
    return loaded
  }
}

export async function loadBbJsProvingBackend(): Promise<ProvingBackend> {
  return new BbJsProvingBackend()
}

export function normalizeProvingField(value: string): B256 {
  const hex = value.startsWith('0x') ? value : `0x${value}`
  const bytes = hexToBytes(hex as Hex)
  if (bytes.length > 32) {
    throw new PayyClientError(
      'proof_output_malformed',
      'proving backend returned field wider than bytes32'
    )
  }
  const out = new Uint8Array(32)
  out.set(bytes, 32 - bytes.length)
  return bytesToHex(out)
}

async function loadDefaultCircuit(
  operation: PrivacyOperationKind
): Promise<CompiledCircuit> {
  switch (operation) {
    case 'mint':
      return (await import('./artifacts/mint.json'))
        .default as unknown as CompiledCircuit
    case 'burn':
      return (await import('./artifacts/burn.json'))
        .default as unknown as CompiledCircuit
    case 'transfer_send':
      return (await import('./artifacts/transfer_send.json'))
        .default as unknown as CompiledCircuit
    case 'transfer_claim':
      return (await import('./artifacts/transfer_claim.json'))
        .default as unknown as CompiledCircuit
  }
}
