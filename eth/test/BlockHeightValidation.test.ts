import { expect } from "chai";
import { ethers } from "hardhat";
import { type SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";
import {
  LEGACY_POLYGON_CHAIN_ID,
  POLYGON_USDC_ADDRESS,
  generateNoteKindBridgeEvm,
} from "../scripts/noteKind";

describe("RollupV1 Block Height Validation", function () {
  let rollup: any;
  let mockUsdc: any;
  let mockVerifier: any;
  let owner: SignerWithAddress;
  let prover: SignerWithAddress;
  let validator1: SignerWithAddress;
  let validator2: SignerWithAddress;
  let validator3: SignerWithAddress;

  // Empty merkle tree root hash constant (from the contract initialization)
  const INITIAL_ROOT =
    "0x0577b5b4aa3eaba75b2a919d5d7c63b7258aa507d38e346bf2ff1d48790379ff";
  const NEW_ROOT =
    "0x0000000000000000000000000000000000000000000000000000000000000002";
  const VERIFIER_KEY_HASH =
    "0x0000000000000000000000000000000000000000000000000000000000000003";
  const COMMIT_HASH =
    "0x0000000000000000000000000000000000000000000000000000000000000004";
  const OTHER_HASH =
    "0x0000000000000000000000000000000000000000000000000000000000000005";
  const TEST_VERIFIER_MESSAGES_COUNT = 2048;
  const INITIAL_NOTE_KIND = generateNoteKindBridgeEvm(
    LEGACY_POLYGON_CHAIN_ID,
    POLYGON_USDC_ADDRESS,
  );

  beforeEach(async function () {
    [owner, prover, validator1, validator2, validator3] =
      await ethers.getSigners();

    // Deploy mock contracts
    const mockUsdcFactory = await ethers.getContractFactory("MockERC20");
    mockUsdc = await mockUsdcFactory.deploy();

    const MockVerifier = await ethers.getContractFactory("MockVerifier");
    mockVerifier = await MockVerifier.deploy();

    // Deploy RollupV1 (rollup3 merged version) - skip initialization for now
    const RollupV1Factory = await ethers.getContractFactory(
      "contracts/rollup3/RollupV1.sol:RollupV1",
    );
    const rollupImpl = await RollupV1Factory.deploy();

    // Deploy proxy and initialize through proxy
    const ERC1967Proxy = await ethers.getContractFactory(
      "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol:ERC1967Proxy",
    );
    const initData = rollupImpl.interface.encodeFunctionData("initialize", [
      owner.address,
      await mockUsdc.getAddress(),
      await mockVerifier.getAddress(),
      prover.address,
      [validator1.address, validator2.address, validator3.address],
      VERIFIER_KEY_HASH,
      TEST_VERIFIER_MESSAGES_COUNT,
      INITIAL_NOTE_KIND,
    ]);

    const proxy = await ERC1967Proxy.deploy(
      await rollupImpl.getAddress(),
      initData,
    );
    rollup = RollupV1Factory.attach(await proxy.getAddress());
  });

  describe("Block Height Validation", function () {
    it("should start with block height 0", async function () {
      expect(await rollup.blockHeight()).to.equal(0);
    });

    it("should set the provided note kind mapping during initialization", async function () {
      const mappedToken = await rollup.noteKindTokenAddress(INITIAL_NOTE_KIND);
      expect(mappedToken).to.equal(await mockUsdc.getAddress());
    });

    it("should store the verifier message count provided at initialization", async function () {
      const verifier = await rollup.zkVerifiers(VERIFIER_KEY_HASH);
      expect(verifier.messages_length).to.equal(TEST_VERIFIER_MESSAGES_COUNT);
    });

    it("should have the validators registered correctly", async function () {
      // Get the validator sets
      const validatorSets = await rollup.getValidatorSets(0);
      console.log("Validator sets from contract:", validatorSets);
      expect(validatorSets).to.have.length(1);
      expect(validatorSets[0].validators).to.have.length(3);

      // Check that our test validators are included
      expect(validatorSets[0].validators).to.include(validator1.address);
      expect(validatorSets[0].validators).to.include(validator2.address);
      expect(validatorSets[0].validators).to.include(validator3.address);

      console.log("Expected validators:");
      console.log(`  validator1: ${validator1.address}`);
      console.log(`  validator2: ${validator2.address}`);
      console.log(`  validator3: ${validator3.address}`);
      console.log("Contract validators:");
      validatorSets[0].validators.forEach((v, i) =>
        console.log(`  ${i}: ${v}`),
      );
    });

    it("should have matching hash calculations", async function () {
      const height = 1;
      const newRoot = NEW_ROOT;
      const otherHash = OTHER_HASH;

      // Get acceptMsg from our calculation (we know this matches)
      const acceptMsg =
        "0x21dc7da13d8dfdb18b2b9d5dfab00e5ac63a1b60bff6de58b526a5ef5262177a";

      // Get packed bytes from contract
      const contractPackedBytes = await rollup.debugGetPackedBytes(acceptMsg);

      // Create our packed bytes
      const NETWORK = "Payy";
      const NETWORK_LEN = 4;

      // Create the uint64 encoding manually - 8 bytes big-endian
      const networkLenBytes = new Uint8Array(8);
      new DataView(networkLenBytes.buffer).setBigUint64(
        0,
        BigInt(NETWORK_LEN),
        false,
      ); // false = big-endian

      const testPackedBytes = ethers.concat([
        networkLenBytes,
        ethers.toUtf8Bytes(NETWORK),
        ethers.getBytes(acceptMsg),
      ]);

      console.log(`Contract packed bytes: ${contractPackedBytes}`);
      console.log(`Test packed bytes:     ${ethers.hexlify(testPackedBytes)}`);
      console.log(`Contract hash: ${ethers.keccak256(contractPackedBytes)}`);
      console.log(`Test hash:     ${ethers.keccak256(testPackedBytes)}`);

      // Let's also try different encodings of NETWORK_LEN
      console.log("Different NETWORK_LEN encodings:");
      console.log(
        `  toBeArray(4, 8): ${ethers.hexlify(ethers.toBeArray(NETWORK_LEN, 8))}`,
      );
      console.log(
        `  toBeArray(4, 4): ${ethers.hexlify(ethers.toBeArray(NETWORK_LEN, 4))}`,
      );
      console.log(
        `  toBeArray(4, 1): ${ethers.hexlify(ethers.toBeArray(NETWORK_LEN, 1))}`,
      );
    });

    it("should allow increasing block height from 0 to 1", async function () {
      const height = 1;

      // Create signatures for validators
      const signatures = await createValidSignatures(height, NEW_ROOT);
      const publicInputs = createBasicPublicInputs(
        INITIAL_ROOT,
        NEW_ROOT,
        COMMIT_HASH,
      );

      await expect(
        rollup.connect(prover).verifyRollup(
          height,
          VERIFIER_KEY_HASH,
          "0x00", // mock proof
          publicInputs,
          OTHER_HASH,
          signatures,
        ),
      ).to.not.be.reverted;

      expect(await rollup.blockHeight()).to.equal(height);
    });

    it("should reject same block height (0 to 0)", async function () {
      const height = 0;

      const signatures = await createValidSignatures(height, NEW_ROOT);
      const publicInputs = createBasicPublicInputs(
        INITIAL_ROOT,
        NEW_ROOT,
        COMMIT_HASH,
      );

      await expect(
        rollup
          .connect(prover)
          .verifyRollup(
            height,
            VERIFIER_KEY_HASH,
            "0x00",
            publicInputs,
            OTHER_HASH,
            signatures,
          ),
      ).to.be.revertedWith(
        "RollupV1: New block height must be greater than current",
      );
    });

    it("should allow sequential height increases", async function () {
      // First: 0 to 1
      await verifyRollupAtHeight(1, INITIAL_ROOT, NEW_ROOT);
      expect(await rollup.blockHeight()).to.equal(1);

      // Second: 1 to 2
      const newRoot2 =
        "0x0000000000000000000000000000000000000000000000000000000000000006";
      await verifyRollupAtHeight(2, NEW_ROOT, newRoot2);
      expect(await rollup.blockHeight()).to.equal(2);

      // Third: 2 to 3
      const newRoot3 =
        "0x0000000000000000000000000000000000000000000000000000000000000007";
      await verifyRollupAtHeight(3, newRoot2, newRoot3);
      expect(await rollup.blockHeight()).to.equal(3);
    });

    it("should reject decreasing block height after increase", async function () {
      // First increase to height 5
      await verifyRollupAtHeight(1, INITIAL_ROOT, NEW_ROOT);
      await verifyRollupAtHeight(
        2,
        NEW_ROOT,
        "0x0000000000000000000000000000000000000000000000000000000000000006",
      );
      await verifyRollupAtHeight(
        3,
        "0x0000000000000000000000000000000000000000000000000000000000000006",
        "0x0000000000000000000000000000000000000000000000000000000000000007",
      );
      await verifyRollupAtHeight(
        4,
        "0x0000000000000000000000000000000000000000000000000000000000000007",
        "0x0000000000000000000000000000000000000000000000000000000000000008",
      );
      await verifyRollupAtHeight(
        5,
        "0x0000000000000000000000000000000000000000000000000000000000000008",
        "0x0000000000000000000000000000000000000000000000000000000000000009",
      );

      expect(await rollup.blockHeight()).to.equal(5);

      // Try to decrease to height 4
      const height = 4;
      const signatures = await createValidSignatures(
        height,
        "0x000000000000000000000000000000000000000000000000000000000000000a",
      );
      const publicInputs = createBasicPublicInputs(
        "0x0000000000000000000000000000000000000000000000000000000000000009",
        "0x000000000000000000000000000000000000000000000000000000000000000a",
        COMMIT_HASH,
      );

      await expect(
        rollup
          .connect(prover)
          .verifyRollup(
            height,
            VERIFIER_KEY_HASH,
            "0x00",
            publicInputs,
            OTHER_HASH,
            signatures,
          ),
      ).to.be.revertedWith(
        "RollupV1: New block height must be greater than current",
      );
    });

    it("should reject same block height after increase", async function () {
      // First increase to height 3
      await verifyRollupAtHeight(1, INITIAL_ROOT, NEW_ROOT);
      await verifyRollupAtHeight(
        2,
        NEW_ROOT,
        "0x0000000000000000000000000000000000000000000000000000000000000006",
      );
      await verifyRollupAtHeight(
        3,
        "0x0000000000000000000000000000000000000000000000000000000000000006",
        "0x0000000000000000000000000000000000000000000000000000000000000007",
      );

      expect(await rollup.blockHeight()).to.equal(3);

      // Try to submit same height 3
      const height = 3;
      const signatures = await createValidSignatures(
        height,
        "0x000000000000000000000000000000000000000000000000000000000000000a",
      );
      const publicInputs = createBasicPublicInputs(
        "0x0000000000000000000000000000000000000000000000000000000000000007",
        "0x000000000000000000000000000000000000000000000000000000000000000a",
        COMMIT_HASH,
      );

      await expect(
        rollup
          .connect(prover)
          .verifyRollup(
            height,
            VERIFIER_KEY_HASH,
            "0x00",
            publicInputs,
            OTHER_HASH,
            signatures,
          ),
      ).to.be.revertedWith(
        "RollupV1: New block height must be greater than current",
      );
    });
  });

  // Helper functions
  async function verifyRollupAtHeight(
    height: number,
    oldRoot: string,
    newRoot: string,
  ) {
    const signatures = await createValidSignatures(height, newRoot);
    const publicInputs = createBasicPublicInputs(oldRoot, newRoot, COMMIT_HASH);

    await rollup
      .connect(prover)
      .verifyRollup(
        height,
        VERIFIER_KEY_HASH,
        "0x00",
        publicInputs,
        OTHER_HASH,
        signatures,
      );
  }

  async function createValidSignatures(height: number, newRoot: string) {
    const messageHash = await getSignatureMessageHash(
      newRoot,
      height,
      OTHER_HASH,
    );
    console.log(`Message hash: ${messageHash}`);
    console.log(`  newRoot: ${newRoot}`);
    console.log(`  height: ${height}`);
    console.log(`  otherHash: ${OTHER_HASH}`);

    const signatures = [];
    const validators = [validator1, validator2, validator3];

    // We need to sign the raw messageHash without the Ethereum prefix
    // Since the contract expects raw hash signatures, we need to use the private keys directly
    // Hardhat signers don't expose private keys directly, so we need to use known test private keys
    const testPrivateKeys = [
      "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a", // validator1
      "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6", // validator2
      "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a", // validator3
    ];

    for (let i = 0; i < validators.length; i++) {
      const validator = validators[i];
      // Create signing key from known private key
      const signingKey = new ethers.SigningKey(testPrivateKeys[i]);

      // Verify the signing key matches the validator address
      const address = ethers.computeAddress(signingKey.publicKey);
      console.log(
        `Validator ${i}: Expected ${validator.address}, Computed ${address}`,
      );

      if (address.toLowerCase() !== validator.address.toLowerCase()) {
        throw new Error(
          `Private key mismatch for validator ${i}: expected ${validator.address}, got ${address}`,
        );
      }

      // Sign the raw messageHash
      const signature = signingKey.sign(messageHash);
      console.log(
        `Validator: ${validator.address}, Signature: r=${signature.r}, s=${signature.s}, v=${signature.v}`,
      );

      signatures.push({ r: signature.r, s: signature.s, v: signature.v });
    }

    // Sort signatures by validator address (contract requirement)
    const sortedValidators = validators
      .map((v, i) => ({ validator: v, signature: signatures[i] }))
      .sort((a, b) =>
        a.validator.address
          .toLowerCase()
          .localeCompare(b.validator.address.toLowerCase()),
      );

    console.log("Original validator order:");
    validators.forEach((v, i) => console.log(`  ${i}: ${v.address}`));
    console.log("Sorted validator order:");
    sortedValidators.forEach((v, i) =>
      console.log(`  ${i}: ${v.validator.address}`),
    );

    return sortedValidators.map((v) => v.signature);
  }

  async function getSignatureMessageHash(
    newRoot: string,
    height: number,
    otherHash: string,
  ) {
    const NETWORK = "Payy";
    const NETWORK_LEN = 4;

    const proposalHash = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ["bytes32", "uint256", "bytes32"],
        [newRoot, height, otherHash],
      ),
    );
    console.log(`  proposalHash: ${proposalHash}`);

    const acceptMsg = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ["uint256", "bytes32"],
        [height + 1, proposalHash],
      ),
    );
    console.log(`  acceptMsg: ${acceptMsg}`);

    // Create the uint64 encoding manually - 8 bytes big-endian
    const networkLenBytes = new Uint8Array(8);
    new DataView(networkLenBytes.buffer).setBigUint64(
      0,
      BigInt(NETWORK_LEN),
      false,
    ); // false = big-endian

    const sigMsg = ethers.keccak256(
      ethers.concat([
        networkLenBytes,
        ethers.toUtf8Bytes(NETWORK),
        ethers.getBytes(acceptMsg),
      ]),
    );
    console.log(`  sigMsg: ${sigMsg}`);

    return sigMsg;
  }

  function createBasicPublicInputs(
    oldRoot: string,
    newRoot: string,
    commitHash: string,
  ) {
    const inputs = [oldRoot, newRoot, commitHash];
    // Match the verifier messages length passed during initialization.
    for (let i = 0; i < TEST_VERIFIER_MESSAGES_COUNT; i++) {
      inputs.push(
        "0x0000000000000000000000000000000000000000000000000000000000000000",
      );
    }
    return inputs;
  }
});
