import type {
  B256,
  EphemeralKeyPair,
  Note,
  OwnerSignature,
  PrivacyAccount,
  PrivacyAddress,
  PrivacySigner,
  TxnData
} from './types'
import {
  bb,
  fieldSub,
  grumpkinScalarMulPoint,
  noteCommitment,
  poseidonHash,
  privacyAddressFromPublicKey,
  privacyAddressOwner,
  publicKeyFromPrivateKey,
  randomNonZeroB256
} from './crypto'
import {
  assertHex,
  b256ToBigInt,
  bytesToHex,
  hexToBytes,
  privacyAccountAddress
} from './utils'
import { PayyClientError } from './errors'
import { keccak_256 } from '@noble/hashes/sha3'

const GRUMPKIN_SCALAR_MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n
const DERIVATION_DOMAIN = new TextEncoder().encode('payy/grumpkin/v1')

export class LocalPrivacySigner implements PrivacySigner {
  private readonly privateKey: B256
  private cachedAccount?: {
    readonly address: PrivacyAddress
    readonly publicKeyX: B256
    readonly publicKeyY: B256
  }

  constructor(privateKey: B256) {
    validateGrumpkinPrivateKey(privateKey)
    this.privateKey = privateKey
  }

  async accounts(): Promise<readonly PrivacyAccount[]> {
    const account = await this.account()
    return [
      {
        privacyAddress: account.address,
        signer: this
      }
    ]
  }

  async signTxCommitment(args: {
    readonly privacyAccount: PrivacyAccount
    readonly txCommitment: B256
  }): Promise<OwnerSignature> {
    const account = await this.account()
    const address = privacyAccountAddress(args.privacyAccount)
    if (address.bytes !== account.address.bytes) {
      throw new PayyClientError(
        'privacy_account_mismatch',
        'privacy account mismatch'
      )
    }
    const signature = (await bb()).schnorrConstructSignature({
      message: hexToBytes(args.txCommitment),
      privateKey: hexToBytes(this.privateKey)
    })
    return {
      publicKeyX: account.publicKeyX,
      publicKeyY: account.publicKeyY,
      signature: bytesToHex(concatBytes([signature.s, signature.e]))
    }
  }

  async decryptSenderNote(args: {
    readonly privacyAccount: PrivacyAccount
    readonly txnData: TxnData
  }): Promise<Note | null> {
    return this.decodeNote(
      args.privacyAccount,
      args.txnData.senderEncryptedNote,
      args.txnData.userEncryptedKey
    )
  }

  async decryptRecipientNote(args: {
    readonly privacyAccount: PrivacyAccount
    readonly txnData: TxnData
  }): Promise<Note | null> {
    return this.decodeNote(
      args.privacyAccount,
      args.txnData.recipientEncryptedNote,
      args.txnData.recipientEncryptedKey
    )
  }

  async generateEphemeralKey(): Promise<EphemeralKeyPair> {
    const privateKey = await randomNonZeroB256()
    const publicKey = await publicKeyFromPrivateKey(privateKey)
    return {
      privateKey,
      privacyAddress: privacyAddressFromPublicKey(publicKey)
    }
  }

  private async decodeNote(
    privacyAccount: PrivacyAccount,
    encryptedNote: readonly [B256, B256, B256, B256, B256],
    encryptedKey: readonly [B256, B256, B256, B256]
  ): Promise<Note | null> {
    const account = await this.account()
    const address = privacyAccountAddress(privacyAccount)
    if (address.bytes !== account.address.bytes) {
      throw new PayyClientError(
        'privacy_account_mismatch',
        'privacy account mismatch'
      )
    }
    if (encryptedKey.every((word) => word === zeroHash())) {
      return null
    }
    const shared = await grumpkinScalarMulPoint(this.privateKey, {
      x: encryptedKey[0],
      y: encryptedKey[1]
    })
    const symmetricKey = fieldSub(
      b256ToBigInt(encryptedKey[2]),
      BigInt(await poseidonHash([BigInt(shared.x), BigInt(shared.y)]))
    )
    const fields = await Promise.all(
      encryptedNote.map(async (word, index) =>
        fieldSub(
          b256ToBigInt(word),
          BigInt(await poseidonHash([symmetricKey, BigInt(index)]))
        )
      )
    )
    const note = {
      kind: 1n,
      token: fields[0],
      nonce: fields[1],
      psi: fields[2],
      owner: fields[3],
      value: fields[4]
    }
    if ((await noteCommitment(note)) === zeroHash()) {
      return null
    }
    if (note.owner !== (await privacyAddressOwner(address))) {
      return null
    }
    return note
  }

  private async account(): Promise<{
    readonly address: PrivacyAddress
    readonly publicKeyX: B256
    readonly publicKeyY: B256
  }> {
    if (this.cachedAccount !== undefined) {
      return this.cachedAccount
    }
    const publicKey = await publicKeyFromPrivateKey(this.privateKey)
    this.cachedAccount = {
      address: privacyAddressFromPublicKey(publicKey),
      publicKeyX: publicKey.x,
      publicKeyY: publicKey.y
    }
    return this.cachedAccount
  }
}

export function createLocalPrivacySignerFromGrumpkinPrivateKey(
  grumpkinPrivateKey: B256
): PrivacySigner {
  return new LocalPrivacySigner(grumpkinPrivateKey)
}

export function createLocalPrivacySigner(evmPrivateKey: B256): PrivacySigner {
  return new LocalPrivacySigner(deriveGrumpkinPrivateKey(evmPrivateKey))
}

export function deriveGrumpkinPrivateKey(evmPrivateKey: B256): B256 {
  assertHex(evmPrivateKey, 32)
  const digest = keccak_256(
    concatBytes([DERIVATION_DOMAIN, hexToBytes(evmPrivateKey)])
  )
  const x = bytesToBigInt(digest)
  return bigIntToB256(1n + (x % (GRUMPKIN_SCALAR_MODULUS - 1n)))
}

function concatBytes(chunks: readonly Uint8Array[]): Uint8Array {
  const length = chunks.reduce((sum, chunk) => sum + chunk.length, 0)
  const out = new Uint8Array(length)
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.length
  }
  return out
}

function bytesToBigInt(bytes: Uint8Array): bigint {
  let value = 0n
  for (const byte of bytes) {
    value = (value << 8n) + BigInt(byte)
  }
  return value
}

function bigIntToB256(value: bigint): B256 {
  const bytes = new Uint8Array(32)
  let remaining = value
  for (let i = 31; i >= 0; i -= 1) {
    bytes[i] = Number(remaining & 255n)
    remaining >>= 8n
  }
  return bytesToHex(bytes)
}

function validateGrumpkinPrivateKey(privateKey: B256): void {
  assertHex(privateKey, 32)
  const scalar = BigInt(privateKey)
  if (scalar === 0n || scalar >= GRUMPKIN_SCALAR_MODULUS) {
    throw new PayyClientError(
      'field_out_of_range',
      'Grumpkin private key is out of range'
    )
  }
}

function zeroHash(): B256 {
  return `0x${'0'.repeat(64)}`
}
