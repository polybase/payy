import hre from "hardhat";

async function main(): Promise<void> {
  const [owner] = await hre.viem.getWalletClients();

  const socialRecovery = await hre.viem.deployContract(
    "contracts/SocialRecovery.sol:SocialRecovery",
    [owner.account.address],
  );

  console.log(
    `SOCIAL_RECOVERY_CONTRACT_ADDRESS=${socialRecovery.address}`,
  );
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});


