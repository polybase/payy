import type { B256, Note, PrivacyAddress, PrivacyAddressPrefix } from './types'
import { keccak_256 } from '@noble/hashes/sha3'
import { PayyClientError } from './errors'
import { bytesToHex, hexToBytes, privacyAddress } from './utils'

const BN254_SCALAR_MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n
const GRUMPKIN_B = 17n
const SIGN_MASK = 0x80
const RESERVED_MASK = 0x40

type BbSyncApi = {
  poseidon2Hash(command: { readonly inputs: readonly Uint8Array[] }): {
    readonly hash: Uint8Array
  }
  schnorrComputePublicKey(command: { readonly privateKey: Uint8Array }): {
    readonly publicKey: { readonly x: Uint8Array; readonly y: Uint8Array }
  }
  schnorrConstructSignature(command: {
    readonly message: Uint8Array
    readonly privateKey: Uint8Array
  }): { readonly s: Uint8Array; readonly e: Uint8Array }
  grumpkinGetRandomFr(command: { readonly dummy: number }): {
    readonly value: Uint8Array
  }
  grumpkinMul(command: {
    readonly point: { readonly x: Uint8Array; readonly y: Uint8Array }
    readonly scalar: Uint8Array
  }): { readonly point: { readonly x: Uint8Array; readonly y: Uint8Array } }
}

type BbJsModule = {
  readonly BarretenbergSync: {
    initSingleton(): Promise<BbSyncApi>
  }
}

export type GrumpkinPublicKey = {
  readonly x: B256
  readonly y: B256
}

export async function bb(): Promise<BbSyncApi> {
  const bbModule = (await import('@aztec/bb.js')) as unknown as BbJsModule
  return bbModule.BarretenbergSync.initSingleton()
}

export async function poseidonHash(inputs: readonly bigint[]): Promise<B256> {
  const result = (await bb()).poseidon2Hash({
    inputs: inputs.map(bigIntToBytes32)
  })
  return bytesToHex(result.hash)
}

export async function noteCommitment(note: Note): Promise<B256> {
  if (note.kind === 0n) {
    return zeroHash()
  }
  return poseidonHash([
    note.kind,
    note.token,
    note.nonce,
    note.psi,
    note.owner,
    note.value
  ])
}

export async function noteNullifier(note: Note): Promise<B256> {
  const commitment = await noteCommitment(note)
  if (commitment === zeroHash()) {
    return zeroHash()
  }
  return poseidonHash([BigInt(commitment), note.psi])
}

export async function firstNonceHash(
  kind: bigint,
  token: bigint,
  owner: bigint
): Promise<B256> {
  return poseidonHash([kind, token, owner, 0n, 0n])
}

export async function nextNonceHash(
  kind: bigint,
  token: bigint,
  owner: bigint,
  outputNonce: bigint,
  inputPsi: bigint
): Promise<B256> {
  return poseidonHash([kind, token, owner, outputNonce, inputPsi])
}

export async function hashMerge(inputs: readonly bigint[]): Promise<bigint> {
  return BigInt(await poseidonHash(inputs))
}

export async function txCommitment(
  chainId: bigint,
  bridgeAddress: bigint,
  inputCommitment0: bigint,
  inputCommitment1: bigint,
  outputCommitment0: bigint,
  outputCommitment1: bigint,
  burnRecipient: bigint,
  mintFrom: bigint,
  userEncryptedKeyHashValue: bigint,
  recipientEncryptedKeyHashValue: bigint,
  receivePrefixValue: bigint
): Promise<bigint> {
  return hashMerge([
    1n,
    chainId,
    bridgeAddress,
    inputCommitment0,
    inputCommitment1,
    outputCommitment0,
    outputCommitment1,
    burnRecipient,
    mintFrom,
    userEncryptedKeyHashValue,
    recipientEncryptedKeyHashValue,
    receivePrefixValue
  ])
}

export function receivePrefix(owner: bigint): bigint {
  const bytes = bigIntToBytes32(owner)
  return bytesToBigInt(bytes.slice(0, 6))
}

export function encryptedKeyHash(words: readonly B256[]): bigint {
  const hash = BigInt(
    bytesToHex(keccak_256(concatBytes(words.map(hexToBytes))))
  )
  return hash % BN254_SCALAR_MODULUS
}

export function userEncryptedKeyHash(words: readonly B256[]): bigint {
  return encryptedKeyHash(words)
}

export async function publicKeyFromPrivateKey(
  privateKey: B256
): Promise<GrumpkinPublicKey> {
  const publicKey = (await bb()).schnorrComputePublicKey({
    privateKey: hexToBytes(privateKey)
  }).publicKey
  return {
    x: bytesToHex(publicKey.x),
    y: bytesToHex(publicKey.y)
  }
}

export async function grumpkinScalarMulPoint(
  privateKey: B256,
  publicKey: GrumpkinPublicKey
): Promise<GrumpkinPublicKey> {
  const point = (await bb()).grumpkinMul({
    point: {
      x: hexToBytes(publicKey.x),
      y: hexToBytes(publicKey.y)
    },
    scalar: hexToBytes(privateKey)
  }).point
  return {
    x: bytesToHex(point.x),
    y: bytesToHex(point.y)
  }
}

export async function encryptedNote(
  note: Note,
  symmetricKey: bigint
): Promise<readonly [B256, B256, B256, B256, B256]> {
  const fields = await encryptPayload(
    [note.token, note.nonce, note.psi, note.owner, note.value],
    symmetricKey
  )
  return fields as readonly [B256, B256, B256, B256, B256]
}

export async function encryptPayload(
  payload: readonly bigint[],
  symmetricKey: bigint
): Promise<readonly B256[]> {
  return Promise.all(
    payload.map(async (value, index) =>
      bigIntToB256(
        fieldMod(value + (await hashMerge([symmetricKey, BigInt(index)])))
      )
    )
  )
}

export async function encryptKeyForPublicKey(
  symmetricKey: bigint,
  publicKey: GrumpkinPublicKey
): Promise<readonly [B256, B256, B256, B256]> {
  const ephemeralPrivateKey = await randomNonZeroField()
  const ephemeralPublicKey = await publicKeyFromPrivateKey(
    bigIntToB256(ephemeralPrivateKey)
  )
  const shared = await grumpkinScalarMulPoint(
    bigIntToB256(ephemeralPrivateKey),
    publicKey
  )
  return [
    ephemeralPublicKey.x,
    ephemeralPublicKey.y,
    bigIntToB256(
      fieldMod(
        symmetricKey + (await hashMerge([BigInt(shared.x), BigInt(shared.y)]))
      )
    ),
    zeroHash()
  ]
}

export async function encryptChainKey(
  symmetricKey: bigint,
  chainPublicKey: GrumpkinPublicKey
): Promise<readonly [B256, B256, B256]> {
  const symmetricKeyHex = bigIntToB256(symmetricKey)
  const publicKey = await publicKeyFromPrivateKey(symmetricKeyHex)
  const shared = await grumpkinScalarMulPoint(symmetricKeyHex, chainPublicKey)
  return [
    publicKey.x,
    publicKey.y,
    bigIntToB256(
      fieldMod(
        symmetricKey + (await hashMerge([BigInt(shared.x), BigInt(shared.y)]))
      )
    )
  ]
}

export function privacyAddressFromPublicKey(
  publicKey: GrumpkinPublicKey
): PrivacyAddress {
  const bytes = hexToBytes(publicKey.x)
  if (BigInt(publicKey.y) % 2n === 1n) {
    bytes[0] |= SIGN_MASK
  }
  return privacyAddress(bytesToHex(bytes))
}

export async function privacyAddressOwner(
  address: PrivacyAddress
): Promise<bigint> {
  const { x, y } = publicKeyFromPrivacyAddress(address)
  return BigInt(await poseidonHash([x, y]))
}

export async function privacyAddressPrefix(
  address: PrivacyAddress
): Promise<PrivacyAddressPrefix> {
  const owner = await privacyAddressOwner(address)
  return {
    bytes: `0x${owner.toString(16).padStart(64, '0').slice(0, 12)}`
  }
}

export async function ephemeralOwner(privateKey: B256): Promise<bigint> {
  const publicKey = await publicKeyFromPrivateKey(privateKey)
  return privacyAddressOwner(privacyAddressFromPublicKey(publicKey))
}

export async function randomField(): Promise<bigint> {
  return BigInt(
    bytesToHex((await bb()).grumpkinGetRandomFr({ dummy: 0 }).value)
  )
}

export async function randomNonZeroField(): Promise<bigint> {
  for (;;) {
    const value = await randomField()
    if (value !== 0n) {
      return value
    }
  }
}

export async function randomNonZeroB256(): Promise<B256> {
  return bigIntToB256(await randomNonZeroField())
}

export async function randomU240(): Promise<bigint> {
  for (;;) {
    const bytes = (await bb()).grumpkinGetRandomFr({ dummy: 0 }).value
    bytes[0] = 0
    bytes[1] = 0
    const value = BigInt(bytesToHex(bytes))
    if (value !== 0n) {
      return value
    }
  }
}

export function zeroHash(): B256 {
  return `0x${'0'.repeat(64)}`
}

export function bigIntToB256(value: bigint): B256 {
  return bytesToHex(bigIntToBytes32(value))
}

export function fieldSub(lhs: bigint, rhs: bigint): bigint {
  return mod(lhs - rhs)
}

export function publicKeyFromPrivacyAddress(address: PrivacyAddress): {
  readonly x: bigint
  readonly y: bigint
} {
  const bytes = hexToBytes(address.bytes)
  if ((bytes[0] & RESERVED_MASK) !== 0) {
    throw new PayyClientError(
      'invalid_privacy_address',
      'invalid privacy address'
    )
  }
  const signIsOdd = (bytes[0] & SIGN_MASK) !== 0
  bytes[0] &= ~(SIGN_MASK | RESERVED_MASK)
  const x = bytesToBigInt(bytes)
  if (x >= BN254_SCALAR_MODULUS) {
    throw new PayyClientError(
      'invalid_privacy_address',
      'invalid privacy address'
    )
  }
  let y = modSqrt(mod(x * x * x - GRUMPKIN_B))
  if (y === null) {
    throw new PayyClientError(
      'invalid_privacy_address',
      'invalid privacy address'
    )
  }
  if ((y % 2n === 1n) !== signIsOdd) {
    y = BN254_SCALAR_MODULUS - y
  }
  return { x, y }
}

function modSqrt(value: bigint): bigint | null {
  if (value === 0n) {
    return 0n
  }
  const p = BN254_SCALAR_MODULUS
  if (modPow(value, (p - 1n) / 2n, p) !== 1n) {
    return null
  }
  let q = p - 1n
  let s = 0n
  while (q % 2n === 0n) {
    q /= 2n
    s += 1n
  }
  let z = 2n
  while (modPow(z, (p - 1n) / 2n, p) !== p - 1n) {
    z += 1n
  }
  let m = s
  let c = modPow(z, q, p)
  let t = modPow(value, q, p)
  let r = modPow(value, (q + 1n) / 2n, p)
  while (t !== 1n) {
    let i = 1n
    let t2i = mod(t * t)
    while (t2i !== 1n) {
      t2i = mod(t2i * t2i)
      i += 1n
      if (i === m) {
        return null
      }
    }
    const b = modPow(c, 1n << (m - i - 1n), p)
    r = mod(r * b)
    c = mod(b * b)
    t = mod(t * c)
    m = i
  }
  return r
}

function mod(value: bigint): bigint {
  const result = value % BN254_SCALAR_MODULUS
  return result < 0n ? result + BN254_SCALAR_MODULUS : result
}

export function fieldMod(value: bigint): bigint {
  return mod(value)
}

export function bytesToBigInt(bytes: Uint8Array): bigint {
  return BigInt(bytesToHex(bytes))
}

export async function computeMerkleRoot(
  leaf: bigint,
  pathElement: bigint,
  siblings: readonly bigint[]
): Promise<bigint> {
  let value = leaf
  for (let index = 0; index < siblings.length; index += 1) {
    const bit = ((pathElement >> BigInt(index)) & 1n) === 1n
    value = bit
      ? await hashMerge([siblings[index], value])
      : await hashMerge([value, siblings[index]])
  }
  return value
}

function modPow(base: bigint, exponent: bigint, modulus: bigint): bigint {
  let result = 1n
  let power = base % modulus
  let remaining = exponent
  while (remaining > 0n) {
    if ((remaining & 1n) === 1n) {
      result = (result * power) % modulus
    }
    power = (power * power) % modulus
    remaining >>= 1n
  }
  return result
}

function bigIntToBytes32(value: bigint): Uint8Array {
  const out = new Uint8Array(32)
  let remaining = value
  for (let i = 31; i >= 0; i -= 1) {
    out[i] = Number(remaining & 255n)
    remaining >>= 8n
  }
  return out
}

function concatBytes(chunks: readonly Uint8Array[]): Uint8Array {
  const out = new Uint8Array(
    chunks.reduce((sum, chunk) => sum + chunk.length, 0)
  )
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.length
  }
  return out
}
