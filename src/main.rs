use types::Result;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use serde_json::{Value};

#[tokio::main]
async fn main() -> Result<()> {
    let telegram_token = env::var("DIK_DIK_TELEGRAM_TOKEN").expect("DIK_DIK_TELEGRAM_TOKEN not set");
    let infura_api_key = env::var("INFURA_API_KEY").expect("INFURA_API_KEY not set");

    let mut file = File::open("./src/contract-address-registry/ethereum.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let v: Value = serde_json::from_str(&contents)?;

    let client = blockchain_client::create_client(&infura_api_key)?;

    let mut committee_report = String::from("[DAILY TOKEMAK REPORT ☢️]:
");

    for (asset, strategy_address) in v["strategies"]["tokemak"].as_object().unwrap().iter() {
        let t_asset_balance = blockchain_client::get_t_asset_balance(&client, &strategy_address.as_str().unwrap()).await?;
        let want_balance_in_pool = blockchain_client::get_liquid_want_in_pool(&client, &strategy_address.as_str().unwrap()).await?;
        let t_asset_total_supply = blockchain_client::get_t_asset_total_supply(&client, &strategy_address.as_str().unwrap()).await?;
        let mut health_ratio = want_balance_in_pool / t_asset_total_supply;
        health_ratio.rescale(2);

        committee_report.push_str(&format!(
"Report for {}
 - amount of t{} we own: {}
 - health ratio (A/L): {}
 - tokemak assets: {}
 - tokemak liabilities: {}
", asset, asset, t_asset_balance, health_ratio, want_balance_in_pool, t_asset_total_supply));
    }

    print!("{}", committee_report);
    //telegram_client::send_message_to_committee(&committee_report, &telegram_token).await?;

    Ok(())
}

async fn daily_check() -> Result<()> {
    Ok(())
}

mod telegram_client {
    use crate::types::Result;
    use urlencoding::encode;

    const TOKEMAK_COMMITTEE_TELEGRAM_CHAT_ID: i64 = -1001175962929;

    pub async fn send_message_to_committee(message: &str, token: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}", token, TOKEMAK_COMMITTEE_TELEGRAM_CHAT_ID, encode(message));

        reqwest::get(url)
            .await?;

        Ok(())
    }
}

mod blockchain_client {
    use ethers::prelude::*;
    use std::sync::Arc;
    use crate::types::Result;
    use rust_decimal::prelude::*;

    abigen!(
        IERC20,
        "./src/abis/IERC20.json",
        event_derives(serde::Deserialize, serde::Serialize)
    );

    abigen!(
        TokemakStrategy,
        "./src/abis/TokemakStrategy.json",
        event_derives(serde::Deserialize, serde::Serialize)
    );

    pub type Client = Arc<Provider::<Http>>;

    pub fn create_client(infura_api_key: &str) -> Result<Client> {
        let infura_url = format!("https://mainnet.infura.io/v3/{}", infura_api_key);
        let client = Provider::<Http>::try_from(infura_url)?;
        Ok(Arc::new(client))
    }

    pub async fn get_t_asset_balance(client: &Client, strategy_address: &str) -> Result<Decimal> {
        let strategy_address = strategy_address.parse::<Address>()?;
        let t_asset_address = get_t_asset_address(client, strategy_address).await?;
        let t_asset = IERC20::new(t_asset_address, Arc::clone(&client));
        let decimals = get_t_asset_decimals(&t_asset).await?;

        let mut t_asset_balance = Decimal::from_i128_with_scale(t_asset.balance_of(strategy_address).call().await?.as_u128().try_into().unwrap(), decimals);
        t_asset_balance.rescale(6);

        Ok(t_asset_balance)
    }

    pub async fn get_t_asset_total_supply(client: &Client, strategy_address: &str) -> Result<Decimal> {
        let strategy_address = strategy_address.parse::<Address>()?;
        let t_asset_address = get_t_asset_address(client, strategy_address).await?;
        let t_asset = IERC20::new(t_asset_address, Arc::clone(&client));
        let decimals = get_t_asset_decimals(&t_asset).await?;

        let mut t_asset_total_supply = Decimal::from_i128_with_scale(t_asset.total_supply().call().await?.as_u128().try_into().unwrap(), decimals);
        t_asset_total_supply.rescale(6);

        Ok(t_asset_total_supply)
    }
        
    pub async fn get_liquid_want_in_pool(client: &Client, strategy_address: &str) -> Result<Decimal> {
        let strategy_address = strategy_address.parse::<Address>()?;
        let t_asset_address = get_t_asset_address(client, strategy_address).await?;

        let want_address = TokemakStrategy::new(strategy_address, Arc::clone(&client)).want().call().await?;
        let want = IERC20::new(want_address, Arc::clone(&client));

        let want_decimals = want.decimals().call().await?.into();
        let mut want_pool_balance = Decimal::from_i128_with_scale(want.balance_of(t_asset_address).call().await?.as_u128().try_into().unwrap(), want_decimals); // t asset address is same as liquidity pool address
        want_pool_balance.rescale(6);

        Ok(want_pool_balance)
    }

    async fn get_t_asset_decimals(t_asset: &IERC20<Provider::<Http>>) -> Result<u32> {
        let decimals: u32 = t_asset.decimals().call().await?.into();
        Ok(decimals)
    }

    async fn get_t_asset_address(client: &Client, strategy_address: Address) -> Result<Address> {
        let weth_strategy_address = "0x2EFB43C8C9AFe71d98B3093C3FD4dEB7Ce543C6D".parse::<Address>()?;

        if strategy_address == weth_strategy_address {
            Ok("0xD3D13a578a53685B4ac36A1Bab31912D2B2A2F36".parse::<Address>()?)
        } else {
            Ok(TokemakStrategy::new(strategy_address, Arc::clone(&client)).t_asset().call().await?)
        }
    }
}

mod types {
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        EthrsContractError(#[from] ethers::contract::ContractError<ethers::providers::Provider<ethers::providers::Http>>),

        #[error(transparent)]
        UrlParseError(#[from] url::ParseError),

        #[error(transparent)]
        AddressParseStringToHexError(#[from] rustc_hex::FromHexError),

        #[error(transparent)]
        EyreReport(#[from] eyre::Report),

        #[error(transparent)]
        IOError(#[from] std::io::Error),

        #[error(transparent)]
        SerdeJsonError(#[from] serde_json::Error),

        #[error(transparent)]
        ReqwestError(#[from] reqwest::Error),
    }

    pub type Result<T> = std::result::Result<T, Error>;
}
