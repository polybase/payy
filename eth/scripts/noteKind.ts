import { bytesToHex, hexToBytes, type Hex } from "viem";

const NOTE_KIND_LENGTH = 32;
const ADDRESS_LENGTH = 20;

export const BRIDGED_EVM_NOTE_KIND_FORMAT = 2n;
export const LEGACY_POLYGON_CHAIN_ID = 137n;
export const POLYGON_USDC_ADDRESS =
  "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359" as `0x${string}`;

export function generateNoteKindBridgeEvm(
  chain: bigint,
  address: `0x${string}`,
): Hex {
  if (chain < 0n || chain > 0xffff_ffff_ffff_ffffn) {
    throw new Error("chain must fit within uint64");
  }

  const bytes = new Uint8Array(NOTE_KIND_LENGTH);
  const view = new DataView(bytes.buffer);

  view.setUint16(
    0,
    Number(BRIDGED_EVM_NOTE_KIND_FORMAT),
    false,
  );
  view.setBigUint64(2, chain, false);

  const addressBytes = hexToBytes(address);
  if (addressBytes.length !== ADDRESS_LENGTH) {
    throw new Error("address must be exactly 20 bytes");
  }
  bytes.set(addressBytes, 10);

  return bytesToHex(bytes);
}
