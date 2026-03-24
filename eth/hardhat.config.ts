import { type HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox-viem";
import "@nomicfoundation/hardhat-ethers";
import "@nomicfoundation/hardhat-chai-matchers";

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.20",
    settings: {
      viaIR: true,
      optimizer: {
        enabled: true,
        runs: 200,
      },
    },
  },
  networks: {
    hardhat: {
      chainId: 1337,
      hardfork: "prague",
      throwOnTransactionFailures: true,
      throwOnCallFailures: true,
      loggingEnabled: true,
      mining: {
        auto: true,
        interval: 1000,
      },
    },
    localhost: {
      // url: 'http://localhost:8546',
      chainId: 1337,
    },
  },
};

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
  sepolia: { chainId: 1337 },
  base: { chainId: 8453 },
  bnb: { chainId: 56 },
  arbitrum: { chainId: 42161 },
  optimism: { chainId: 10 },
} as any;

Object.keys(NETWORKS).forEach((network) => {
  const networkUrl = process.env[`${network.toUpperCase()}_URL`];
  if (networkUrl === undefined || config?.networks === undefined) return;

  // Ensure we have a secret key for this network
  if (process.env.SECRET_KEY === undefined)
    throw new Error("SECRET_KEY is not set");

  config.networks[network] = {
    url: networkUrl,
    accounts: [process.env.SECRET_KEY ?? ""],
    chainId: NETWORKS[network]?.chainId,
    gasPrice:
      process.env.GAS_PRICE_GWEI !== undefined
        ? parseInt(process.env.GAS_PRICE_GWEI) * 1_000_000_000
        : undefined,
  };
});

console.error(config);

export default config;
