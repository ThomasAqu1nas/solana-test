pub mod processor;
pub mod state;
pub mod instruction;
pub mod entrypoint;

// #[cfg(test)]
// mod tests {
//     use crate::instruction::DepositInstruction;
//     use crate::processor::Processor;
//     use crate::state::DepositAccount;
//     use borsh::{BorshDeserialize, BorshSerialize};
//     use solana_program::{
//         account_info::AccountInfo, clock::Epoch, pubkey::Pubkey,
//         program_pack::Pack
//     };

//     fn create_account_info<'a>(
//         key: &'a Pubkey,
//         owner: &'a Pubkey,
//         lamports: u64,
//         data_len: usize,
//         is_signer: bool,
//     ) -> AccountInfo<'a> {
//         let lamports_box = Box::new(lamports);
//         let lamports_ref: &'a mut u64 = Box::leak(lamports_box);
    
//         let data = vec![0_u8; data_len].into_boxed_slice();
//         let data_ref: &'a mut [u8] = Box::leak(data);
    
//         AccountInfo::new(
//             key,
//             is_signer,
//             true,
//             lamports_ref,
//             data_ref,
//             owner,
//             false,
//             Epoch::default(),
//         )
//     }

//     #[test]
//     fn test_process_deposit() {
//         let program_id = Pubkey::new_unique();
//         let user_pubkey = Pubkey::new_unique();

//         let (expected_deposit_pda, _bump) =
//             Pubkey::find_program_address(&[b"deposit", user_pubkey.as_ref()], &program_id);

//         let deposit_account =
//             create_account_info(&expected_deposit_pda, &program_id, 1000, DepositAccount::LEN, false);
//         let init_state = DepositAccount {
//             owner: user_pubkey,
//             balance: 0,
//         };
//         init_state.serialize(&mut &mut deposit_account.data.borrow_mut()[..]).unwrap();

//         let binding = Pubkey::new_unique();
//         let user_account =
//             create_account_info(&user_pubkey, &binding, 5000, 0, true);

//         let instruction = DepositInstruction::Deposit { amount: 200 };
//         let mut instruction_data = vec![];
//         instruction.serialize(&mut instruction_data).unwrap();

//         let accounts = vec![deposit_account.clone(), user_account.clone()];

//         let result = Processor::process_instruction(&program_id, &accounts, &instruction_data);
//         assert!(result.is_ok(), "Депозит должен пройти успешно");

//         let deposit_state_after =
//             DepositAccount::try_from_slice(&deposit_account.data.borrow()).unwrap();
//         assert_eq!(deposit_state_after.balance, 200);
//     }

//     #[test]
//     fn test_process_withdraw_insufficient_balance() {
//         let program_id = Pubkey::new_unique();
//         let user_pubkey = Pubkey::new_unique();

//         let (expected_deposit_pda, _bump) =
//             Pubkey::find_program_address(&[b"deposit", user_pubkey.as_ref()], &program_id);

//         let deposit_account =
//             create_account_info(&expected_deposit_pda, &program_id, 1000, DepositAccount::LEN, false);
//         let init_state = DepositAccount {
//             owner: user_pubkey,
//             balance: 100,
//         };
//         init_state.serialize(&mut &mut deposit_account.data.borrow_mut()[..]).unwrap();

//         let binding = Pubkey::new_unique();
//         let destination_account =
//             create_account_info(&user_pubkey, &binding, 5000, 0, true);

//         let instruction = DepositInstruction::Withdraw { amount: 200 };
//         let mut instruction_data = vec![];
//         instruction.serialize(&mut instruction_data).unwrap();

//         let accounts = vec![deposit_account.clone(), destination_account.clone()];

//         let result = Processor::process_instruction(&program_id, &accounts, &instruction_data);
//         assert!(result.is_err(), "Вывод должен завершиться ошибкой при недостатке средств");
//     }

//     #[test]
//     fn test_process_withdraw_success() {
//         let program_id = Pubkey::new_unique();
//         let user_pubkey = Pubkey::new_unique();

//         let (expected_deposit_pda, _bump) =
//             Pubkey::find_program_address(&[b"deposit", user_pubkey.as_ref()], &program_id);

//         let deposit_account =
//             create_account_info(&expected_deposit_pda, &program_id, 2000, DepositAccount::LEN, false);
//         let init_state = DepositAccount {
//             owner: user_pubkey,
//             balance: 500,
//         };
//         init_state.serialize(&mut &mut deposit_account.data.borrow_mut()[..]).unwrap();

//         let binding = Pubkey::new_unique();
//         let destination_account =
//            	create_account_info(&user_pubkey, &binding, 5000, 0, true);

//         let instruction = DepositInstruction::Withdraw { amount: 300 };
//         let mut instruction_data = vec![];
//         instruction.serialize(&mut instruction_data).unwrap();

//         let accounts = vec![deposit_account.clone(), destination_account.clone()];

//         let result = Processor::process_instruction(&program_id, &accounts, &instruction_data);
//         assert!(result.is_ok(), "Вывод должен пройти успешно");

//         let deposit_state_after =
//             DepositAccount::try_from_slice(&deposit_account.data.borrow()).unwrap();
//         assert_eq!(deposit_state_after.balance, 200);
//     }
// }
