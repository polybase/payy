use crate::{Client, Result};
use ethereum_types::U64;
use web3::contract::Contract;
use web3::transports::Http;
use web3::types::{Address, U256};

const ERC20_BALANCE_OF_ARTIFACT: &str = r#"{"abi":[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"stateMutability":"view","type":"function"}]}"#;

#[derive(Debug, Clone)]
pub struct ERC20Contract {
    client: Client,
    contract: Contract<Http>,
    address: Address,
    /// The ethereum block height used for all contract calls.
    /// If None, the latest block is used.
    block_height: Option<U64>,
}

impl ERC20Contract {
    pub fn new(client: Client, contract: Contract<Http>, address: Address) -> Self {
        Self {
            client,
            contract,
            address,
            block_height: None,
        }
    }

    pub async fn load(client: Client, contract_addr: &str) -> Result<Self> {
        let contract = client.load_contract_from_str(contract_addr, ERC20_BALANCE_OF_ARTIFACT)?;
        Ok(Self::new(client, contract, contract_addr.parse()?))
    }

    pub fn at_height(&self, block_height: u64) -> Self {
        Self {
            block_height: Some(U64::from(block_height)),
            ..self.clone()
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub async fn balance(&self, owner: Address) -> Result<U256> {
        let balance = self
            .client
            .query(
                &self.contract,
                "balanceOf",
                (owner,),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;
        Ok(balance)
    }
}
