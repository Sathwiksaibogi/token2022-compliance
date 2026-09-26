pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;
use spl_discriminator::SplDiscriminate;
use state::AuthorizationStatus;

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
        instructions::initialize_policy::initialize_policy_handler(
            ctx,
            max_transfer_amount,
            daily_transfer_limit,
            expires_at,
        )
    }

    pub fn initialize_authorization(
        ctx: Context<InitializeAuthorization>,
        wallet: Pubkey,
    ) -> Result<()> {
        instructions::initialize_authorization::initialize_authorization_handler(ctx, wallet)
    }

    pub fn set_authorization_status(
        ctx: Context<SetAuthorizationStatus>,
        new_status: AuthorizationStatus,
    ) -> Result<()> {
        instructions::set_authorization_status::set_authorization_status_handler(ctx, new_status)
    }

    pub fn initialize_transfer_stats(
        ctx: Context<InitializeTransferStats>,
        wallet: Pubkey,
    ) -> Result<()> {
        instructions::initialize_transfer_stats::initialize_transfer_stats_handler(ctx, wallet)
    }

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        instructions::initialize_extra_account_meta_list::initialize_extra_account_meta_list_handler(
            ctx,
        )
    }

    #[instruction(
        discriminator = spl_transfer_hook_interface::instruction::ExecuteInstruction::SPL_DISCRIMINATOR_SLICE
    )]
    pub fn execute(ctx: Context<ExecuteTransferHook>, amount: u64) -> Result<()> {
        instructions::execute_transfer_hook::execute_transfer_hook_handler(ctx, amount)
    }

    pub fn update_policy(ctx: Context<UpdatePolicy>, args: UpdatePolicyArgs) -> Result<()> {
        instructions::update_policy::update_policy_handler(ctx, args)
    }

    pub fn update_transfer_fee(
        ctx: Context<UpdateTransferFee>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        instructions::update_transfer_fee::update_transfer_fee_handler(
            ctx,
            transfer_fee_basis_points,
            maximum_fee,
        )
    }
}
