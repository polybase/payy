const assert = require('node:assert/strict')
const test = require('node:test')

const {
  ethersProviderAdapter,
  ethersRawTransactionSubmitter,
  ethersSignerSubmitter,
  toEthersTransaction
} = require('../dist/ethers')

const CHAIN_ID = 7298
const FROM = '0x00000000000000000000000000000000000000aa'
const TO = '0x00000000000000000000000000000000000000bb'
const TOPIC0 = `0x${'11'.repeat(32)}`
const TOPIC1 = `0x${'22'.repeat(32)}`
const TX_HASH = `0x${'33'.repeat(32)}`
const EVM_PRIVATE_KEY = `0x${'11'.repeat(32)}`

function preparedCall(chainId = CHAIN_ID) {
  return {
    operation: 'mint',
    chainId,
    bridgeRequest: {
      from: FROM,
      to: TO,
      data: '0x1234',
      value: 7n,
      gasLimit: 42n
    },
    verificationKeyHash: '0x',
    proof: '0x',
    publicInputs: [],
    txCommitment: '0x',
    statePreview: {
      privacyAccount: { bytes: '0x' },
      token: TO,
      recentRoot: '0x',
      inputCommitments: [],
      inputNullifiers: [],
      outputCommitments: []
    }
  }
}

test('ethers provider adapter exposes read interface', async () => {
  const provider = { getNetwork: async () => ({ chainId: BigInt(CHAIN_ID) }) }

  assert.equal(await ethersProviderAdapter(provider).getChainId(), CHAIN_ID)
})

test('ethers provider adapter forwards historical read block', async () => {
  const calls = []
  const provider = {
    call: async (request, blockTag) => {
      calls.push({ request, blockTag })
      return '0x1234'
    }
  }

  const result = await ethersProviderAdapter(provider).readContract({
    to: TO,
    data: '0xabcd',
    blockNumber: 44n
  })

  assert.equal(result, '0x1234')
  assert.deepEqual(calls, [
    { request: { to: TO, data: '0xabcd' }, blockTag: 44n }
  ])
})

test('ethers provider adapter preserves positional topics', async () => {
  const calls = []
  const provider = {
    getLogs: async (filter) => {
      calls.push(filter)
      return [
        {
          address: TO,
          blockNumber: 3,
          transactionIndex: 4,
          index: 5,
          transactionHash: TX_HASH,
          topics: [TOPIC0, TOPIC1],
          data: '0xabcd'
        }
      ]
    }
  }

  const result = await ethersProviderAdapter(provider).getLogs({
    address: TO,
    fromBlock: 1n,
    toBlock: 2n,
    topics: [null, TOPIC1]
  })

  assert.deepEqual(calls[0].topics, [null, TOPIC1])
  assert.deepEqual(result[0], {
    address: TO,
    blockNumber: 3n,
    transactionIndex: 4,
    logIndex: 5,
    transactionHash: TX_HASH,
    topics: [TOPIC0, TOPIC1],
    data: '0xabcd'
  })
})

test('ethers provider adapter rejects missing log metadata', async () => {
  const provider = {
    getLogs: async () => [
      {
        address: TO,
        blockNumber: 3,
        transactionIndex: 4,
        transactionHash: TX_HASH,
        topics: [TOPIC0],
        data: '0xabcd'
      }
    ]
  }

  await assert.rejects(
    ethersProviderAdapter(provider).getLogs({
      address: TO,
      fromBlock: 1n,
      toBlock: 2n,
      topics: [TOPIC0]
    }),
    {
      code: 'missing_log_metadata',
      data: { field: 'logIndex' }
    }
  )
})

test('ethers provider adapter waits for requested confirmations', async () => {
  const receipt = {
    hash: '0x1234',
    blockNumber: 10,
    status: 1
  }
  const blockNumbers = [11, 12]
  const provider = {
    getBlockNumber: async () => blockNumbers.shift() ?? 12,
    getTransactionReceipt: async () => receipt
  }

  const result = await ethersProviderAdapter(
    provider
  ).waitForTransactionReceipt({
    hash: receipt.hash,
    confirmations: 3,
    timeoutMs: 100,
    pollIntervalMs: 1
  })

  assert.equal(result.transactionHash, receipt.hash)
  assert.equal(result.blockNumber, 10n)
  assert.equal(result.status, 'success')
  assert.equal(blockNumbers.length, 0)
})

test('ethers provider adapter reports receipt timeout as typed error', async () => {
  const provider = {
    getTransactionReceipt: async () => null
  }

  await assert.rejects(
    ethersProviderAdapter(provider).waitForTransactionReceipt({
      hash: TX_HASH,
      timeoutMs: 0,
      pollIntervalMs: 1
    }),
    {
      code: 'receipt_timeout',
      data: { hash: TX_HASH, timeoutMs: 0 }
    }
  )
})

test('ethers provider adapter rejects unknown receipt status', async () => {
  const provider = {
    getTransactionReceipt: async () => ({
      hash: TX_HASH,
      blockNumber: 10,
      status: null
    })
  }

  await assert.rejects(
    ethersProviderAdapter(provider).getTransactionReceipt(TX_HASH),
    {
      code: 'receipt_status_unknown',
      data: { hash: TX_HASH }
    }
  )
})

test('ethers submitters report sender mismatches as typed errors', async () => {
  await assert.rejects(
    ethersSignerSubmitter({
      getAddress: async () => FROM,
      sendTransaction: async () => ({ hash: TX_HASH })
    }).sendTransaction({
      from: '0x00000000000000000000000000000000000000cc',
      to: TO,
      data: '0x',
      value: 0n
    }),
    { code: 'evm_account_mismatch' }
  )

  await assert.rejects(
    ethersRawTransactionSubmitter({}).sendLocalTransaction(EVM_PRIVATE_KEY, {
      from: '0x00000000000000000000000000000000000000cc',
      to: TO,
      data: '0x',
      value: 0n
    }),
    { code: 'evm_account_mismatch' }
  )
})

test('ethers raw submitter rejects unavailable eip1559 fee data', async () => {
  const submitter = ethersRawTransactionSubmitter({
    getFeeData: async () => ({
      maxFeePerGas: null,
      maxPriorityFeePerGas: 1n
    })
  })

  await assert.rejects(submitter.getFeeData(), {
    code: 'fee_data_unavailable',
    data: { field: 'maxFeePerGas' }
  })

  await assert.rejects(
    submitter.sendLocalTransaction(EVM_PRIVATE_KEY, {
      to: TO,
      data: '0x',
      value: 0n
    }),
    {
      code: 'fee_data_unavailable',
      data: { field: 'maxFeePerGas' }
    }
  )
})

test('ethers raw submitter rejects unavailable priority fee data', async () => {
  const submitter = ethersRawTransactionSubmitter({
    getFeeData: async () => ({
      maxFeePerGas: 1n,
      maxPriorityFeePerGas: null
    })
  })

  await assert.rejects(submitter.getFeeData(), {
    code: 'fee_data_unavailable',
    data: { field: 'maxPriorityFeePerGas' }
  })
})

test('ethers raw submitter estimates gas with local sender', async () => {
  const estimateCalls = []
  const provider = {
    getNetwork: async () => ({ chainId: BigInt(CHAIN_ID) }),
    getFeeData: async () => ({
      maxFeePerGas: 2n,
      maxPriorityFeePerGas: 1n
    }),
    estimateGas: async (request) => {
      estimateCalls.push(request)
      return 21_000n
    },
    getTransactionCount: async () => 0,
    broadcastTransaction: async () => ({ hash: TX_HASH })
  }
  const submitter = ethersRawTransactionSubmitter(provider)
  const localAddress = await submitter.getLocalAddress(EVM_PRIVATE_KEY)

  await submitter.sendLocalTransaction(EVM_PRIVATE_KEY, {
    to: TO,
    data: '0x',
    value: 0n
  })

  assert.deepEqual(estimateCalls, [
    {
      to: TO,
      data: '0x',
      value: 0n,
      from: localAddress
    }
  ])
})

test('toEthersTransaction maps prepared payy calls to ethers requests', () => {
  const call = preparedCall()
  assert.deepEqual(toEthersTransaction(call), {
    chainId: CHAIN_ID,
    from: FROM,
    to: TO,
    data: '0x1234',
    value: 7n,
    gasLimit: 42n
  })

  assert.deepEqual(
    toEthersTransaction(
      { preparedCall: call, payload: null },
      { from: { address: FROM } }
    ),
    {
      chainId: CHAIN_ID,
      from: FROM,
      to: TO,
      data: '0x1234',
      value: 7n,
      gasLimit: 42n
    }
  )

  assert.equal(
    toEthersTransaction({
      result: { preparedCall: call, payload: null }
    }).to,
    TO
  )
})

test('toEthersTransaction rejects chain and sender mismatches', () => {
  const call = preparedCall()
  assert.throws(
    () => toEthersTransaction(call, { chainId: CHAIN_ID + 1 }),
    /chain id mismatch/
  )
  assert.throws(
    () =>
      toEthersTransaction(call, {
        from: '0x00000000000000000000000000000000000000cc'
      }),
    /from does not match/
  )
})
