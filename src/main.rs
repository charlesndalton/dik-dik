use types::Result;
use std::env;

const TOKEMAK_COMMITTEE_TELEGRAM_CHAT_ID: i64 = -1001175962929;

#[tokio::main]
async fn main() -> Result<()> {
    let token = env::var("DIK_DIK_TELEGRAM_TOKEN").expect("DIK_DIK_TELEGRAM_TOKEN not set");
    let infura_api_key = env::var("INFURA_API_KEY").expect("INFURA_API_KEY not set");
    let dai_strategy = "0xBD455373692c8F4bae2131d66BFfD5fE26C6b659";

    let client = blockchain_client::create_client(&infura_api_key)?;

    println!("TDAI BALANCE: {}", blockchain_client::get_t_asset_balance(&client, &dai_strategy).await?);

    Ok(())
}

async fn daily_check() -> Result<()> {
    Ok(())
}

mod blockchain_client {
    use ethers::prelude::*;
    use std::sync::Arc;
    use crate::types::Result;
    use std::env;
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
        let strategy = TokemakStrategy::new(strategy_address, Arc::clone(&client));

        let t_asset_address = strategy.t_asset().call().await?;
        let t_asset = IERC20::new(t_asset_address, Arc::clone(&client));

        let decimals: u32 = t_asset.decimals().call().await?.into();
        let mut t_asset_balance = Decimal::from_i128_with_scale(t_asset.balance_of(strategy_address).call().await?.as_u128().try_into().unwrap(), decimals);
        t_asset_balance.rescale(6);

        Ok(t_asset_balance)
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
    }

    pub type Result<T> = std::result::Result<T, Error>;
}
