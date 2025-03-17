use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum DepositInstruction {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
}
