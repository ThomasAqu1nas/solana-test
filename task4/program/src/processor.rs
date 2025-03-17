use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
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
        let instruction = DepositInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        match instruction {
            DepositInstruction::Deposit { amount } => {
                Self::process_deposit(program_id, accounts, amount)
            }
            DepositInstruction::Withdraw { amount } => {
                Self::process_withdraw(program_id, accounts, amount)
            }
        }
    }

    fn process_deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let deposit_account_info = next_account_info(account_info_iter)?;
        let user_account_info = next_account_info(account_info_iter)?;

        if !user_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if deposit_account_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut deposit_account = if deposit_account_info.data_is_empty() {
            DepositAccount { balance: 0 }
        } else {
            DepositAccount::try_from_slice(&deposit_account_info.data.borrow())?
        };

        deposit_account.balance = deposit_account
            .balance
            .checked_add(amount)
            .ok_or(ProgramError::InvalidInstructionData)?;

        deposit_account.serialize(&mut &mut deposit_account_info.data.borrow_mut()[..])?;
        msg!("Депозит {} лампортов успешен. Новый баланс: {}", amount, deposit_account.balance);
        Ok(())
    }

    fn process_withdraw(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let deposit_account_info = next_account_info(account_info_iter)?;
        let destination_account_info = next_account_info(account_info_iter)?;

        if !destination_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if deposit_account_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut deposit_account = DepositAccount::try_from_slice(&deposit_account_info.data.borrow())?;

        if deposit_account.balance < amount {
            return Err(ProgramError::InsufficientFunds);
        }
        deposit_account.balance -= amount;

        deposit_account.serialize(&mut &mut deposit_account_info.data.borrow_mut()[..])?;

        **deposit_account_info.try_borrow_mut_lamports()? = deposit_account_info
            .lamports()
            .checked_sub(amount)
            .ok_or(ProgramError::InsufficientFunds)?;
        **destination_account_info.try_borrow_mut_lamports()? = destination_account_info
            .lamports()
            .checked_add(amount)
            .ok_or(ProgramError::InvalidInstructionData)?;
        msg!("Вывод {} лампортов успешен. Новый баланс: {}", amount, deposit_account.balance);
        Ok(())
    }
}
