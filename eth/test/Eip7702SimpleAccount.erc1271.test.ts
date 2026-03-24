import { expect } from 'chai'
import hre from 'hardhat'
import { secp256k1 } from '@noble/curves/secp256k1'
import { keccak256, hexToBytes, type Hex } from 'viem'
import { privateKeyToAccount } from 'viem/accounts'

const OWNER_PRIVATE_KEY = '0x59c6995e998f97a5a0044974fee25b4152cf92f5b1eb2d6feab718a6b7f7d9c7'
const OTHER_PRIVATE_KEY = '0x8b3a350cf5c34c9194ca10060b7a0fba8f5a2f05d5d3f5c5cbd3e55e3f66b2b1'

const ownerAccount = privateKeyToAccount(OWNER_PRIVATE_KEY)

const digestMagic = '0x1626ba7e'
const bytesMagic = '0x20c13b0b'
const invalidMagic = '0xffffffff'

const digestSignatureAbi = [
  {
    inputs: [
      { internalType: 'bytes32', name: 'hash', type: 'bytes32' },
      { internalType: 'bytes', name: 'signature', type: 'bytes' }
    ],
    name: 'isValidSignature',
    outputs: [{ internalType: 'bytes4', name: '', type: 'bytes4' }],
    stateMutability: 'view',
    type: 'function'
  }
] as const

const bytesSignatureAbi = [
  {
    inputs: [
      { internalType: 'bytes', name: 'data', type: 'bytes' },
      { internalType: 'bytes', name: 'signature', type: 'bytes' }
    ],
    name: 'isValidSignature',
    outputs: [{ internalType: 'bytes4', name: '', type: 'bytes4' }],
    stateMutability: 'view',
    type: 'function'
  }
] as const

function signDigest(hash: Hex, privateKey: Hex): `0x${string}` {
  const signature = secp256k1.sign(hexToBytes(hash), hexToBytes(privateKey))
  const r = signature.r.toString(16).padStart(64, '0')
  const s = signature.s.toString(16).padStart(64, '0')
  const v = (signature.recovery + 27).toString(16).padStart(2, '0')
  return `0x${r}${s}${v}`
}

describe('Eip7702SimpleAccount ERC1271', () => {
  let bytecode: Hex

  before(async () => {
    const artifact = await hre.artifacts.readArtifact('Eip7702SimpleAccount')
    bytecode = artifact.deployedBytecode as Hex
  })

  beforeEach(async () => {
    const testClient = await hre.viem.getTestClient()
    await testClient.setCode({ address: ownerAccount.address, bytecode })
  })

  it('returns the ERC-1271 magic value for digest signatures signed by the contract owner', async () => {
    const digest = keccak256('0xdeadbeef')
    const signature = signDigest(digest, OWNER_PRIVATE_KEY)
    const publicClient = await hre.viem.getPublicClient()

    const result = await publicClient.readContract({
      address: ownerAccount.address,
      abi: digestSignatureAbi,
      functionName: 'isValidSignature',
      args: [digest, signature]
    })

    expect(result).to.equal(digestMagic)
  })

  it('rejects digest signatures that were not produced by the contract owner', async () => {
    const digest = keccak256('0x1337')
    const signature = signDigest(digest, OTHER_PRIVATE_KEY)
    const publicClient = await hre.viem.getPublicClient()

    const result = await publicClient.readContract({
      address: ownerAccount.address,
      abi: digestSignatureAbi,
      functionName: 'isValidSignature',
      args: [digest, signature]
    })

    expect(result).to.equal(invalidMagic)
  })

  it('returns the legacy bytes magic value when signing raw calldata with the owner key', async () => {
    const data = '0xc0ffee'
    const digest = keccak256(data)
    const signature = signDigest(digest, OWNER_PRIVATE_KEY)
    const publicClient = await hre.viem.getPublicClient()

    const result = await publicClient.readContract({
      address: ownerAccount.address,
      abi: bytesSignatureAbi,
      functionName: 'isValidSignature',
      args: [data, signature]
    })

    expect(result).to.equal(bytesMagic)
  })

  it('rejects bytes signatures signed by any other key', async () => {
    const data = '0x1234'
    const digest = keccak256(data)
    const signature = signDigest(digest, OTHER_PRIVATE_KEY)
    const publicClient = await hre.viem.getPublicClient()

    const result = await publicClient.readContract({
      address: ownerAccount.address,
      abi: bytesSignatureAbi,
      functionName: 'isValidSignature',
      args: [data, signature]
    })

    expect(result).to.equal(invalidMagic)
  })
})
