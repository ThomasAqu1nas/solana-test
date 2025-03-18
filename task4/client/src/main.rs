use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

use program::instruction::DepositInstruction;

fn main() {
    let rpc_url = "http://localhost:8899";
    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    let payer = Keypair::from_bytes(&[
        245, 16, 4, 124, 237, 134, 72, 220, 123, 111, 12, 122, 59, 100, 150, 134, 192,
        139, 154, 10, 65, 247, 116, 72, 185, 90, 103, 172, 54, 190, 29, 92, 58, 31,
        249, 24, 193, 207, 28, 190, 197, 31, 72, 216, 147, 0, 154, 43, 158, 17, 148,
        199, 33, 243, 87, 203, 80, 150, 36, 168, 27, 249, 178, 253
    ]).expect("Не удалось импортировать ключ");
    println!("Payer: {}", payer.pubkey());

    // Запрашиваем аирдроп для аккаунта payer, если его баланс равен 0
    let balance = client.get_balance(&payer.pubkey()).unwrap();
    if balance == 0 {
        let airdrop_amount = 1_000_000_000; // 1 SOL = 1_000_000_000 lamports
        let signature = client.request_airdrop(&payer.pubkey(), airdrop_amount).unwrap();
        client.confirm_transaction(&signature).unwrap();
        println!("Получен аирдроп: {} lamports", airdrop_amount);
    }

    let program_id = Pubkey::from_str("F1N6jUWGC1VYYUArJXcE9w1rrshJZusrrpnDsTiHeLLD").unwrap();

    let (deposit_pda, _bump) = Pubkey::find_program_address(&[b"deposit", payer.pubkey().as_ref()], &program_id);
    println!("Deposit PDA: {}", deposit_pda);

    if client.get_account(&deposit_pda).is_err() {
        let init_instruction = DepositInstruction::Initialize;
        let init_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(deposit_pda, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            ],
            data: init_instruction.pack(),
        };

        let recent_blockhash = client.get_latest_blockhash().unwrap();
        let init_tx = Transaction::new_signed_with_payer(
            &[init_ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        let init_result = client.send_and_confirm_transaction(&init_tx);
        println!("Initialize deposit account result: {:?}", init_result);
    } else {
        println!("Deposit account уже существует.");
    }

    let deposit_amount: u64 = 500;
    let deposit_instruction = DepositInstruction::Deposit { amount: deposit_amount };
    let deposit_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::ID, false), // системная программа для CPI
        ],
        data: deposit_instruction.pack(),
    };
    let recent_blockhash = client.get_latest_blockhash().unwrap();
    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let deposit_result = client.send_and_confirm_transaction(&deposit_tx);
    println!("Deposit result: {:?}", deposit_result);

    let withdraw_amount: u64 = 200;
    let withdraw_instruction = DepositInstruction::Withdraw { amount: withdraw_amount };
    let withdraw_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_pda, false),
            AccountMeta::new(payer.pubkey(), true),
        ],
        data: withdraw_instruction.pack(),
    };
    let recent_blockhash = client.get_latest_blockhash().unwrap();
    let withdraw_tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let withdraw_result = client.send_and_confirm_transaction(&withdraw_tx);
    println!("Withdraw result: {:?}", withdraw_result);
}
