pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

use anchor_lang::prelude::*;

declare_id!("Bzf35ZWCmtpzrYxQRd3y6x73NDHASarXkxK3RmFsmsW6");

#[program]
pub mod token2022_compliance {
    use super::*;

    pub fn initialize_policy(
        ctx: Context<InitializePolicy>,
        max_transfer_amount: u64,
        daily_transfer_limit: u64,
        expires_at: i64,
    ) -> Result<()> {
        instructions::initialize_policy::handler(
            ctx,
            max_transfer_amount,
            daily_transfer_limit,
            expires_at,
        )
    }
}
