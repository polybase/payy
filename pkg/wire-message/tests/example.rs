#![allow(clippy::disallowed_names)]
use borsh::{BorshDeserialize, BorshSerialize};
use wire_message::{Error, WireMessage, wire_message};

#[wire_message]
enum ExampleMessage {
    V1(V1),
    V2(V2),
    V3(V3),
}

#[derive(BorshSerialize, BorshDeserialize)]
struct V1 {
    foo: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct V2 {
    foo: Vec<u8>,
    bar: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct V3 {
    bar: Vec<u8>,
}

impl WireMessage for ExampleMessage {
    type Ctx = ();
    type Err = core::convert::Infallible;

    const MAX_VERSION: u64 = 3;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
            Self::V2(_) => 2,
            Self::V3(_) => 3,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, Error> {
        match self {
            Self::V1(V1 { foo }) => Ok(Self::V2(V2 { foo, bar: vec![] })),
            Self::V2(V2 { bar, .. }) => Ok(Self::V3(V3 { bar })),
            Self::V3(_) => Err(Self::max_version_error()),
        }
    }
}

fn main() {
    let message = ExampleMessage::V1(V1 { foo: vec![1, 2, 3] });
    let v3 = message.upgrade(&mut ()).unwrap();

    assert_eq!(v3.version(), 3);
}
