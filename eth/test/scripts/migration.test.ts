/* eslint-disable @typescript-eslint/no-unused-expressions */
import { loadFixture } from '@nomicfoundation/hardhat-toolbox-viem/network-helpers'
import { expect } from 'chai'
import fs from 'fs'
import hre from 'hardhat'
import path from 'path'
import { keccak256, toHex, zeroHash } from 'viem'
import { extractMints } from '../../scripts/extract-mints'
import { filterMints } from '../../scripts/filter-mints'
import { loadMints } from '../../scripts/lib/migration-utils'
import { submitMints } from '../../scripts/submit-mints'

describe('Migration Scripts', function () {
  const OUTPUT_FILE = path.join(__dirname, 'temp-mints.json')
  const FILTERED_FILE = path.join(__dirname, 'filtered-mints.json')

  afterEach(() => {
    if (fs.existsSync(OUTPUT_FILE)) {
      fs.unlinkSync(OUTPUT_FILE)
    }
    if (fs.existsSync(FILTERED_FILE)) {
      fs.unlinkSync(FILTERED_FILE)
    }
  })

  async function deployFixture() {
    const [owner] = await hre.viem.getWalletClients()
    const publicClient = await hre.viem.getPublicClient()
    const source = await hre.viem.deployContract('RollupMigrationMock')
    const target = await hre.viem.deployContract('RollupMigrationMock')
    return { source, target, owner, publicClient }
  }

  it('should extract, filter, and submit mints correctly', async function () {
    const { source, target, owner, publicClient } = await loadFixture(deployFixture)

    const mint1 = { hash: keccak256(toHex('mint1')), value: 100n, kind: toHex('kind1', { size: 32 }) }
    const mint2 = { hash: keccak256(toHex('mint2')), value: 200n, kind: toHex('kind2', { size: 32 }) }
    const mint3 = { hash: keccak256(toHex('mint3')), value: 300n, kind: toHex('kind3', { size: 32 }) }
    const mint4 = { hash: keccak256(toHex('mint4')), value: 400n, kind: toHex('kind4', { size: 32 }) }

    // 1. Setup Source State (Extraction Phase)
    // Mint 1: Valid
    const h1 = await source.write.emitMintAdded([mint1.hash, mint1.value, mint1.kind])
    await publicClient.waitForTransactionReceipt({ hash: h1 })

    // Mint 2: Spent
    const h2 = await source.write.emitMintAdded([mint2.hash, mint2.value, mint2.kind])
    await publicClient.waitForTransactionReceipt({ hash: h2 })
    const h3 = await source.write.emitMinted([mint2.hash, zeroHash, zeroHash])
    await publicClient.waitForTransactionReceipt({ hash: h3 })

    // Mint 3: Valid
    const h4 = await source.write.emitMintAdded([mint3.hash, mint3.value, mint3.kind])
    await publicClient.waitForTransactionReceipt({ hash: h4 })

    // Mint 4: Valid (at extraction time)
    const h5 = await source.write.emitMintAdded([mint4.hash, mint4.value, mint4.kind])
    await publicClient.waitForTransactionReceipt({ hash: h5 })

    // 2. Extract
    await extractMints({
      sourceRpcUrl: 'http://localhost:8545',
      sourceAddress: source.address,
      outputFile: OUTPUT_FILE,
      startBlock: 0n,
      concurrency: 1,
      blockBatchSize: 100n,
      client: publicClient
    })

    const extracted = await loadMints(OUTPUT_FILE)
    expect(extracted[mint1.hash].spent).to.equal(false)
    expect(extracted[mint2.hash].spent).to.equal(true)
    expect(extracted[mint3.hash].spent).to.equal(false)
    expect(extracted[mint4.hash].spent).to.equal(false)

    // 3. Change State (Between Extract and Filter)
    // Spend Mint 4 on chain
    const h6 = await source.write.emitMinted([mint4.hash, zeroHash, zeroHash])
    await publicClient.waitForTransactionReceipt({ hash: h6 })

    // 4. Filter
    await filterMints({
      sourceRpcUrl: 'http://localhost:8545',
      sourceAddress: source.address,
      inputFile: OUTPUT_FILE,
      outputFile: FILTERED_FILE,
      client: publicClient
    })

    const filtered = await loadMints(FILTERED_FILE)
    // Mint 1: kept
    expect(filtered[mint1.hash]).to.not.be.undefined
    // Mint 2: removed (spent in extract)
    expect(filtered[mint2.hash]).to.be.undefined
    // Mint 3: kept
    expect(filtered[mint3.hash]).to.not.be.undefined
    // Mint 4: removed (spent on chain after extract)
    expect(filtered[mint4.hash]).to.be.undefined

    // 5. Submit
    await submitMints({
      targetRpcUrl: 'http://localhost:8545',
      targetAddress: target.address,
      privateKey: '0x0000000000000000000000000000000000000000000000000000000000000001',
      inputFile: FILTERED_FILE,
      publicClient,
      walletClient: owner
    })

    // 6. Verify Target
    const logs = await publicClient.getLogs({
      address: target.address,
      event: {
        type: 'event',
        name: 'MintAdded',
        inputs: [
          { type: 'bytes32', name: 'mint_hash', indexed: true },
          { type: 'uint256', name: 'value' },
          { type: 'bytes32', name: 'note_kind' }
        ]
      },
      fromBlock: 'earliest'
    })

    expect(logs.length).to.equal(2)
    const hashes = logs.map(l => l.args.mint_hash)
    expect(hashes).to.include(mint1.hash)
    expect(hashes).to.include(mint3.hash)
    expect(hashes).to.not.include(mint2.hash)
    expect(hashes).to.not.include(mint4.hash)

    // 7. Test Print Mode
    const consoleLogs: string[] = []
    const originalLog = console.log
    console.log = (...args) => consoleLogs.push(args.join(' '))

    // We need new candidates to test printTx effectively.
    // Add Mint 5 to file manually.
    const mint5 = { hash: keccak256(toHex('mint5')), value: '500', noteKind: toHex('kind5', { size: 32 }), spent: false, mintHash: keccak256(toHex('mint5')) }
    const inputMints = await loadMints(FILTERED_FILE)
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    inputMints[mint5.hash] = mint5 as any
    await fs.promises.writeFile(FILTERED_FILE, JSON.stringify(inputMints))

    try {
      await submitMints({
        targetRpcUrl: 'http://localhost:8545',
        targetAddress: target.address,
        privateKey: '0x0000000000000000000000000000000000000000000000000000000000000001',
        inputFile: FILTERED_FILE,
        publicClient,
        walletClient: owner,
        printTx: true
      })
    } finally {
      console.log = originalLog
    }

    const dataLogs = consoleLogs.filter(l => l.includes('Data: 0x'))
    expect(dataLogs.length).to.be.greaterThan(0)
    expect(dataLogs[0]).to.include('Data: 0x') // Ensure it printed hex data
  })
})
