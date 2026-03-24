import hre from "hardhat";
import { deployBin } from "./shared";

const CHAINS = ["BASE", "ARB", "OPT", "ETH"];

async function main(): Promise<void> {
  const chainId = hre.network.config.chainId ?? "DEV";
  const isDev = chainId === "DEV" || chainId === 1337;

  if (!isDev) {
    throw new Error("ONLY use in DEV");
  }

  for (const chain of CHAINS) {
    const usdcContractAddr = await deployBin("USDC.bin");
    console.log(`${chain}_USDC_CONTRACT_ADDR=${usdcContractAddr}`);
  }

  console.error("All contracts deployed");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
