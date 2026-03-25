// lint-long-file-override allow-max-lines=300
use std::str::FromStr;

use clap::Parser;
use contracts::{Client, USDCContract, wallet::Wallet};
use element::Element;
use ethereum_types::U256;
use eyre::Result;
use serde::{Deserialize, Serialize};
use web3::types::Address;
use zk_primitives::{
    Note, NoteURLPayload, bridged_polygon_usdc_note_kind, get_address_for_private_key,
};

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
#[clap(name = "Wallet")]
#[command(author = "Dev Wallet <hello@polybase.xyz>")]
#[command(author, version, about = "Dev Wallet", long_about = None)]
#[command(propagate_version = true)]
pub struct CliArgs {
    #[clap(subcommand)]
    pub command: Command,

    #[clap(
        short,
        long,
        default_value = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    )]
    pub private_key: String,

    #[clap(long, default_value = "http://localhost:8545")]
    pub rpc_url: String,

    #[clap(long, default_value = "1337")]
    pub chain_id: u128,

    #[clap(long, default_value = "5fbdb2315678afecb367f032d93f642f64180aa3")]
    pub usdc_addr: String,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub enum Command {
    Eth(EthArgs),
    Balance(BalanceArgs),
    Transfer(TransferArgs),
    DecodePayload(DecodePayloadArgs),
    EncodePayload,
    Halo2Address(Halo2AddressArgs),
    Commitment(CommitmentArgs),
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct CommitmentArgs {
    #[clap(name = "address", index = 1)]
    pub address: String,
    #[clap(name = "psi", index = 2)]
    pub psi: String,
    #[clap(name = "value", index = 3)]
    pub value: u64,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct Halo2AddressArgs {
    #[clap(name = "private_key", index = 1)]
    pub private_key: String,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct EthArgs {
    #[clap(name = "address", index = 1)]
    pub address: Option<Address>,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct BalanceArgs {
    #[clap(name = "address", index = 1)]
    pub address: Option<Address>,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct TransferArgs {
    #[clap(name = "to", index = 1)]
    pub to: Address,

    #[clap(name = "value", index = 2)]
    pub value: f64,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct NullifierArgs {
    #[clap(name = "commitment", index = 1)]
    pub commitment: Element,

    #[clap(name = "psi", index = 2)]
    pub psi: Element,
}

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct DecodePayloadArgs {
    #[clap(name = "payload", index = 1)]
    pub payload: String,
}

pub struct PolyWallet {
    wallet: Wallet,
    usdc: USDCContract,
}

impl PolyWallet {
    pub fn new(usdc: USDCContract, wallet: Wallet) -> Self {
        Self { usdc, wallet }
    }

    pub async fn balance(&self, owner: Address) -> Result<U256> {
        let bal = self.usdc.balance(owner).await?;
        Ok(bal)
    }

    pub async fn transfer_usdc(&self, value: u128, to: Address) -> Result<()> {
        self.usdc.transfer(to, value).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    let args = CliArgs::parse();

    let private_key = &args.private_key;
    let wallet = Wallet::new_from_str(private_key)?;
    let client = Client::new(&args.rpc_url, None);

    let poly = PolyWallet::new(
        USDCContract::load(
            client.clone(),
            args.chain_id,
            &args.usdc_addr,
            wallet.web3_secret_key(),
        )
        .await?,
        wallet,
    );

    match args.command {
        Command::Eth(args) => {
            let address = args.address.unwrap_or(poly.wallet.to_eth_address());
            let balance = client.eth_balance(address).await?.low_u64() as f64;
            println!("Balance:  {:.2} ETH", balance / 1_000_000_000_000_000_000.0);
        }
        Command::Balance(args) => {
            let owner = args.address.unwrap_or(poly.wallet.to_eth_address());
            let balance: f64 = (poly.balance(owner).await.unwrap().low_u64() as f64) / 1_000_000.0;
            println!();
            println!("Balance:  {balance:.2} USDC");
            println!();
        }
        Command::Transfer(args) => {
            let value = (args.value * 1_000_000f64) as u128;
            let to = args.to;
            poly.transfer_usdc(value, args.to).await?;
            println!("Transfering {value} USDC to 0x{to:x}");
        }
        Command::DecodePayload(args) => {
            let payload = args.payload.trim_start_matches("https://payy.link/s/#");
            let payload = zk_primitives::decode_activity_url_payload(payload);
            println!("{}", serde_json::to_string_pretty(&payload)?);
            eprintln!("Address: {}", payload.address().to_hex());
            println!("PSI: {}", payload.psi().to_hex());
            println!("Commitment: {}", payload.commitment().to_hex());
        }
        Command::EncodePayload => {
            let payload = serde_json::from_reader::<_, NoteURLPayload>(std::io::stdin())?;
            let encoded = payload.encode_activity_url_payload();
            println!("{encoded}");
        }
        Command::Halo2Address(args) => {
            let pk = Element::from_str(&args.private_key).unwrap();
            let old_address = hash_poseidon::hash_merge([pk, Element::ZERO]);
            let new_address = get_address_for_private_key(pk);

            println!("Halo2 Address: {}", old_address.to_hex());
            println!("Noir Address: {}", new_address.to_hex());
        }
        Command::Commitment(args) => {
            let address = Element::from_str(&args.address).unwrap();
            let psi = Element::from_str(&args.psi).unwrap();
            let value = Element::new(args.value);

            let note = Note::new_with_psi(address, value, psi, bridged_polygon_usdc_note_kind());
            println!("Commitment: {}", note.commitment().to_hex());
        }
    }

    Ok(())
}
