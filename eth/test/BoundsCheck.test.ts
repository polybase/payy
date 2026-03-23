import { expect } from "chai";
import { ethers } from "hardhat";

describe("Bounds Check Logic Tests", function () {
  let boundsTest: any;

  const MAX_FUTURE_BLOCKS = 2_592_000; // 30 days worth of blocks

  beforeEach(async function () {
    const BoundsTestFactory = await ethers.getContractFactory("BoundsTestContract");
    boundsTest = await BoundsTestFactory.deploy();
  });

  describe("validateBounds function", function () {
    it("should allow validFrom = 0 (special case for initialization)", async function () {
      await expect(boundsTest.validateBounds(0)).to.not.be.rejected;
    });

    it("should allow validFrom within 30 days of current block", async function () {
      const currentBlock = await boundsTest.getCurrentBlock();
      const validFrom = currentBlock + 1000n; // 1000 blocks in the future

      await expect(boundsTest.validateBounds(validFrom)).to.not.be.rejected;
    });

    it("should allow validFrom exactly at the boundary (30 days)", async function () {
      const maxAllowed = await boundsTest.getMaxAllowedValidFrom();
      
      await expect(boundsTest.validateBounds(maxAllowed)).to.not.be.rejected;
    });

    it("should reject validFrom more than 30 days in the future", async function () {
      const maxAllowed = await boundsTest.getMaxAllowedValidFrom();
      const validFrom = maxAllowed + 1n; // One block beyond the limit

      await expect(boundsTest.validateBounds(validFrom))
        .to.be.revertedWith("BoundsTest: validFrom cannot be more than 30 days in the future");
    });

    it("should reject extremely large validFrom values", async function () {
      const validFrom = ethers.MaxUint256; // type(uint256).max

      await expect(boundsTest.validateBounds(validFrom))
        .to.be.revertedWith("BoundsTest: validFrom cannot be more than 30 days in the future");
    });

    it("should have correct MAX_FUTURE_BLOCKS constant", async function () {
      const maxFutureBlocks = await boundsTest.getMaxFutureBlocks();
      expect(Number(maxFutureBlocks)).to.equal(MAX_FUTURE_BLOCKS);
    });

    it("should calculate correct max allowed validFrom", async function () {
      const currentBlock = await boundsTest.getCurrentBlock();
      const maxAllowed = await boundsTest.getMaxAllowedValidFrom();
      const expectedMax = currentBlock + BigInt(MAX_FUTURE_BLOCKS);
      
      expect(maxAllowed).to.equal(expectedMax);
    });
  });
});