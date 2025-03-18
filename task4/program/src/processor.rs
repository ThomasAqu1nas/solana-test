use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
    program_pack::Pack
};

use crate::instruction::DepositInstruction;
use crate::state::DepositAccount;
use borsh::{BorshDeserialize, BorshSerialize};

pub struct Processor;
impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = DepositInstruction::unpack(instruction_data)?;
        match instruction {
            DepositInstruction::Deposit { amount } => {
                Self::process_deposit(program_id, accounts, amount)
            }
            DepositInstruction::Withdraw { amount } => {
                Self::process_withdraw(program_id, accounts, amount)
            },
            DepositInstruction::Initialize => {
                Self::process_initialize(program_id, accounts)
            }
        }
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let deposit_account_info = next_account_info(account_info_iter)?;
        let user_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;

        let (expected_deposit_pda, bump) = Pubkey::find_program_address(
            &[b"deposit", user_account_info.key.as_ref()],
            program_id,
        );
        if expected_deposit_pda != *deposit_account_info.key {
            msg!("Неверный адрес депозитного аккаунта");
            return Err(ProgramError::InvalidAccountData);
        }

        // Если аккаунт уже инициализирован - ошибка
        if !deposit_account_info.data_is_empty() {
            msg!("Депозитный аккаунт уже инициализирован");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let rent = Rent::from_account_info(rent_sysvar_info)?;
        let required_lamports = rent.minimum_balance(DepositAccount::LEN);

        let create_account_ix = solana_program::system_instruction::create_account(
            user_account_info.key,              // плательщик
            deposit_account_info.key,           // новый аккаунт
            required_lamports,
            DepositAccount::LEN as u64,
            program_id,
        );

        let seeds = &[b"deposit", user_account_info.key.as_ref(), &[bump]];
        solana_program::program::invoke_signed(
            &create_account_ix,
            &[
                user_account_info.clone(),
                deposit_account_info.clone(),
                system_program_info.clone(),
            ],
            &[seeds],
        )?;

        let deposit_state = DepositAccount {
            owner: *user_account_info.key,
            balance: 0,
        };
        deposit_state.serialize(&mut &mut deposit_account_info.data.borrow_mut()[..])?;
        msg!("Депозитный аккаунт инициализирован для {}", user_account_info.key);
        Ok(())
    }

    /// 0. [Writable] Депозитный аккаунт (PDA)
    /// 1. [Signer, Writable] Пользователь (owner)
    fn process_deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let deposit_account_info = next_account_info(account_info_iter)?;
        let user_account_info = next_account_info(account_info_iter)?;
        // Получаем аккаунт системной программы
        let system_program_info = next_account_info(account_info_iter)?;
    
        if !user_account_info.is_signer {
            msg!("Подпись пользователя обязательна");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if deposit_account_info.owner != program_id {
            msg!("Неверный владелец депозитного аккаунта");
            return Err(ProgramError::IncorrectProgramId);
        }
    
        let (expected_deposit_pda, _bump) = Pubkey::find_program_address(
            &[b"deposit", user_account_info.key.as_ref()],
            program_id,
        );
        if expected_deposit_pda != *deposit_account_info.key {
            msg!("Депозитный аккаунт не соответствует ожидаемому PDA");
            return Err(ProgramError::InvalidAccountData);
        }
    
        let mut deposit_state = DepositAccount::unpack_from_slice(&deposit_account_info.data.borrow())?;
        if deposit_state.owner != *user_account_info.key {
            msg!("Пользователь не является владельцем депозитного аккаунта");
            return Err(ProgramError::IllegalOwner);
        }
    
        // Формируем системную инструкцию перевода lamports
        let transfer_ix = solana_program::system_instruction::transfer(
            user_account_info.key,
            deposit_account_info.key,
            amount,
        );
        // Передаем системную программу в качестве третьего аккаунта для CPI
        solana_program::program::invoke(
            &transfer_ix,
            &[
                user_account_info.clone(),
                deposit_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    
        deposit_state.balance = deposit_state
            .balance
            .checked_add(amount)
            .ok_or(ProgramError::InvalidInstructionData)?;
    
        deposit_state.pack_into_slice(&mut deposit_account_info.data.borrow_mut());
        msg!("Депозит {} lamports успешен. Новый баланс: {}", amount, deposit_state.balance);
        Ok(())
    }
    

    /// 0. [Writable] Депозитный аккаунт (PDA)
    /// 1. [Signer, Writable] Счёт получателя (должен совпадать с owner депозитного аккаунта)
    fn process_withdraw(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let deposit_account_info = next_account_info(account_info_iter)?;
        let destination_account_info = next_account_info(account_info_iter)?;

        if !destination_account_info.is_signer {
            msg!("Подпись получателя обязательна");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if deposit_account_info.owner != program_id {
            msg!("Неверный владелец депозитного аккаунта");
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut deposit_state = DepositAccount::try_from_slice(&deposit_account_info.data.borrow())?;
        if deposit_state.owner != *destination_account_info.key {
            msg!("Только владелец депозитного аккаунта может выводить средства");
            return Err(ProgramError::IllegalOwner);
        }
        if deposit_state.balance < amount {
            msg!("Недостаточный баланс в депозитном аккаунте");
            return Err(ProgramError::InsufficientFunds);
        }

        deposit_state.balance -= amount;
        deposit_state.serialize(&mut &mut deposit_account_info.data.borrow_mut()[..])?;

        let deposit_lamports = **deposit_account_info.try_borrow_mut_lamports()?;
        if deposit_lamports < amount {
            msg!("Фактический баланс lamports меньше требуемой суммы");
            return Err(ProgramError::InsufficientFunds);
        }
        **deposit_account_info.try_borrow_mut_lamports()? -= amount;
        **destination_account_info.try_borrow_mut_lamports()? += amount;
        msg!("Вывод {} lamports успешен. Новый баланс: {}", amount, deposit_state.balance);
        Ok(())
    }
}
