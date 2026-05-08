import { keccak_256 } from '@noble/hashes/sha3'
import { PayyClientError } from './errors'
import type {
  Address,
  B256,
  Hex,
  PayyMerklePath,
  PayyEvmReadClient,
  TxnData
} from './types'
import {
  assertHex,
  bytesToHex,
  encodeUtf8,
  hexToBytes,
  zeroHash
} from './utils'

const WORD_BYTES = 32

type WordArray3 = readonly [B256, B256, B256]
type WordArray4 = readonly [B256, B256, B256, B256]
type WordArray5 = readonly [B256, B256, B256, B256, B256]

export class BridgeClient {
  private readonly privacyBridge: Address
  private readonly readClient: PayyEvmReadClient

  constructor(privacyBridge: Address, readClient: PayyEvmReadClient) {
    this.privacyBridge = privacyBridge
    this.readClient = readClient
  }

  async getRoot(): Promise<B256> {
    return wordAt(await this.read(this.encodeGetRootCall()), 0)
  }

  async getMerklePath(commitment: B256): Promise<PayyMerklePath> {
    return decodeMerklePath(
      await this.read(this.encodeGetMerklePathCall(commitment))
    )
  }

  async elementExists(element: B256): Promise<boolean> {
    const word = wordAt(
      await this.read(this.encodeElementExistsCall(element)),
      0
    )
    if (word === zeroHash()) {
      return false
    }
    if (word === wordFromBigInt(1n)) {
      return true
    }
    throw malformedReturn('elementExists bool')
  }

  async getTxnHashByNonceHash(nonceHash: B256): Promise<B256 | null> {
    const value = wordAt(
      await this.read(this.encodeGetTxnHashByNonceHashCall(nonceHash)),
      0
    )
    return value === zeroHash() ? null : value
  }

  async getTxnHashByCommitment(commitment: B256): Promise<B256 | null> {
    const value = wordAt(
      await this.read(this.encodeGetTxnHashByCommitmentCall(commitment)),
      0
    )
    return value === zeroHash() ? null : value
  }

  async getTxnData(txnHash: B256): Promise<TxnData> {
    return decodeTxnData(await this.read(this.encodeGetTxnDataCall(txnHash)))
  }

  computeTxHash(
    verificationKeyHash: B256,
    proof: Hex,
    publicInputs: readonly B256[]
  ): B256 {
    return keccakHex(
      encodeBridgeProofParams(verificationKeyHash, proof, publicInputs)
    )
  }

  async getChainPublicKey(): Promise<readonly [B256, B256]> {
    const response = await this.read(this.encodeGetChainPublicKeyCall())
    return [wordAt(response, 0), wordAt(response, 1)]
  }

  encodeGetRootCall(): Hex {
    return encodeCall('getRoot()', [])
  }

  encodeGetMerklePathCall(commitment: B256): Hex {
    return encodeCall('getMerklePath(bytes32)', [word(commitment)])
  }

  encodeElementExistsCall(element: B256): Hex {
    return encodeCall('elementExists(bytes32)', [word(element)])
  }

  encodeGetTxnHashByNonceHashCall(nonceHash: B256): Hex {
    return encodeCall('getTxnHashByNonceHash(bytes32)', [word(nonceHash)])
  }

  encodeGetTxnHashByCommitmentCall(commitment: B256): Hex {
    return encodeCall('getTxnHashByCommitment(bytes32)', [word(commitment)])
  }

  encodeGetTxnDataCall(txnHash: B256): Hex {
    return encodeCall('getTxnData(bytes32)', [word(txnHash)])
  }

  encodeComputeTxHashCall(
    verificationKeyHash: B256,
    proof: Hex,
    publicInputs: readonly B256[]
  ): Hex {
    return encodeCall('computeTxHash(bytes32,bytes,bytes32[])', [
      encodeBridgeProofParams(verificationKeyHash, proof, publicInputs)
    ])
  }

  encodeGetChainPublicKeyCall(): Hex {
    return encodeCall('getChainPublicKey()', [])
  }

  encodeMintCall(
    verificationKeyHash: B256,
    proof: Hex,
    publicInputs: readonly B256[],
    userEncryptedKey: WordArray4 = zeroWordArray4()
  ): Hex {
    return encodeCall('mint(bytes32,bytes,bytes32[],bytes32[4])', [
      encodeProofCallParams(verificationKeyHash, proof, publicInputs, [
        ...userEncryptedKey
      ])
    ])
  }

  encodeBurnCall(
    verificationKeyHash: B256,
    proof: Hex,
    publicInputs: readonly B256[],
    userEncryptedKey: WordArray4 = zeroWordArray4()
  ): Hex {
    return encodeCall('burn(bytes32,bytes,bytes32[],bytes32[4])', [
      encodeProofCallParams(verificationKeyHash, proof, publicInputs, [
        ...userEncryptedKey
      ])
    ])
  }

  encodeTransferCall(
    verificationKeyHash: B256,
    proof: Hex,
    publicInputs: readonly B256[],
    bridgeMemo: B256,
    userEncryptedKey: WordArray4 = zeroWordArray4(),
    recipientEncryptedKey: WordArray4 = zeroWordArray4()
  ): Hex {
    return encodeCall(
      'transfer(bytes32,bytes,bytes32[],bytes32[4],bytes32[4],bytes32)',
      [
        encodeProofCallParams(verificationKeyHash, proof, publicInputs, [
          ...userEncryptedKey,
          ...recipientEncryptedKey,
          bridgeMemo
        ])
      ]
    )
  }

  private async read(data: Hex): Promise<Hex> {
    return this.readClient.readContract({
      to: this.privacyBridge,
      data
    })
  }
}

export function externalTransferTopic(): B256 {
  return eventSignatureTopic('ExternalTransfer(bytes6,bytes32)')
}

export function eventSignatureTopic(signature: string): B256 {
  return keccakHex(bytesToHex(encodeUtf8(signature)))
}

function encodeBridgeProofParams(
  verificationKeyHash: B256,
  proof: Hex,
  publicInputs: readonly B256[]
): Hex {
  return encodeProofCallParams(verificationKeyHash, proof, publicInputs, [])
}

function encodeProofCallParams(
  verificationKeyHash: B256,
  proof: Hex,
  publicInputs: readonly B256[],
  trailingStaticWords: readonly B256[]
): Hex {
  const proofTail = dynamicBytes(proof)
  const publicInputsTail = dynamicWords(publicInputs)
  const headWords = 3 + trailingStaticWords.length
  const proofOffset = BigInt(headWords * WORD_BYTES)
  const publicInputsOffset = proofOffset + BigInt(hexByteLength(proofTail))
  return concatHex([
    word(verificationKeyHash),
    wordFromBigInt(proofOffset),
    wordFromBigInt(publicInputsOffset),
    ...trailingStaticWords.map(word),
    proofTail,
    publicInputsTail
  ])
}

function encodeCall(signature: string, params: readonly Hex[]): Hex {
  return concatHex([selector(signature), ...params])
}

function selector(signature: string): Hex {
  return `0x${keccakHex(bytesToHex(encodeUtf8(signature))).slice(2, 10)}` as Hex
}

function decodeTxnData(response: Hex): TxnData {
  let index = 0
  const verificationKeyHash = wordAt(response, index)
  index += 1
  const senderEncryptedNote = wordArray5(response, index)
  index += 5
  const recipientEncryptedNote = wordArray5(response, index)
  index += 5
  const senderChainEncryptedKey = wordArray3(response, index)
  index += 3
  const recipientChainEncryptedKey = wordArray3(response, index)
  index += 3
  const userEncryptedKey = wordArray4(response, index)
  index += 4
  const recipientEncryptedKey = wordArray4(response, index)
  index += 4
  const memo = wordAt(response, index)
  return {
    verificationKeyHash,
    senderEncryptedNote,
    recipientEncryptedNote,
    senderChainEncryptedKey,
    recipientChainEncryptedKey,
    userEncryptedKey,
    recipientEncryptedKey,
    memo
  }
}

function decodeMerklePath(response: Hex): PayyMerklePath {
  const root = wordAt(response, 0)
  const offset = Number(wordBigInt(wordAt(response, 1)))
  if (offset % WORD_BYTES !== 0) {
    throw malformedReturn('getMerklePath offset')
  }
  const length = Number(wordBigInt(wordAtByteOffset(response, offset)))
  const start = offset + WORD_BYTES
  const out: B256[] = []
  for (let i = 0; i < length; i += 1) {
    out.push(wordAtByteOffset(response, start + i * WORD_BYTES))
  }
  return { root, siblings: out }
}

function wordArray3(response: Hex, index: number): WordArray3 {
  return [
    wordAt(response, index),
    wordAt(response, index + 1),
    wordAt(response, index + 2)
  ]
}

function wordArray4(response: Hex, index: number): WordArray4 {
  return [
    wordAt(response, index),
    wordAt(response, index + 1),
    wordAt(response, index + 2),
    wordAt(response, index + 3)
  ]
}

function wordArray5(response: Hex, index: number): WordArray5 {
  return [
    wordAt(response, index),
    wordAt(response, index + 1),
    wordAt(response, index + 2),
    wordAt(response, index + 3),
    wordAt(response, index + 4)
  ]
}

function wordAt(response: Hex, index: number): B256 {
  return wordAtByteOffset(response, index * WORD_BYTES)
}

function wordAtByteOffset(response: Hex, offset: number): B256 {
  assertHex(response)
  const start = 2 + offset * 2
  const end = start + WORD_BYTES * 2
  if (response.length < end) {
    throw malformedReturn('short contract return')
  }
  return `0x${response.slice(start, end)}` as B256
}

function word(value: B256): Hex {
  assertHex(value, 32)
  return value
}

function wordFromBigInt(value: bigint): B256 {
  return `0x${value.toString(16).padStart(64, '0')}` as B256
}

function wordBigInt(value: B256): bigint {
  return BigInt(value)
}

function dynamicBytes(value: Hex): Hex {
  const bytes = hexToBytes(value)
  return concatHex([
    wordFromBigInt(BigInt(bytes.length)),
    bytesToHex(padRight(bytes, WORD_BYTES))
  ])
}

function dynamicWords(values: readonly B256[]): Hex {
  return concatHex([
    wordFromBigInt(BigInt(values.length)),
    ...values.map((value) => word(value))
  ])
}

function padRight(bytes: Uint8Array, multiple: number): Uint8Array {
  const paddedLength = Math.ceil(bytes.length / multiple) * multiple
  const out = new Uint8Array(paddedLength)
  out.set(bytes)
  return out
}

function concatHex(parts: readonly Hex[]): Hex {
  return `0x${parts.map((part) => part.slice(2)).join('')}` as Hex
}

function hexByteLength(value: Hex): number {
  assertHex(value)
  return (value.length - 2) / 2
}

function keccakHex(value: Hex): B256 {
  return bytesToHex(keccak_256(hexToBytes(value)))
}

function zeroWordArray4(): WordArray4 {
  return [zeroHash(), zeroHash(), zeroHash(), zeroHash()]
}

function malformedReturn(detail: string): PayyClientError {
  return new PayyClientError(
    'contract_return_malformed',
    `malformed PrivacyBridge return: ${detail}`
  )
}
