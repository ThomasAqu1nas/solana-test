use solana_program::program_error::ProgramError;
#[derive(Debug)]
pub enum DepositInstruction {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
    Initialize,
}

impl DepositInstruction {
    pub fn pack(&self) -> Vec<u8> {
        match self {
            DepositInstruction::Deposit { amount } => {
                let mut buf = vec![0];
                buf.extend_from_slice(&amount.to_le_bytes());
                buf
            },
            DepositInstruction::Withdraw { amount } => {
                let mut buf = vec![1];
                buf.extend_from_slice(&amount.to_le_bytes());
                buf
            },
            DepositInstruction::Initialize => vec![2]
        }
    }

    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = data.split_at(1);
        match tag[0] {
            0 => {
                let amount = u64::from_le_bytes(
                    rest.try_into().map_err(|_| ProgramError::InvalidInstructionData)?
                );
                Ok(DepositInstruction::Deposit { amount })
            },
            1 => {
                let amount = u64::from_le_bytes(
                    rest.try_into().map_err(|_| ProgramError::InvalidInstructionData)?
                );
                Ok(DepositInstruction::Withdraw { amount })
            },
            2 => Ok(DepositInstruction::Initialize),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
