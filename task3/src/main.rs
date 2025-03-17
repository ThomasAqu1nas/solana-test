/*
Проблема:
Error: Status { 
    code: Unauthenticated, 
    message: "client IP not whitelisted", 
    metadata: MetadataMap { 
        headers: {
            "date": "Mon, 17 Mar 2025 16:30:07 GMT", 
            "content-type": "application/grpc", 
            "content-length": "0", 
            "strict-transport-security": "max-age=31536000; 
            includeSubDomains"
        } 
    }, 
    source: None 
}
    получилось организовать хэндшейк с сервером, 
    но моего айпи нет в вайтлисте, поэтому затестить
    не представляется возможным
*/


mod solana;
pub mod geyser {
    tonic::include_proto!("geyser");
}

use futures::FutureExt;
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint};
use tonic::{Request, Status, service::Interceptor};
use tonic::metadata::{MetadataKey, MetadataValue};

use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use config::Config;

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

#[derive(serde::Deserialize, Debug, Clone)]
struct AppConfig {
    recipient_address: String,
    rpc_endpoint: String,
    transfer_amount_sol: f64,
    max_concurrent_transfers: usize,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let api_key = std::env::var("GEYSER_API_KEY")
        .expect("GEYSER_API_KEY not set in .env file");

    let config: AppConfig = Config::builder()
        .add_source(config::File::with_name("config.yaml"))
        .build()?
        .try_deserialize()?;

    println!("Config loaded successfully: {:?}", config);

    let pem = tokio::fs::read("server.pem").await?;
    let ca_cert = Certificate::from_pem(pem);
    let tls_config = ClientTlsConfig::new()
        .ca_certificate(ca_cert)
        .domain_name("grpc.ny.shyft.to");


    let endpoint = Endpoint::from_static("https://grpc.ny.shyft.to").tls_config(tls_config)?;
    let channel = endpoint.connect().await?;
    let interceptor = AuthInterceptor { api_key };
    let mut client = geyser::geyser_client::GeyserClient::with_interceptor(channel, interceptor);

    let mut subscribe_req = geyser::SubscribeRequest::default();
    subscribe_req.blocks.insert(
        "subscribe".to_string(),
        geyser::SubscribeRequestFilterBlocks::default(),
    );

    let request_stream = tokio_stream::iter(vec![subscribe_req]);
    let mut response_stream = client.subscribe(request_stream).await?.into_inner();

    println!("Subscribed to Geyser stream. Waiting for blocks...");

    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_transfers));
    let solana_rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
        config.rpc_endpoint.clone()
    ));
    let shared_config = Arc::new(config);

    while let Some(update) = response_stream.message().await? {
        if let Some(geyser::subscribe_update::UpdateOneof::Block(block)) = update.update_oneof {
            println!("New block received, slot: {}", block.slot);

            let config = Arc::clone(&shared_config);
            let permit = Arc::clone(&semaphore).acquire_owned().await?;
            let connection = Arc::clone(&solana_rpc_client);
            tokio::spawn(async move {
                solana::send_sol_transfer(
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