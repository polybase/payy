use crate::{Client, Result};
use ethereum_types::U64;
use web3::contract::{Contract, tokens::Tokenizable};
use web3::transports::Http;
use web3::types::{Address, FilterBuilder, H256, U256};

/// EIP-7702 SimpleAccount helper for idempotency queries
#[derive(Debug, Clone)]
pub struct Eip7702Account {
    client: Client,
    contract: Contract<Http>,
    address: Address,
    /// Optional fixed block height for consistent queries
    block_height: Option<U64>,
}

impl Eip7702Account {
    pub fn new(client: Client, contract: Contract<Http>, address: Address) -> Self {
        Self {
            client,
            contract,
            address,
            block_height: None,
        }
    }

    pub async fn load(client: Client, account_addr: &str) -> Result<Self> {
        let contract_json = include_str!(
            "../../../eth/artifacts/contracts/Eip7702SimpleAccount.sol/Eip7702SimpleAccount.json"
        );
        let contract = client.load_contract_from_str(account_addr, contract_json)?;
        Ok(Self::new(client, contract, account_addr.parse()?))
    }

    pub fn at_height(mut self, block_height: Option<u64>) -> Self {
        self.block_height = block_height.map(|x| x.into());
        self
    }

    pub fn address(&self) -> Address {
        self.address
    }

    /// Check if a meta nonce was used by scanning the NonceUsed event; returns txn hash if found.
    ///
    /// Contract ABI: NonceUsed(uint256 indexed nonce)
    pub async fn nonce_used_txn(&self, nonce: U256) -> Result<Option<H256>> {
        let event = self.contract.abi().event("NonceUsed")?;
        let topic_filter = web3::ethabi::RawTopicFilter {
            topic0: web3::ethabi::Topic::This(nonce.into_token()),
            // Event has a single indexed parameter; leave remaining topics unconstrained
            topic1: web3::ethabi::Topic::Any,
            topic2: web3::ethabi::Topic::Any,
        };
        let tf = event.filter(topic_filter)?;
        let filter = FilterBuilder::default()
            .address(vec![self.address])
            .from_block(web3::types::BlockNumber::Earliest)
            .to_block(web3::types::BlockNumber::Latest)
            .topic_filter(tf)
            .build();

        let logs = self.client.logs(filter).await?;
        Ok(logs.into_iter().filter_map(|l| l.transaction_hash).next())
    }

    /// Direct view call to isNonceUsed(uint256)
    pub async fn is_nonce_used(&self, nonce: U256) -> Result<bool> {
        let used: bool = self
            .client
            .query(
                &self.contract,
                "isNonceUsed",
                (nonce,),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;
        Ok(used)
    }
}
