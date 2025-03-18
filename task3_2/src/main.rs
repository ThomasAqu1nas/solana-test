use std::sync::Arc;
use std::str::FromStr;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction, transaction::Transaction};
use tokio::sync::Semaphore;
use futures::FutureExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use tonic::{metadata::{MetadataKey, MetadataValue}, transport::{Certificate, ClientTlsConfig, Endpoint}, Request, Status};
use config::Config;
use yellowstone_grpc_client::Interceptor;
use yellowstone_grpc_proto::geyser::{geyser_client::GeyserClient, subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestFilterBlocks};

#[derive(serde::Deserialize, Debug, Clone)]
struct AppConfig {
    recipient_address: String,
    rpc_endpoint: String,
    transfer_amount_sol: f64,
    max_concurrent_transfers: usize,
}

#[derive(Clone)]
struct AuthInterceptor {
    api_key: String,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        req.metadata_mut().insert(
            MetadataKey::from_static("x-api-key"),
            MetadataValue::from_str(&self.api_key)
                .map_err(|_| Status::invalid_argument("Invalid API key"))?,
        );
        Ok(req)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let api_key = std::env::var("GEYSER_API_KEY")
        .expect("GEYSER_API_KEY not set in .env file");
    let config: AppConfig = Config::builder()
        .add_source(config::File::with_name("config.yaml"))
        .build()?
        .try_deserialize()?;
    println!("Config loaded: {:?}", config);

    let pem = tokio::fs::read("server.pem").await?;
    let ca_cert = Certificate::from_pem(pem);
    let tls_config = ClientTlsConfig::new()
        .ca_certificate(ca_cert)
        .domain_name("grpc.ny.shyft.to");

    let endpoint = Endpoint::from_static("https://grpc.ny.shyft.to").tls_config(tls_config)?;
    let channel = endpoint.connect().await?;
    let interceptor = AuthInterceptor { api_key };
    let mut geyser_client = GeyserClient::with_interceptor(channel, interceptor);

    let mut subscribe_req = SubscribeRequest::default();
    subscribe_req.blocks.insert(
        "subscribe".to_string(),
        SubscribeRequestFilterBlocks::default(),
    );
    let request_stream = tokio_stream::iter(vec![subscribe_req]);
    let mut response_stream = geyser_client.subscribe(request_stream).await?.into_inner();

    println!("Subscribed to Yellowstone Geyser stream. Waiting for blocks...");

    let solana_rpc_client = Arc::new(RpcClient::new(config.rpc_endpoint.clone()));
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_transfers));
    let shared_config = Arc::new(config);

    while let Some(update) = response_stream.message().await? {
        if let Some(UpdateOneof::Block(block)) = update.update_oneof {
            println!("New block received, slot: {}", block.slot);

            let config = Arc::clone(&shared_config);
            let permit = Arc::clone(&semaphore).acquire_owned().await?;
            let connection = Arc::clone(&solana_rpc_client);
            tokio::spawn(async move {
                send_sol_transfer(
                    connection,
                    &config.recipient_address,
                    config.transfer_amount_sol,
                )
                .map(move |result| {
                    match result {
                        Ok(_) => println!("SOL transfer successful for slot {}", block.slot),
                        Err(e) => eprintln!("Failed SOL transfer for slot {}: {:?}", block.slot, e),
                    }
                    drop(permit);
                })
                .await;
            });
        } else {
            println!("Received non-block update");
        }
    }

    Ok(())
}


pub async fn send_sol_transfer(
    rpc_client: Arc<RpcClient>,
    recipient: &str,
    amount_sol: f64,
) -> anyhow::Result<()> {
    let sender_keypair = Keypair::from_bytes(&[
        245, 16, 4, 124, 237, 134, 72, 220, 123, 111, 12, 122, 59, 100, 150, 134, 192,
        139, 154, 10, 65, 247, 116, 72, 185, 90, 103, 172, 54, 190, 29, 92, 58, 31,
        249, 24, 193, 207, 28, 190, 197, 31, 72, 216, 147, 0, 154, 43, 158, 17, 148,
        199, 33, 243, 87, 203, 80, 150, 36, 168, 27, 249, 178, 253
    ])?;

    let recipient_pubkey = Pubkey::from_str(recipient)?;
    let lamports = (amount_sol * solana_sdk::native_token::LAMPORTS_PER_SOL as f64) as u64;

    let recent_blockhash = rpc_client.get_latest_blockhash()
        .await?;
    let ix = system_instruction::transfer(&sender_keypair.pubkey(), &recipient_pubkey, lamports);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        recent_blockhash,
    );

    rpc_client.send_and_confirm_transaction(&tx).await?;

    Ok(())
}