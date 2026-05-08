import bs58 from 'bs58'
import type {
  B256,
  ClaimLink,
  DirectSendDelivery,
  IncomingTransfer,
  Note,
  ParsedClaimLink
} from './types'
import { PayyClientError } from './errors'
import { ephemeralOwner, noteCommitment } from './crypto'
import { bytesToHex } from './utils'

const CLAIM_LINK_VERSION_V3 = 3
const BN254_SCALAR_MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n
const MAX_NOTE_VALUE = 1n << 240n

export class LinksClient {
  async parse(value: string): Promise<ParsedClaimLink> {
    const { message, secret } = splitLink(value)
    let payload: Uint8Array
    try {
      payload = bs58.decode(secret)
    } catch (cause) {
      throw new PayyClientError(
        'invalid_claim_link',
        'invalid claim link payload'
      )
    }
    if (payload[0] !== CLAIM_LINK_VERSION_V3) {
      throw new PayyClientError(
        'invalid_claim_link',
        'unsupported claim link version'
      )
    }
    if (payload[1] === 0) {
      return decodeDirect(message, payload)
    }
    if (payload[1] === 1) {
      return decodeEphemeral(message, payload)
    }
    throw new PayyClientError('invalid_claim_link', 'invalid claim link mode')
  }
}

export function encodeDirectClaimLink(
  delivery: DirectSendDelivery,
  message?: string
): ClaimLink {
  return encodeLink(message, [
    Uint8Array.from([CLAIM_LINK_VERSION_V3, 0]),
    noteToBytes(delivery.note)
  ])
}

export function encodeEphemeralClaimLink(
  transfer: IncomingTransfer,
  message?: string
): ClaimLink {
  return encodeLink(message, [
    Uint8Array.from([CLAIM_LINK_VERSION_V3, 1]),
    noteToBytes(transfer.note),
    hexToFixedBytes(transfer.ephemeralPrivateKey)
  ])
}

function splitLink(value: string): {
  readonly message: string | null
  readonly secret: string
} {
  const separator = value.indexOf('#')
  if (separator < 0) {
    throw new PayyClientError('invalid_claim_link', 'missing claim link secret')
  }
  const path = value.slice(0, separator)
  const secret = value.slice(separator + 1)
  if (path === '/s') {
    return { message: null, secret }
  }
  if (path.startsWith('/s/')) {
    const message = path.slice(3)
    if (message.includes('/')) {
      throw new PayyClientError(
        'invalid_claim_link',
        'invalid claim link route'
      )
    }
    return {
      message: message.length === 0 ? null : percentDecode(message),
      secret
    }
  }
  throw new PayyClientError('invalid_claim_link', 'invalid claim link route')
}

async function decodeDirect(
  message: string | null,
  payload: Uint8Array
): Promise<ParsedClaimLink> {
  const offset = { value: 2 }
  const note = bytesToNote(payload, offset)
  if (offset.value !== payload.length) {
    throw new PayyClientError('invalid_claim_link', 'invalid claim link length')
  }
  const commitment = await noteCommitment(note)
  await validateReceivedNote(note, commitment)
  return {
    claimSourceKind: 'direct',
    message,
    directNote: {
      note,
      commitment
    }
  }
}

async function decodeEphemeral(
  message: string | null,
  payload: Uint8Array
): Promise<ParsedClaimLink> {
  const offset = { value: 2 }
  const note = bytesToNote(payload, offset)
  const ephemeralPrivateKey = bytesToHex(
    slice(payload, offset.value, offset.value + 32)
  )
  offset.value += 32
  if (offset.value !== payload.length) {
    throw new PayyClientError('invalid_claim_link', 'invalid claim link length')
  }
  const commitment = await noteCommitment(note)
  await validateReceivedNote(note, commitment)
  if ((await ephemeralOwner(ephemeralPrivateKey)) !== note.owner) {
    throw new PayyClientError(
      'invalid_claim_link',
      'ephemeral key owner mismatch'
    )
  }
  return {
    claimSourceKind: 'ephemeral',
    message,
    incomingTransfer: {
      note,
      commitment,
      ephemeralPrivateKey
    }
  }
}

function encodeLink(
  message: string | undefined,
  chunks: readonly Uint8Array[]
): ClaimLink {
  const path = message === undefined ? '/s' : `/s/${percentEncode(message)}`
  return {
    value: `${path}#${bs58.encode(concat(chunks))}`
  }
}

function noteToBytes(note: Note): Uint8Array {
  return concat([
    Uint8Array.from([Number(note.kind)]),
    bigIntToBytes32(note.token).slice(12),
    compactField(note.nonce),
    bigIntToBytes32(note.psi),
    bigIntToBytes32(note.owner),
    compactField(note.value)
  ])
}

function bytesToNote(bytes: Uint8Array, offset: { value: number }): Note {
  const kind = readByte(bytes, offset)
  if (kind !== 1) {
    throw new PayyClientError('invalid_claim_link', 'invalid note kind')
  }
  return {
    kind: BigInt(kind),
    token: bytesToBigInt(readToken(bytes, offset)),
    nonce: bytesToBigInt(readCompactField(bytes, offset)),
    psi: bytesToBigInt(readWord(bytes, offset)),
    owner: bytesToBigInt(readWord(bytes, offset)),
    value: bytesToBigInt(readCompactField(bytes, offset))
  }
}

async function validateReceivedNote(
  note: Note,
  commitment: B256
): Promise<void> {
  if (
    note.nonce >= BN254_SCALAR_MODULUS
    || note.psi >= BN254_SCALAR_MODULUS
    || note.owner >= BN254_SCALAR_MODULUS
  ) {
    throw new PayyClientError(
      'invalid_claim_link',
      'field element out of range'
    )
  }
  if (note.nonce !== 0n || note.value === 0n) {
    throw new PayyClientError('invalid_claim_link', 'invalid claim note')
  }
  if (note.value >= MAX_NOTE_VALUE) {
    throw new PayyClientError('invalid_claim_link', 'note value out of range')
  }
  if ((await noteCommitment(note)) !== commitment) {
    throw new PayyClientError(
      'commitment_mismatch',
      'claim note commitment mismatch'
    )
  }
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

function compactField(value: bigint): Uint8Array {
  const word = bigIntToBytes32(value)
  const leadingZeros = word.findIndex((byte) => byte !== 0)
  const start = leadingZeros === -1 ? 32 : leadingZeros
  return concat([Uint8Array.from([start]), word.slice(start)])
}

function bytesToBigInt(bytes: Uint8Array): bigint {
  let value = 0n
  for (const byte of bytes) {
    value = (value << 8n) + BigInt(byte)
  }
  return value
}

function hexToFixedBytes(value: B256): Uint8Array {
  if (!value.startsWith('0x') || value.length !== 66) {
    throw new PayyClientError('invalid_claim_link', 'invalid bytes32 value')
  }
  const bytes = new Uint8Array(32)
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(value.slice(2 + i * 2, 4 + i * 2), 16)
  }
  return bytes
}

function readByte(bytes: Uint8Array, offset: { value: number }): number {
  if (offset.value >= bytes.length) {
    throw new PayyClientError(
      'invalid_claim_link',
      'claim link payload too short'
    )
  }
  const value = bytes[offset.value]
  offset.value += 1
  return value
}

function readToken(bytes: Uint8Array, offset: { value: number }): Uint8Array {
  const token = slice(bytes, offset.value, offset.value + 20)
  offset.value += 20
  const word = new Uint8Array(32)
  word.set(token, 12)
  return word
}

function readWord(bytes: Uint8Array, offset: { value: number }): Uint8Array {
  const word = slice(bytes, offset.value, offset.value + 32)
  offset.value += 32
  return word
}

function readCompactField(
  bytes: Uint8Array,
  offset: { value: number }
): Uint8Array {
  const leadingZeros = readByte(bytes, offset)
  if (leadingZeros > 32) {
    throw new PayyClientError('invalid_claim_link', 'invalid compact integer')
  }
  const valueLength = 32 - leadingZeros
  const compact = slice(bytes, offset.value, offset.value + valueLength)
  offset.value += valueLength
  const word = new Uint8Array(32)
  word.set(compact, leadingZeros)
  return word
}

function concat(chunks: readonly Uint8Array[]): Uint8Array {
  const length = chunks.reduce((sum, chunk) => sum + chunk.length, 0)
  const out = new Uint8Array(length)
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.length
  }
  return out
}

function slice(bytes: Uint8Array, start: number, end: number): Uint8Array {
  if (bytes.length < end) {
    throw new PayyClientError(
      'invalid_claim_link',
      'claim link payload too short'
    )
  }
  return bytes.slice(start, end)
}

function percentEncode(value: string): string {
  return encodeURIComponent(value).replace(
    /[!'()*]/g,
    (char) => `%${char.charCodeAt(0).toString(16).toUpperCase()}`
  )
}

function percentDecode(value: string): string {
  try {
    return decodeURIComponent(value)
  } catch {
    throw new PayyClientError('invalid_claim_link', 'invalid link message')
  }
}
