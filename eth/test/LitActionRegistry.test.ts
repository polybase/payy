import { expect } from "chai";
import { ethers } from "hardhat";
import { HardhatEthersSigner } from "@nomicfoundation/hardhat-ethers/signers";

describe("LitActionRegistry", function () {
  let owner: HardhatEthersSigner;
  let nonOwner: HardhatEthersSigner;
  let registry: any;

  beforeEach(async function () {
    [owner, nonOwner] = await ethers.getSigners();

    const LitActionRegistry = await ethers.getContractFactory("LitActionRegistry");
    registry = await LitActionRegistry.deploy(owner.address);
    await registry.waitForDeployment();
  });

  it("sets child lit action CID", async function () {
    const cid = "QmChildCID";

    await expect(registry.setChildLitActionCID(cid))
      .to.emit(registry, "ChildLitActionCIDUpdated")
      .withArgs("", cid);

    expect(await registry.getChildLitActionCID()).to.equal(cid);
  });

  it("returns CID via alias", async function () {
    const cid = "QmAliasCID";
    await registry.setChildLitActionCID(cid);

    expect(await registry.getChildIPFSCID()).to.equal(cid);
  });

  it("enforces owner-only setter", async function () {
    await expect(
      registry.connect(nonOwner).setChildLitActionCID("cid"),
    )
      .to.be.revertedWithCustomError(registry, "OwnableUnauthorizedAccount")
      .withArgs(nonOwner.address);
  });

  it("rejects empty CID", async function () {
    await expect(registry.setChildLitActionCID(""))
      .to.be.revertedWith("Child Lit Action CID cannot be empty");
  });

  it("emits update with previous CID", async function () {
    const first = "QmFirst";
    const second = "QmSecond";

    await registry.setChildLitActionCID(first);

    await expect(registry.setChildLitActionCID(second))
      .to.emit(registry, "ChildLitActionCIDUpdated")
      .withArgs(first, second);
  });
});
