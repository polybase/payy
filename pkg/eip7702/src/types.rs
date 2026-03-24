use std::time::Duration;

use ethereum_types::{Address, H256, U256};
use secp256k1::SecretKey;
use test_spy::spy_mock;

use crate::error::Result;

/// Authorization tuple for EIP-7702 SetCode transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    pub chain_id: U256,
    pub delegate: Address,
    pub nonce: U256,
    pub y_parity: u8,
    pub r: H256,
    pub s: H256,
}

/// Meta-call structure used by executeMetaMany.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaCall {
    pub target: Address,
    pub value: U256,
    pub data: Vec<u8>,
}

#[spy_mock]
#[async_trait::async_trait]
pub trait Eip7702Relayer {
    async fn ensure_upgraded(
        &self,
        user_sk: &SecretKey,
        user: Address,
        delegate: Address,
        relayer: Address,
    ) -> Result<H256>;

    #[allow(clippy::too_many_arguments)]
    async fn send_meta_tx(
        &self,
        user_sk: &SecretKey,
        user: Address,
        calls: Vec<MetaCall>,
        relayer: Address,
        nonce: U256,
        valid_after: U256,
        valid_until: U256,
    ) -> Result<H256>;

    async fn wait_for_upgrade(
        &self,
        user: Address,
        delegate: Address,
        timeout: Duration,
    ) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_call_debug_eq() {
        let a = MetaCall {
            target: Address::zero(),
            value: U256::zero(),
            data: vec![1, 2, 3],
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert!(format!("{a:?}").contains("MetaCall"));
    }
}
