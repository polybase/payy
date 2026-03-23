import { readFile } from "fs/promises";
import hre from "hardhat";
import { encodeFunctionData, keccak256, stringToBytes } from "viem";
import { deployBin, deployBytecode } from "./shared";
import {
  LEGACY_POLYGON_CHAIN_ID,
  POLYGON_USDC_ADDRESS,
  generateNoteKindBridgeEvm,
} from "./noteKind";

type LinkReferences = Record<
  string,
  Record<string, Array<{ length: number; start: number }>>
>;

// Auto-updated by generate_fixturecs.sh - do not modify manually
const AGG_FINAL_VERIFICATION_KEY_HASH =
  "0x122d2ac7542fa020cbfff0836b5d0c30898330074b19869179bba49b5db69967";

const USDC_ADDRESSES: Record<number | string, `0x${string}`> = {
  // Ethereum Mainnet
  1: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
  // Ethereum Goerli Testnet
  // 5: '0x07865c6e87b9f70255377e024ace6630c1eaa37f',
  // Polygon Mainnet
  137: POLYGON_USDC_ADDRESS,
  // Polygon Mumbai Testnet
  // 80001: '0x2058A9D7613eEE744279e3856Ef0eAda5FCbaA7e'
};

const DEFAULT_VERIFIER_MESSAGES_COUNT = 1000;
// Legacy note kind used across all deployments for backwards compatibility.
const LEGACY_POLYGON_USDC_NOTE_KIND = generateNoteKindBridgeEvm(
  LEGACY_POLYGON_CHAIN_ID,
  POLYGON_USDC_ADDRESS,
);

const HONK_VERIFIER_SUFFIX = "_HonkVerifier.bin";

const computePlaceholder = (fullyQualifiedLibName: string): string => {
  const hash = keccak256(stringToBytes(fullyQualifiedLibName)).slice(2);
  return `__$${hash.slice(0, 34)}$__`;
};

// Note: this is only used when deploying with the no-op verifier,
// because we need to pad out the libraries
// that would otherwise get deployed,
// to get stable addresses.
const deployLibrariesForProgram = async (binFile: string): Promise<void> => {
  const linkRefPath = `contracts/${binFile.replace(/\.bin$/, ".linkrefs.json")}`;

  let linkRefsRaw: string;
  try {
    linkRefsRaw = await readFile(linkRefPath, "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      return;
    }
    throw error;
  }

  const linkRefs = JSON.parse(linkRefsRaw) as LinkReferences;
  const hasLibraries = Object.values(linkRefs).some(
    (libraries) => Object.keys(libraries).length > 0,
  );
  if (!hasLibraries) return;

  const programDir =
    binFile.lastIndexOf("/") >= 0
      ? binFile.slice(0, binFile.lastIndexOf("/"))
      : "";
  const programFilename = binFile.slice(binFile.lastIndexOf("/") + 1);
  if (!programFilename.endsWith(HONK_VERIFIER_SUFFIX)) {
    throw new Error(
      `Cannot infer program name from ${binFile}; expected suffix ${HONK_VERIFIER_SUFFIX}`,
    );
  }

  const programName = programFilename.slice(0, -HONK_VERIFIER_SUFFIX.length);
  const prefix = programDir.length > 0 ? `${programDir}/` : "";

  for (const libraries of Object.values(linkRefs)) {
    for (const libraryName of Object.keys(libraries)) {
      const libraryBinFile = `${prefix}${programName}_${libraryName}.bin`;
      await deployBin(libraryBinFile);
    }
  }
};

const linkAndDeployVerifier = async (
  binFile: string,
): Promise<`0x${string}`> => {
  const binPath = `contracts/${binFile}`;
  const unlinkedBytecode = (await readFile(binPath, "utf8")).trimEnd();
  const linkRefFile = `contracts/${binFile.replace(
    /\.bin$/,
    ".linkrefs.json",
  )}`;

  let linkRefsRaw: string;
  try {
    linkRefsRaw = await readFile(linkRefFile, "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      if (unlinkedBytecode.includes("__$")) {
        throw new Error(
          `Missing link reference metadata for ${binFile}. ` +
            "Re-run noir/generate_fixtures.sh to regenerate verifier artifacts.",
        );
      }
      return deployBytecode(unlinkedBytecode);
    }
    throw error;
  }

  const linkRefs = JSON.parse(linkRefsRaw) as LinkReferences;
  const hasLibraries = Object.values(linkRefs).some(
    (libraries) => Object.keys(libraries).length > 0,
  );
  if (!hasLibraries) {
    return deployBytecode(unlinkedBytecode);
  }

  const programDir =
    binFile.lastIndexOf("/") >= 0
      ? binFile.slice(0, binFile.lastIndexOf("/"))
      : "";
  const programFilename = binFile.slice(binFile.lastIndexOf("/") + 1);
  if (!programFilename.endsWith(HONK_VERIFIER_SUFFIX)) {
    throw new Error(
      `Cannot infer program name from ${binFile}; expected suffix ${HONK_VERIFIER_SUFFIX}`,
    );
  }

  const programName = programFilename.slice(0, -HONK_VERIFIER_SUFFIX.length);
  const prefix = programDir.length > 0 ? `${programDir}/` : "";

  const placeholderMappings = new Map<string, `0x${string}`>();

  for (const [sourceName, libraries] of Object.entries(linkRefs)) {
    for (const libraryName of Object.keys(libraries)) {
      const placeholder = computePlaceholder(`${sourceName}:${libraryName}`);
      if (placeholderMappings.has(placeholder)) continue;

      const libraryBinFile = `${prefix}${programName}_${libraryName}.bin`;
      let libraryAddress: `0x${string}`;
      try {
        libraryAddress = await deployBin(libraryBinFile);
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT") {
          throw new Error(
            `Missing bytecode for ${libraryName} at ${libraryBinFile}. ` +
              "Re-run noir/generate_fixtures.sh to regenerate verifier artifacts.",
          );
        }
        throw error;
      }
      placeholderMappings.set(placeholder, libraryAddress);
      console.log(
        `Linked library ${libraryName} (${placeholder}) at ${libraryAddress}`,
      );
    }
  }

  let linkedBytecode = unlinkedBytecode;
  for (const [placeholder, address] of placeholderMappings.entries()) {
    if (!linkedBytecode.includes(placeholder)) {
      throw new Error(
        `Placeholder ${placeholder} not found in bytecode for ${binFile}`,
      );
    }
    linkedBytecode = linkedBytecode
      .split(placeholder)
      .join(address.replace(/^0x/, ""));
  }

  return await deployBytecode(linkedBytecode);
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

  if (useNoopVerifier) {
    await deployLibrariesForProgram("noir/agg_final_HonkVerifier.bin");
  }

  const aggregateBinFile = maybeNoopVerifier("noir/agg_final_HonkVerifier.bin");
  const aggregateVerifierAddr = await linkAndDeployVerifier(aggregateBinFile);
  console.log(`AGGREGATE_VERIFIER_ADDR=${aggregateVerifierAddr}`);

  const rollupV1 = await hre.viem.deployContract(
    "contracts/rollup3/RollupV1.sol:RollupV1",
  );
  console.log(`ROLLUP_V1_IMPL_ADDR=${rollupV1.address}`);

  const rollupInitializeCalldata = encodeFunctionData({
    abi: rollupV1.abi,
    functionName: "initialize",
    args: [
      ownerAddress,
      usdcAddress,
      aggregateVerifierAddr,
      proverAddress,
      validators,
      AGG_FINAL_VERIFICATION_KEY_HASH,
      DEFAULT_VERIFIER_MESSAGES_COUNT,
      LEGACY_POLYGON_USDC_NOTE_KIND,
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

  // Deploy EIP-7702 delegate smart account implementation (meta-tx, no EntryPoint).
  const eip7702Delegate = await hre.viem.deployContract(
    "contracts/Eip7702SimpleAccount.sol:Eip7702SimpleAccount",
  );
  console.log(`EIP7702_SIMPLE_ACCOUNT_ADDR=${eip7702Delegate.address}`);

  console.error("All contracts deployed");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
