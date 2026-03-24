use super::Base64Bytes;
use serde::{Deserialize, Serialize};

#[test]
fn base64bytes_json_roundtrip() {
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Payload {
        data: Base64Bytes,
    }

    let bytes = vec![0xde, 0xad, 0xbe, 0xef];
    let payload = Payload {
        data: Base64Bytes::from(bytes.clone()),
    };

    let json = serde_json::to_string(&payload).expect("serialize payload");
    assert_eq!(json, "{\"data\":\"3q2+7w==\"}");

    let decoded: Payload = serde_json::from_str(&json).expect("deserialize payload");
    assert_eq!(decoded, payload);
    assert_eq!(decoded.data.as_slice(), bytes.as_slice());
}

#[test]
fn base64bytes_from_impls_preserve_data() {
    let bytes = vec![1, 2, 3, 4];
    let from_vec = Base64Bytes::from(bytes.clone());
    let from_slice = Base64Bytes::from(bytes.as_slice());

    assert_eq!(from_vec.as_slice(), bytes.as_slice());
    assert_eq!(from_slice.as_slice(), bytes.as_slice());

    let round_trip: Vec<u8> = from_vec.into();
    assert_eq!(round_trip, bytes);
}

#[test]
fn deserialize_base64_trims_input() {
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Payload {
        data: Base64Bytes,
    }

    let json = "{\"data\":\"  3q2+7w==\\n\"}";
    let decoded: Payload = serde_json::from_str(json).expect("deserialize payload with whitespace");

    assert_eq!(decoded.data.as_slice(), &[0xde, 0xad, 0xbe, 0xef]);
}
