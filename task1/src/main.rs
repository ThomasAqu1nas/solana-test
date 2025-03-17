use std::{error::Error, sync::Arc};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::Deserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::Semaphore;

#[derive(Debug, Deserialize)]
struct PukeysConfig {
    pub public_keys: Vec<String>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = config::Config::builder()
        .add_source(config::File::with_name("/home/ando/documents/other/test/task1/config.yaml"))
        .build()?;
    let config: PukeysConfig = config.try_deserialize()?;

    let rpc_url = "https://api.mainnet-beta.solana.com";
    let client = Arc::new(RpcClient::new(rpc_url.to_string()));

    let wallets: Vec<Pubkey> = config.public_keys
        .iter()
        .map(|s| Pubkey::from_str_const(s))
        .collect();

    let semaphore = Arc::new(Semaphore::new(5));
    let mut tasks = FuturesUnordered::new();

    for wallet in wallets.into_iter() {
        let client = Arc::clone(&client);
        let sem = Arc::clone(&semaphore);
        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await;
            let result = client.get_balance(&wallet).await;
            (wallet, result)
        }));
    }

    while let Some(join_result) = tasks.next().await {
        match join_result {
            Ok((wallet, Ok(balance))) => println!("Wallet {}: balance: {} lamports", wallet, balance),
            Ok((wallet, Err(e))) => eprintln!("Error for wallet {}: {:?}", wallet, e),
            Err(e) => eprintln!("Task join error: {:?}", e),
        }
    }

    Ok(())
}
