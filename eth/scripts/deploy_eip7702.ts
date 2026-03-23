import hre from "hardhat";

// Deploy only the EIP-7702 delegate contract (Eip7702SimpleAccount)
// Usage:
//   hardhat run scripts/deploy_eip7702.ts --network <network>

async function main(): Promise<void> {
  const contract = await hre.viem.deployContract(
    "contracts/Eip7702SimpleAccount.sol:Eip7702SimpleAccount",
  );
  console.log(`EIP7702_SIMPLE_ACCOUNT_ADDR=${contract.address}`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

