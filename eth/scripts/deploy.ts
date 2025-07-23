import { readFile } from "fs/promises";
import { join } from "path";
import hre from "hardhat";
import { encodeFunctionData } from "viem";
import { deployBin } from "./shared";

// Auto-updated by generate_fixtures.sh - do not modify manually
const AGG_UTXO_VERIFICATION_KEY_HASH = "0x1970936764715b9d95ffe7837f51a97832ef71821b8a042391ddf4f28b3246ca";

const USDC_ADDRESSES: Record<string, string> = {
  // Ethereum Mainnet
  1: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
  // Ethereum Goerli Testnet
  // 5: '0x07865c6e87b9f70255377e024ace6630c1eaa37f',
  // Polygon Mainnet
  137: "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
  // Polygon Mumbai Testnet
  // 80001: '0x2058A9D7613eEE744279e3856Ef0eAda5FCbaA7e'
};

async function main(): Promise<void> {
  const chainId = hre.network.config.chainId ?? "DEV";
  const useNoopVerifier = process.env.DEV_USE_NOOP_VERIFIER === "1";
  const [owner] = await hre.viem.getWalletClients();
  const publicClient = await hre.viem.getPublicClient();

  let usdcAddress: string;
  let isDev = false;

  // Create a local version of USDC for testing
  if (USDC_ADDRESSES[chainId] === undefined) {
    const usdcContractAddr = await deployBin("USDC.bin");
    console.log(`USDC_CONTRACT_ADDR=${usdcContractAddr}`);
    usdcAddress = usdcContractAddr;
    isDev = true;
  } else {
    usdcAddress = USDC_ADDRESSES[chainId];
  }

  let acrossSpokePool = process.env.ACROSS_SPOKE_POOL as
    | `0x${string}`
    | undefined;
  if (acrossSpokePool !== undefined && !acrossSpokePool.startsWith("0x")) {
    throw new Error("ACROSS_SPOKE_POOL is not a valid address");
  }

  if (!isDev && useNoopVerifier) {
    throw new Error("Cannot use no-op verifier if not deploying for dev");
  } else if (useNoopVerifier) {
    console.warn("Warning: using no-op verifier");
  }

  const maybeNoopVerifier = (verifier: string) =>
    useNoopVerifier ? "NoopVerifierHonk.bin" : verifier;

  let proverAddress = process.env.PROVER_ADDRESS as `0x${string}`;
  let validators =
    process.env.VALIDATORS?.split(",") ?? ([] as Array<`0x${string}`>);
  let ownerAddress = process.env.OWNER as `0x${string}`;
  if (!isDev) {
    if (proverAddress === undefined)
      throw new Error("PROVER_ADDRESS is not set");
    if (validators.length === 0) throw new Error("VALIDATORS is not set");
    if (ownerAddress === undefined) throw new Error("OWNER is not set");
  } else {
    if (proverAddress === undefined) {
      proverAddress = owner.account.address;
    }

    if (validators.length === 0) {
      validators = [owner.account.address];
    }

    if (ownerAddress === undefined) {
      ownerAddress = owner.account.address;
    }
  }
  const deployerIsProxyAdmin =
    ownerAddress.toLowerCase() === owner.account.address.toLowerCase();

  console.error({
    proverAddress,
    validators,
    ownerAddress,
    deployerIsProxyAdmin,
  });

  const emptyMerkleTreeRootHash =
    "0x" +
    (
      await readFile(
        join(
          __dirname,
          "../../pkg/contracts/src/empty_merkle_tree_root_hash.txt",
        ),
      )
    )
      .toString()
      .trimEnd();

  const aggregateVerifierAddr = await deployBin(
    maybeNoopVerifier("noir/agg_agg_HonkVerifier.bin"),
  );
  console.log(`AGGREGATE_VERIFIER_ADDR=${aggregateVerifierAddr}`);

  const mintVerifierAddr = await deployBin("noir/mint_HonkVerifier.bin");
  console.log(`MINT_VERIFIER_ADDR=${mintVerifierAddr}`);

  const rollupV1 = await hre.viem.deployContract(
    "contracts/rollup2/RollupV1.sol:RollupV1",
  );
  console.log(`ROLLUP_V1_IMPL_ADDR=${rollupV1.address}`);

  const rollupInitializeCalldata = encodeFunctionData({
    abi: rollupV1.abi,
    functionName: "initialize",
    args: [
      ownerAddress,
      usdcAddress,
      aggregateVerifierAddr,
      mintVerifierAddr,
      proverAddress,
      validators,
      emptyMerkleTreeRootHash,
      useNoopVerifier
        ? "0x0000000000000000000000000000000000000000000000000000000000000000"
        : AGG_UTXO_VERIFICATION_KEY_HASH,
    ],
  });

  const rollupProxy = await hre.viem.deployContract(
    "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy",
    [rollupV1.address, ownerAddress, rollupInitializeCalldata],
    {},
  );

  console.log(`ROLLUP_CONTRACT_ADDR=${rollupProxy.address}`);

  const eip1967AdminStorageSlot =
    "0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103";
  let admin = await publicClient.getStorageAt({
    address: rollupProxy.address,
    slot: eip1967AdminStorageSlot,
  });
  admin = `0x${admin?.slice(2 + 12 * 2)}`;

  console.log(`ROLLUP_PROXY_ADMIN_ADDR=${admin}`);

  const proxyAdmin = await hre.viem.getContractAt(
    "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin",
    admin,
  );

  const [signerOwner] = await hre.ethers.getSigners();
  const usdc = await hre.ethers.getContractAt(
    "IUSDC",
    usdcAddress,
    signerOwner,
  );

  if (isDev) {
    if (owner.chain.name === "hardhat") {
      await owner.sendTransaction({
        to: proverAddress,
        value: hre.ethers.parseEther("1"),
      });
    }

    let res = await usdc.initialize(
      "USD Coin",
      "USDC",
      "USD",
      6,
      signerOwner.address,
      signerOwner.address,
      signerOwner.address,
      signerOwner.address,
      {
        gasLimit: 1_000_000,
      },
    );
    await res.wait();
    res = await usdc.initializeV2("USD Coin", {
      gasLimit: 1_000_000,
    });
    await res.wait();
    res = await usdc.initializeV2_1(signerOwner.address, {
      gasLimit: 1_000_000,
    });
    await res.wait();
    res = await usdc.configureMinter(
      signerOwner.address,
      hre.ethers.parseUnits("1000000000", 6),
      {
        gasLimit: 1_000_000,
      },
    );
    await res.wait();

    res = await usdc.mint(
      signerOwner.address,
      hre.ethers.parseUnits("1000000000", 6),
      {
        gasLimit: 1_000_000,
      },
    );
    await res.wait();
  }

  // Approve our rollup contract to spend USDC from the primary owner account
  const res = await usdc.approve(rollupProxy.address, hre.ethers.MaxUint256, {
    gasLimit: 1_000_000,
  });
  await res.wait();

  console.error("All contracts deployed");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
