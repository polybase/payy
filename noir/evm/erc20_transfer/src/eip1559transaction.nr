use common::Address;

/// Represents an unsigned Ethereum EIP-1559 (type-2) transaction
pub struct Type2Transaction<let N: u32> {
    pub chain_id: u64,
    pub nonce: u64,
    pub max_priority_fee_per_gas: u128,
    pub max_fee_per_gas: u128,
    pub gas_limit: u64,
    pub to: Address,
    pub value: u128,
    pub data: [u8; N],
}

impl<let DATA_LEN: u32> Type2Transaction<DATA_LEN> {
    pub fn hash(self) -> [u8; 32] {
        let (encoded, len) = self.rlp_encode();
        keccak256::keccak256(encoded, len)
    }

    /// RLP encode the transaction for hashing
    fn rlp_encode(self) -> ([u8; 256], u32) {
        let mut encoded = [0u8; 256];

        encoded[0] = 0x02; // transaction type 2
        encoded[1] = 0xf8; // 55 < length of payload <= 255
        // payload length is written to encoded[2] at the end

        let mut offset = 3;

        offset = rlp_append_u64(&mut encoded, offset, self.chain_id);
        offset = rlp_append_u64(&mut encoded, offset, self.nonce);
        offset = rlp_append_u128(&mut encoded, offset, self.max_priority_fee_per_gas);
        offset = rlp_append_u128(&mut encoded, offset, self.max_fee_per_gas);
        offset = rlp_append_u64(&mut encoded, offset, self.gas_limit);

        encoded[offset] = 0x94;
        offset += 1;
        for i in 0..20 {
            encoded[offset + i] = self.to[i];
        }
        offset += 20;

        offset = rlp_append_u128(&mut encoded, offset, self.value);

        encoded[offset] = 0xb8;
        assert((55 < DATA_LEN) & (DATA_LEN <= 255), "Calldata length outside supported range");
        encoded[offset + 1] = DATA_LEN as u8;
        offset += 2;
        for i in 0..DATA_LEN {
            encoded[offset + i] = self.data[i];
        }
        offset += DATA_LEN;

        encoded[offset] = 0xc0;
        offset += 1;

        let payload_len = offset - 3;
        assert((55 < payload_len) & (payload_len <= 255), "Payload length outside supported range");
        encoded[2] = payload_len as u8;

        (encoded, offset)
    }
}

fn rlp_append_u64<let N: u32>(out: &mut [u8; N], offset: u32, value: u64) -> u32 {
    if (value != 0) & (value < 0x80) {
        out[offset] = value as u8;
        offset + 1
    } else {
        let mut bytes = (value as Field).to_be_bytes::<8>();
        rlp_append_integer(out, offset, bytes)
    }
}

fn rlp_append_u128<let N: u32>(out: &mut [u8; N], offset: u32, value: u128) -> u32 {
    if (value != 0) & (value < 0x80) {
        out[offset] = value as u8;
        offset + 1
    } else {
        let mut bytes = (value as Field).to_be_bytes::<16>();
        rlp_append_integer(out, offset, bytes)
    }
}

fn rlp_append_integer<let N: u32, let M: u32>(out: &mut [u8; N], offset: u32, value: [u8; M]) -> u32 {
    let mut next_offset = offset + 1;
    let mut found = false;
    for i in 0..M {
        if value[i] != 0 {
            found = true;
        }
        if found {
            out[next_offset] = value[i];
            next_offset += 1;
        }
    }

    let len = next_offset - offset - 1;
    out[offset] = 0x80 + (len as u8);

    next_offset
}

#[test]
fn rlp_encoding() {
    let mut data = [0u8; 68];
    data[0] = 0x12;
    data[1] = 0x34;
    data[2] = 0x56;
    data[3] = 0x78;

    for i in 0..32 {
        data[4 + i] = 0x11;
        data[36 + i] = 0x22;
    }

    let tx = Type2Transaction {
        chain_id: 1,
        nonce: 0,
        max_priority_fee_per_gas: 2000000000,
        max_fee_per_gas: 100000000000,
        gas_limit: 21000,
        to: [0x11; 20],
        value: 0,
        data,
    };

    let (encoded, len) = tx.rlp_encode();

    let expected: [u8; 112] = [0x02, 0xf8, 0x6d, 0x01, 0x80, 0x84, 0x77, 0x35, 0x94, 0x00, 0x85, 0x17, 0x48, 0x76, 0xe8, 0x00, 0x82, 0x52, 0x08, 0x94, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x80, 0xb8, 0x44, 0x12, 0x34, 0x56, 0x78, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0xc0];

    for i in 0..expected.len() {
        let correct = expected[i];
        let test = encoded[i];
        assert(test == correct, f"byte {i} is {test} but should be {correct}")
    }

    assert(len == expected.len(), f"Unexpected length {len}");
}
