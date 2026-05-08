import { PayyClientError } from './errors'
import type {
  Address,
  B256,
  EvmAccount,
  Hex,
  PrivacyAccount,
  PrivacyAddress
} from './types'

const HEX = '0123456789abcdef'
const HEX_PATTERN = /^[0-9a-fA-F]*$/

export function zeroHash(): B256 {
  return `0x${'0'.repeat(64)}`
}

export function assertHex(
  value: string,
  bytes?: number
): asserts value is B256 {
  const expectedLength = bytes === undefined ? undefined : 2 + bytes * 2
  if (
    !value.startsWith('0x')
    || value.length % 2 !== 0
    || !HEX_PATTERN.test(value.slice(2))
  ) {
    throw new PayyClientError('invalid_hex', 'invalid hex value')
  }
  if (expectedLength !== undefined && value.length !== expectedLength) {
    throw new PayyClientError('invalid_hex', 'invalid hex length')
  }
}

export function hexToBytes(hex: string): Uint8Array {
  assertHex(hex)
  const out = new Uint8Array((hex.length - 2) / 2)
  for (let i = 0; i < out.length; i += 1) {
    out[i] = Number.parseInt(hex.slice(2 + i * 2, 4 + i * 2), 16)
  }
  return out
}

export function canonicalHex(hex: string, bytes?: number): Hex {
  assertHex(hex, bytes)
  return `0x${hex.slice(2).toLowerCase()}` as Hex
}

export function canonicalAddress(address: Address): Address {
  return canonicalHex(address, 20) as Address
}

export function bytesToHex(bytes: Uint8Array): B256 {
  let out = '0x'
  for (const byte of bytes) {
    out += HEX[(byte >> 4) & 15]
    out += HEX[byte & 15]
  }
  return out as B256
}

export function pad32(hex: string): B256 {
  const canonical = canonicalHex(hex)
  return `0x${canonical.slice(2).padStart(64, '0')}` as B256
}

export function encodeUtf8(value: string): Uint8Array {
  return new TextEncoder().encode(value)
}

export function decodeUtf8(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes)
}

export function privacyAddress(bytes: B256): PrivacyAddress {
  return { bytes: canonicalHex(bytes, 32) as B256 }
}

export function privacyAccountAddress(account: PrivacyAccount): PrivacyAddress {
  if ('privacyAddress' in account) {
    return privacyAddress(account.privacyAddress.bytes)
  }
  return privacyAddress(account.bytes)
}

export function evmAccountAddress(account: EvmAccount): Address {
  if (typeof account === 'string') {
    return canonicalAddress(account)
  }
  return canonicalAddress(account.address)
}

export function ensureAmountNonZero(amount: bigint): void {
  if (amount === 0n) {
    throw new PayyClientError('amount_zero', 'amount must be non-zero')
  }
}

export function addressToBigInt(address: Address): bigint {
  assertHex(address, 20)
  return BigInt(address)
}

export function b256ToBigInt(value: B256): bigint {
  assertHex(value, 32)
  return BigInt(value)
}
