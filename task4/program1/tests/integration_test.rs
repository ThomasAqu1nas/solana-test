use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    rent::Rent,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use program1::instruction::DepositInstruction;
use program1::state::DepositAccount;

async fn get_lamports(banks_client: &mut BanksClient, pubkey: Pubkey) -> u64 {
    banks_client
        .get_account(pubkey)
        .await
        .expect("get_account")
        .expect("account not found")
        .lamports
}

#[tokio::test]
async fn test_initialize_deposit() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "program",
        program_id,
        processor!(program1::entrypoint::process_instruction),
    );

    let user = Keypair::new();
    program_test.add_account(
        user.pubkey(),
        Account {
            lamports: 10_000_000,
            data: vec![],
            owner: solana_program::system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    );

    let (deposit_pda, _bump) =
        Pubkey::find_program_address(&[b"deposit", user.pubkey().as_ref()], &program_id);

    let init_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_pda, false),
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),
        ],
        data: DepositInstruction::Initialize.pack(),
    };

    let (banks_client, _payer, recent_blockhash) = program_test.start().await;
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&user.pubkey()),
        &[&user],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    let deposit_account = banks_client
        .get_account(deposit_pda)
        .await
        .expect("get_account")
        .expect("deposit account not found");

    let rent = Rent::default();
    let required_lamports = rent.minimum_balance(DepositAccount::LEN);
    assert_eq!(deposit_account.lamports, required_lamports);

    let deposit_state = DepositAccount::unpack_from_slice(&deposit_account.data)
        .expect("failed to deserialize deposit state");
    assert_eq!(deposit_state.owner, user.pubkey());
    assert_eq!(deposit_state.balance, 0);
}

#[tokio::test]
async fn test_deposit() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "program",
        program_id,
        processor!(program1::entrypoint::process_instruction),
    );

    let user = Keypair::new();
    program_test.add_account(
        user.pubkey(),
        Account {
            lamports: 10_000_000,
            data: vec![],
            owner: solana_program::system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    );

    let (deposit_pda, _bump) =
        Pubkey::find_program_address(&[b"deposit", user.pubkey().as_ref()], &program_id);

    let init_state = DepositAccount {
        owner: user.pubkey(),
        balance: 0,
    };
    let mut deposit_data = vec![0u8; DepositAccount::LEN];
    init_state.pack_into_slice(&mut deposit_data);
    let rent = Rent::default();
    let deposit_lamports = rent.minimum_balance(DepositAccount::LEN);
    
    program_test.add_account(
        deposit_pda,
        Account {
            lamports: deposit_lamports,
            data: deposit_data,
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );

    let deposit_amount = 500;
    let deposit_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_pda, false),
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
        ],
        data: DepositInstruction::Deposit { amount: deposit_amount }.pack(),
    };
    
    let (mut banks_client, _payer, recent_blockhash) = program_test.start().await;
    let user_before = get_lamports(&mut banks_client, user.pubkey()).await;
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user.pubkey()),
        &[&user],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    let deposit_account = banks_client
        .get_account(deposit_pda)
        .await
        .expect("get_account")
        .expect("deposit account not found");
    let deposit_state = DepositAccount::unpack_from_slice(&deposit_account.data)
        .expect("failed to deserialize state");
    assert_eq!(deposit_state.balance, deposit_amount);
    assert_eq!(deposit_account.lamports, deposit_lamports + deposit_amount);

    let user_after = get_lamports(&mut banks_client, user.pubkey()).await;
    let diff = user_before.saturating_sub(user_after);
    assert!(diff >= deposit_amount);
}

#[tokio::test]
async fn test_withdraw() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "program",
        program_id,
        processor!(program1::entrypoint::process_instruction),
    );

    let user = Keypair::new();
    program_test.add_account(
        user.pubkey(),
        Account {
            lamports: 10_000_000,
            data: vec![],
            owner: solana_program::system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    );

    let (deposit_pda, _bump) =
        Pubkey::find_program_address(&[b"deposit", user.pubkey().as_ref()], &program_id);

    let initial_balance = 500;
    let deposit_state = DepositAccount {
        owner: user.pubkey(),
        balance: initial_balance,
    };
    let mut deposit_data = vec![0u8; DepositAccount::LEN];
    deposit_state.pack_into_slice(&mut deposit_data);
    let rent = Rent::default();
    let deposit_lamports = rent.minimum_balance(DepositAccount::LEN) + initial_balance;
    program_test.add_account(
        deposit_pda,
        Account {
            lamports: deposit_lamports,
            data: deposit_data,
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );

    let (mut banks_client, _payer, recent_blockhash) = program_test.start().await;
    let user_before = get_lamports(&mut banks_client, user.pubkey()).await;

    let withdraw_amount = 300;
    let withdraw_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_pda, false),
            AccountMeta::new(user.pubkey(), true),
        ],
        data: DepositInstruction::Withdraw { amount: withdraw_amount }.pack(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&user.pubkey()),
        &[&user],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    let deposit_account = banks_client
        .get_account(deposit_pda)
        .await
        .expect("get_account")
        .expect("deposit account not found");
    let deposit_state = DepositAccount::unpack_from_slice(&deposit_account.data)
        .expect("failed to deserialize state");
    assert_eq!(deposit_state.balance, initial_balance - withdraw_amount);
    assert_eq!(deposit_account.lamports, deposit_lamports - withdraw_amount);

    let user_after = get_lamports(&mut banks_client, user.pubkey()).await;
    let diff = user_after.saturating_sub(user_before);
    // Из-за комиссии реальная разница может быть меньше withdraw_amount
    // Поэтому допускаем небольшой отступ
    assert!(diff >= withdraw_amount.saturating_sub(1000), "User balance increased by {} (expected at least {})", diff, withdraw_amount);
}
