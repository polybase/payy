import { HardhatEthersSigner } from "@nomicfoundation/hardhat-ethers/signers";
import { expect } from "chai";
import { ethers } from "hardhat";

describe("SocialRecovery", function () {
  let owner: HardhatEthersSigner;
  let user: HardhatEthersSigner;
  let nonOwner: HardhatEthersSigner;
  let socialRecovery: any;

  beforeEach(async function () {
    [owner, user, nonOwner] = await ethers.getSigners();

    const SocialRecovery = await ethers.getContractFactory("SocialRecovery");
    socialRecovery = await SocialRecovery.deploy(owner.address);
    await socialRecovery.waitForDeployment();
  });

  describe("Guardian CID functionality", function () {
    it("Should add guardian successfully", async function () {
      await socialRecovery.addGuardianCID(
        user.address,
        "Password",
        "guardian-data",
      );

      const config = await socialRecovery.getGuardianConfig(user.address);
      expect(config.enabled).to.equal(true);
      expect(config.guardianCount).to.equal(1);
      expect(config.threshold).to.equal(1);
    });

    it("Should not allow duplicate guardians", async function () {
      await socialRecovery.addGuardianCID(user.address, "Password", "password-1");

      await expect(
        socialRecovery.addGuardianCID(user.address, "Password", "password-1"),
      ).to.be.revertedWith("Guardian Already exists");
    });

    it("Should return guardian entry for CID hash", async function () {
      const cid = "Password";
      const guardianValue = "guardian-data";

      await socialRecovery.addGuardianCID(user.address, cid, guardianValue);

      const cidHash = ethers.keccak256(ethers.toUtf8Bytes(cid));
      const storedGuardian = await socialRecovery.getGuardianEntry(
        user.address,
        cidHash,
      );

      expect(storedGuardian).to.equal(
        ethers.hexlify(ethers.toUtf8Bytes(guardianValue)),
      );

      const config = await socialRecovery.getGuardianConfig(user.address);
      expect(config.guardianCIDs[0]).to.equal(cidHash);
    });

    it("Should reject empty guardian value", async function () {
      await expect(
        socialRecovery.addGuardianCID(user.address, "Password", ""),
      ).to.be.revertedWith("Guardian value cannot be empty");
    });
  });
});
