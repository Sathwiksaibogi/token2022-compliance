use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

use crate::error::ComplianceError;
use crate::state::GlobalPolicy;

#[derive(Accounts)]
pub struct InitializePolicy<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        owner=token_program.key(),
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer=admin,
        space=8+GlobalPolicy::INIT_SPACE,
        seeds=[b"policy",mint.key().as_ref()],
        bump,
    )]
    pub global_policy: Account<'info, GlobalPolicy>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_policy_handler(
    ctx: Context<InitializePolicy>,
    max_transfer_amount: u64,
    daily_transfer_limit: u64,
    expires_at: i64,
) -> Result<()> {
    require!(
        max_transfer_amount > 0,
        ComplianceError::InvalidMaxTransferAmount
    );
    require!(
        daily_transfer_limit >= max_transfer_amount,
        ComplianceError::InvalidDailyLimit
    );
    let clock = Clock::get()?;
    require!(
        expires_at == 0 || expires_at > clock.unix_timestamp,
        ComplianceError::InvalidExpiration
    );
    ctx.accounts.global_policy.admin = ctx.accounts.admin.key();
    ctx.accounts.global_policy.mint = ctx.accounts.mint.key();
    ctx.accounts.global_policy.enabled = true;
    ctx.accounts.global_policy.max_transfer_amount = max_transfer_amount;
    ctx.accounts.global_policy.daily_transfer_limit = daily_transfer_limit;
    ctx.accounts.global_policy.policy_version = 1;
    ctx.accounts.global_policy.expires_at = expires_at;
    ctx.accounts.global_policy.bump = ctx.bumps.global_policy;
    Ok(())
}
