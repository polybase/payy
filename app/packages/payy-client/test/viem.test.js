const assert = require('node:assert/strict')
const test = require('node:test')

const {
  chains,
  toViemTransaction,
  viemPublicClientAdapter,
  viemRawTransactionSubmitter
} = require('../dist/viem')

const FROM = '0x00000000000000000000000000000000000000aa'
const TO = '0x00000000000000000000000000000000000000bb'
const TOPIC0 = `0x${'11'.repeat(32)}`
const TOPIC1 = `0x${'22'.repeat(32)}`
const TX_HASH = `0x${'33'.repeat(32)}`
const EVM_PRIVATE_KEY = `0x${'11'.repeat(32)}`

function preparedCall(chainId = chains.payy.testnet.id) {
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

test('viem chains expose payy network metadata', () => {
  assert.equal(chains.payy.dev.id, 7297)
  assert.equal(chains.payy.dev.name, 'Payy Dev')
  assert.deepEqual(chains.payy.dev.nativeCurrency, {
    name: 'PUSD',
    symbol: 'PUSD',
    decimals: 16
  })
  assert.deepEqual(chains.payy.dev.rpcUrls.default.http, [
    'http://127.0.0.1:8546'
  ])

  assert.equal(chains.payy.testnet.id, 7298)
  assert.equal(chains.payy.testnet.name, 'Payy Testnet')
  assert.deepEqual(chains.payy.testnet.nativeCurrency, {
    name: 'PUSD',
    symbol: 'PUSD',
    decimals: 16
  })
  assert.deepEqual(chains.payy.testnet.rpcUrls.default.http, [
    'https://rpc.testnet.payy.network'
  ])
  assert.deepEqual(chains.payy.testnet.blockExplorers?.default, {
    name: 'Payy Blockscout',
    url: 'https://blockscout.testnet.payy.network'
  })
})

test('viem public client adapter exposes read interface', async () => {
  const publicClient = { getChainId: async () => chains.payy.testnet.id }

  assert.equal(
    await viemPublicClientAdapter(publicClient).getChainId(),
    chains.payy.testnet.id
  )
})

test('viem public client adapter forwards historical read block', async () => {
  const calls = []
  const publicClient = {
    call: async (request) => {
      calls.push(request)
      return { data: '0x1234' }
    }
  }

  const result = await viemPublicClientAdapter(publicClient).readContract({
    to: TO,
    data: '0xabcd',
    blockNumber: 44n
  })

  assert.equal(result, '0x1234')
  assert.deepEqual(calls, [{ to: TO, data: '0xabcd', blockNumber: 44n }])
})

test('viem public client adapter preserves positional topics', async () => {
  const calls = []
  const log = {
    address: TO,
    blockNumber: 3n,
    transactionIndex: 4,
    logIndex: 5,
    transactionHash: TX_HASH,
    topics: [TOPIC0, TOPIC1],
    data: '0xabcd'
  }
  const publicClient = {
    getLogs: async (filter) => {
      calls.push(filter)
      return [log]
    }
  }

  const result = await viemPublicClientAdapter(publicClient).getLogs({
    address: TO,
    fromBlock: 1n,
    toBlock: 2n,
    topics: [null, TOPIC1]
  })

  assert.deepEqual(calls[0].topics, [null, TOPIC1])
  assert.deepEqual(result, [log])
})

test('viem public client adapter rejects missing log metadata', async () => {
  const publicClient = {
    getLogs: async () => [
      {
        blockNumber: 3n,
        transactionIndex: 4,
        logIndex: 5,
        transactionHash: TX_HASH,
        topics: [TOPIC0],
        data: '0xabcd'
      }
    ]
  }

  await assert.rejects(
    viemPublicClientAdapter(publicClient).getLogs({
      address: TO,
      fromBlock: 1n,
      toBlock: 2n,
      topics: [TOPIC0]
    }),
    {
      code: 'missing_log_metadata',
      data: { field: 'address' }
    }
  )
})

test('viem raw submitter reports sender mismatch as typed error', async () => {
  await assert.rejects(
    viemRawTransactionSubmitter({}).sendLocalTransaction(EVM_PRIVATE_KEY, {
      from: '0x00000000000000000000000000000000000000cc',
      to: TO,
      data: '0x',
      value: 0n
    }),
    { code: 'evm_account_mismatch' }
  )
})

test('toViemTransaction maps prepared payy calls to viem requests', () => {
  const call = preparedCall()
  assert.deepEqual(toViemTransaction(call), {
    account: FROM,
    to: TO,
    data: '0x1234',
    value: 7n,
    gas: 42n
  })

  assert.deepEqual(toViemTransaction(call, { chain: chains.payy.testnet }), {
    account: FROM,
    chain: chains.payy.testnet,
    to: TO,
    data: '0x1234',
    value: 7n,
    gas: 42n
  })

  assert.deepEqual(
    toViemTransaction(
      { preparedCall: call, payload: null },
      { account: { address: FROM }, chain: chains.payy.testnet }
    ),
    {
      account: { address: FROM },
      chain: chains.payy.testnet,
      to: TO,
      data: '0x1234',
      value: 7n,
      gas: 42n
    }
  )

  assert.equal(
    toViemTransaction(
      { result: { preparedCall: call, payload: null } },
      { chain: chains.payy.testnet }
    ).to,
    TO
  )
})

test('toViemTransaction rejects chain and account mismatches', () => {
  const call = preparedCall()
  assert.throws(
    () => toViemTransaction(call, { chain: chains.payy.dev }),
    /chain id mismatch/
  )
  assert.throws(
    () =>
      toViemTransaction(call, {
        account: '0x00000000000000000000000000000000000000cc',
        chain: chains.payy.testnet
      }),
    /account does not match/
  )
})
