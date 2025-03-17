use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signer::{keypair::Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    pubkey::Pubkey,
};
use std::{str::FromStr, sync::Arc};
use anyhow::Result;

pub mod storage {
    pub mod confirmed_block {
        tonic::include_proto!("solana.storage.confirmed_block");
    }
}

pub async fn send_sol_transfer(
    rpc_client: Arc<RpcClient>,
    recipient: &str,
    amount_sol: f64,
) -> Result<()> {
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