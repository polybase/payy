import { defineChain } from 'viem'

export const PAYY_TESTNET_CHAIN_ID = 7298
export const RPC_URL = 'https://rpc.testnet.payy.network'
export const EXPLORER_URL = 'https://blockscout.testnet.payy.network'
export const FAUCET_URL = 'https://faucet.testnet.payy.network'
export const IS_TESTNET = true

export const payyTestnet = defineChain({
  id: PAYY_TESTNET_CHAIN_ID,
  name: 'Payy Testnet',
  nativeCurrency: {
    name: 'PUSD',
    symbol: 'PUSD',
    decimals: 16
  },
  rpcUrls: {
    default: {
      http: [RPC_URL]
    }
  },
  blockExplorers: {
    default: { name: 'Payy Blockscout', url: EXPLORER_URL }
  }
})

export const buildExplorerTxUrl = (hash: string) => `${EXPLORER_URL}/tx/${hash}`
