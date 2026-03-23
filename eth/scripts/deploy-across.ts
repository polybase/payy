import hre from "hardhat";

const ACROSS_ADRESSES: Record<string, string> = {
  // Ethereum Mainnet
  1: "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5",
  // Base
  8453: "0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64",
  // Polygon
  137: "0x9295ee1d8c5b022be115a2ad3c30c72e34e7f096",
  // BNB
  56: "0x4e8E101924eDE233C13e2D8622DC8aED2872d505",
  // Arbitrum
  42161: "0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A",
  // Optimism
  10: "0x6f26bf09b1c792e3228e5467807a900a503c0281",
};

async function main(): Promise<void> {
  const chainId = hre.network.config.chainId ?? "DEV";

  let acrossAddress = process.env.ACROSS_ADDRESS as `0x${string}` | undefined;
  let owner = process.env.OWNER as `0x${string}` | undefined;

  if (acrossAddress === undefined) {
    acrossAddress = ACROSS_ADRESSES[chainId];
  }

  if (!acrossAddress && (chainId === "DEV" || chainId === 1337)) {
    console.log(
      "Deploying mock AcrossWithAuthorization for local development...",
    );
    owner = owner ?? "0x0000000000000000000000000000000000000000";
    // A simple mock contract deployment for local testing
    const mockAcross = await hre.viem.deployContract(
      "AcrossWithAuthorization",
      ["0x0000000000000000000000000000000000000000", owner],
    );
    acrossAddress = mockAcross.address;
    console.log(`Mock AcrossWithAuthorization deployed at: ${acrossAddress}`);
  }

  if (!acrossAddress) {
    throw new Error(
      `ACROSS_ADDRESS address not found for chainId ${chainId}. Please set ACROSS_ADDRESS environment variable.`,
    );
  }

  if (owner === undefined) {
    throw new Error(
      "OWNER environment variable is not set. Please set it to the owner address.",
    );
  }

  console.log(
    `Deploying AcrossWithAuthorization with across address: ${acrossAddress}`,
  );

  const acrossWithAuth = await hre.viem.deployContract(
    "AcrossWithAuthorization",
    [acrossAddress, owner],
  );

  console.log(
    `ACROSS_WITH_AUTHORIZATION_CONTRACT_ADDR=${acrossWithAuth.address}`,
  );
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
