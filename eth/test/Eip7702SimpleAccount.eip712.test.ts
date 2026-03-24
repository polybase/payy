import { expect } from 'chai'
import hre from 'hardhat'
import fs from 'node:fs'
import path from 'node:path'

describe('Eip712 array encoding stays in sync', () => {
  it('hashCalls(Call[]) matches the Rust implementation', async () => {
    const contract = await hre.viem.deployContract('Eip7702SimpleAccount')

    const calls = [
      {
        target: '0x1111111111111111111111111111111111111111',
        value: 1n,
        data: '0x'
      },
      {
        target: '0x2222222222222222222222222222222222222222',
        value: 2n,
        data: '0xdeadbeef'
      }
    ] as const

    const onchain = await contract.read.hashCalls([calls])
    const expected = fs
      .readFileSync(path.join(__dirname, '..', '..', 'fixtures', 'eip7702', 'hashcalls_vector1.hex'), 'utf8')
      .trim()

    expect(onchain.toLowerCase()).to.eq(expected)
  })
})
