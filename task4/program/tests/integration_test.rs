use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    account::Account, instruction::{AccountMeta, Instruction}, program_pack::Pack, rent::Rent, signature::{Keypair, Signer}, transaction::Transaction
};
use borsh::{BorshSerialize, BorshDeserialize};

// Импортируем наши инструкции и состояние
use program::instruction::DepositInstruction;
use program::state::DepositAccount;

#[tokio::test]
async fn test_initialize_deposit() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "program", // имя вашего смарт-контракта
        program_id,
        processor!(program::entrypoint::process_instruction),
    );

    // Создаем тестового пользователя
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

    // Вычисляем ожидаемый PDA для депозитного аккаунта: [b"deposit", user_pubkey]
    let (deposit_pda, _bump) =
        Pubkey::find_program_address(&[b"deposit", user.pubkey().as_ref()], &program_id);

    // Формируем инструкцию инициализации
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

    let (mut banks_client, _payer, recent_blockhash) = program_test.start().await;
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&user.pubkey()),
        &[&user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Загружаем состояние депозитного аккаунта и десериализуем с помощью Pack
    let deposit_account = banks_client
        .get_account(deposit_pda)
        .await
        .expect("get_account")
        .expect("deposit account not found");

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
        processor!(program::entrypoint::process_instruction),
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

    // Формируем инструкцию депозита: перевод 500 lamports
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
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user.pubkey()),
        &[&user],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    // Читаем и десериализуем состояние аккаунта с помощью unpack_from_slice
    let deposit_account = banks_client
        .get_account(deposit_pda)
        .await
        .expect("get_account")
        .expect("deposit account not found");
    let deposit_state = DepositAccount::unpack_from_slice(&deposit_account.data)
        .expect("failed to deserialize state");

    assert_eq!(deposit_state.balance, deposit_amount);
}

#[tokio::test]
async fn test_withdraw() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "program",
        program_id,
        processor!(program::entrypoint::process_instruction),
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
    program_test.add_account(
        deposit_pda,
        Account {
            lamports: 1_000_000,
            data: deposit_data,
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );

    let withdraw_amount = 300;
    let withdraw_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(deposit_pda, false),
            AccountMeta::new(user.pubkey(), true),
        ],
        data: DepositInstruction::Withdraw { amount: withdraw_amount }.pack(),
    };

    let (mut banks_client, _payer, recent_blockhash) = program_test.start().await;
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
}
