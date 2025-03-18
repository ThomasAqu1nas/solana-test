use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::{Pack, Sealed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DepositAccount {
    pub owner: Pubkey,
    pub balance: u64,
}

impl Sealed for DepositAccount {}

impl Pack for DepositAccount {
    const LEN: usize = 40;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = arrayref::array_mut_ref![dst, 0, 40];
        let DepositAccount {
            owner, 
            balance
        } = self;
        let (
            owner_dst, balance_dst
        ) = arrayref::mut_array_refs![dst, 32, 8];
        owner_dst.copy_from_slice(owner.as_array());
        balance_dst.copy_from_slice(&balance.to_le_bytes());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = arrayref::array_ref![src, 0, 40];
        let (owner, balance) = arrayref::array_refs![src, 32, 8];
        let (owner, balance) = (
            Pubkey::new_from_array(*owner),
            u64::from_le_bytes(*balance),
        );

        Ok(Self { 
            owner, balance
        })
    }
}
