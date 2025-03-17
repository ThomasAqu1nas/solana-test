use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    system_instruction,
};
use program::instruction::DepositInstruction;
use borsh::BorshSerialize;
use std::str::FromStr;

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

    let airdrop_sig = client.request_airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    client.confirm_transaction(&airdrop_sig).unwrap();

    let program_id = Pubkey::from_str("F1N6jUWGC1VYYUArJXcE9w1rrshJZusrrpnDsTiHeLLD").unwrap();

    // 1. Создание аккаунта депозита с дополнительными средствами.
    let deposit_account_key = Keypair::new();
    let deposit_account_pubkey = deposit_account_key.pubkey();
    let deposit_account_size = 8;

    let rent_exemption = client
        .get_minimum_balance_for_rent_exemption(deposit_account_size)
        .expect("Не удалось получить минимальный баланс для арендной платы");

    let deposit_amount = 500u64;
    let total_funding = rent_exemption + deposit_amount;

    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &deposit_account_pubkey,
        total_funding,
        deposit_account_size as u64,
        &program_id,
    );

    let recent_blockhash = client.get_latest_blockhash().unwrap();
    let create_account_tx = Transaction::new_signed_with_payer(
        &[create_account_ix],
        Some(&payer.pubkey()),
        &[&payer, &deposit_account_key],
        recent_blockhash,
    );
    let create_account_result = client.send_and_confirm_transaction(&create_account_tx);
    println!("Create deposit account result: {:?}", create_account_result);

    // 2. Выполнение инструкции депозита 
    let deposit_instruction = DepositInstruction::Deposit { amount: deposit_amount };
    let mut deposit_ix_data = vec![];
    deposit_instruction
        .serialize(&mut deposit_ix_data)
        .expect("Не удалось сериализовать инструкцию депозита");

    let deposit_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_account_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
        ],
        data: deposit_ix_data,
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

    // 3. Вывод средств.
    let withdraw_amount = 200u64;
    let withdraw_instruction = DepositInstruction::Withdraw { amount: withdraw_amount };
    let mut withdraw_ix_data = vec![];
    withdraw_instruction
        .serialize(&mut withdraw_ix_data)
        .expect("Не удалось сериализовать инструкцию вывода");

    let withdraw_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_account_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
        ],
        data: withdraw_ix_data,
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
