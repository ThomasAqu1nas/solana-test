pub mod processor;
pub mod state;
pub mod instruction;

#[cfg(feature = "no-entrypoint")]
mod entrypoint;

#[cfg(test)]
mod tests {
    use crate::instruction::DepositInstruction;
    use crate::state::DepositAccount;
    use borsh::{BorshDeserialize, BorshSerialize};
    use solana_program::clock::Epoch;
    use solana_program::account_info::AccountInfo;
    use solana_program::pubkey::Pubkey;

    fn create_account_info<'a>(
        key: &'a Pubkey,
        owner: &'a Pubkey,
        lamports: u64,
        data_len: usize,
        is_signer: bool,
    ) -> AccountInfo<'a> {
        let lamports_box = Box::new(lamports);
        let lamports_ref: &'a mut u64 = Box::leak(lamports_box);
    
        let data = vec![0_u8; data_len].into_boxed_slice();
        let data_ref: &'a mut [u8] = Box::leak(data);
    
        AccountInfo::new(
            key,
            is_signer,
            true,
            lamports_ref,
            data_ref,
            owner,
            false,
            Epoch::default(),
        )
    }
    

    #[test]
    fn test_process_deposit() {
        let program_id = Pubkey::new_unique();
        let deposit_account_key = Pubkey::new_unique();
        let user_account_key = Pubkey::new_unique();
        let deposit_account = create_account_info(&deposit_account_key, &program_id, 1000, 8, false);
        let binding = Pubkey::new_unique();
        let user_account = create_account_info(&user_account_key, &binding, 5000, 0, true);

        let instruction = DepositInstruction::Deposit { amount: 200 };
        let mut instruction_data = vec![];
        instruction.serialize(&mut instruction_data).unwrap();

        let accounts = vec![deposit_account.clone(), user_account.clone()];
        let result = crate::processor::Processor::process_instruction(&program_id, &accounts, &instruction_data);
        assert!(result.is_ok());

        let deposit_state = DepositAccount::try_from_slice(&deposit_account.data.borrow()).unwrap();
        assert_eq!(deposit_state.balance, 200);
    }

    #[test]
    fn test_process_withdraw_insufficient_balance() {
        let program_id = Pubkey::new_unique();
        let deposit_account_key = Pubkey::new_unique();
        let destination_account_key = Pubkey::new_unique();
        let deposit_account = create_account_info(&deposit_account_key, &program_id, 1000, 8, false);
        let binding = Pubkey::new_unique();
        let destination_account = create_account_info(&destination_account_key, &binding, 5000, 0, true);

        let deposit_state = DepositAccount { balance: 100 };
        deposit_state.serialize(&mut &mut deposit_account.data.borrow_mut()[..]).unwrap();

        let instruction = DepositInstruction::Withdraw { amount: 200 };
        let mut instruction_data = vec![];
        instruction.serialize(&mut instruction_data).unwrap();

        let accounts = vec![deposit_account.clone(), destination_account.clone()];
        let result = crate::processor::Processor::process_instruction(&program_id, &accounts, &instruction_data);
        assert!(result.is_err());
    }
}
