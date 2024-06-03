import { type HardhatUserConfig } from 'hardhat/config'
import '@nomicfoundation/hardhat-toolbox-viem'
import '@nomicfoundation/hardhat-ethers'

const config: HardhatUserConfig = {
  solidity: {
    version: '0.8.20',
    settings: {
      viaIR: true,
      optimizer: {
        enabled: true,
        runs: 200
      }
    }
  },
  networks: {
    hardhat: {
      chainId: 1337,
      throwOnTransactionFailures: true,
      throwOnCallFailures: true,
      loggingEnabled: true
    }
  }
}

const NETWORKS = {
  // A hardhat testing instance with a non-default port
  testing: { chainId: 1337 },
  mainnet: { chainId: 1 },
  ropsten: { chainId: 3 },
  rinkeby: { chainId: 4 },
  goerli: { chainId: 5 },
  polygon: { chainId: 137 },
  amoy: { chainId: 80002 },
  kovan: { chainId: 42 },
  sepolia: { chainId: 1337 }
} as any

Object.keys(NETWORKS).forEach((network) => {
  const networkUrl = process.env[`${network.toUpperCase()}_URL`]
  if (networkUrl === undefined || config?.networks === undefined) return

  // Ensure we have a secret key for this network
  if (process.env.SECRET_KEY === undefined) throw new Error('SECRET_KEY is not set')

  config.networks[network] = {
    url: networkUrl,
    accounts: [process.env.SECRET_KEY ?? ''],
    chainId: NETWORKS[network]?.chainId,
    gasPrice: process.env.GAS_PRICE_GWEI !== undefined ? parseInt(process.env.GAS_PRICE_GWEI) * 1_000_000_000 : undefined
  }
})

console.error(config)

export default config
