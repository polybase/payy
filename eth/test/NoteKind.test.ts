import { expect } from "chai";
import {
  LEGACY_POLYGON_CHAIN_ID,
  POLYGON_USDC_ADDRESS,
  generateNoteKindBridgeEvm,
} from "../scripts/noteKind";

describe("note kind helpers", function () {
  it("derives the Polygon USDC note kind", function () {
    const noteKind = generateNoteKindBridgeEvm(
      LEGACY_POLYGON_CHAIN_ID,
      POLYGON_USDC_ADDRESS,
    );

    expect(noteKind).to.equal(
      "0x000200000000000000893c499c542cef5e3811e1192ce70d8cc03d5c33590000",
    );
  });
});
