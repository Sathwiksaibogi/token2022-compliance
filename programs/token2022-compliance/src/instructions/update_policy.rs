use anchor_lang::prelude::*;

use anchor_spl::{token_2022::Token2022, token_interface::Mint};

use crate::{error::ComplianceError, state::GlobalPolicy};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdatePolicyArgs {
    pub enabled: Option<bool>,
    pub max_transfer_amount: Option<u64>,
    pub daily_transfer_limit: Option<u64>,
    pub expires_at: Option<i64>,
}

#[derive(Accounts)]
pub struct UpdatePolicy<'info> {
    pub admin: Signer<'info>,

    #[account(
        owner = token_program.key(),
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [
            b"policy",
            mint.key().as_ref(),
        ],
        bump = global_policy.bump,
        has_one = admin,
        has_one = mint,
    )]
    pub global_policy: Account<'info, GlobalPolicy>,

    pub token_program: Program<'info, Token2022>,
}

pub fn update_policy_handler(ctx: Context<UpdatePolicy>, args: UpdatePolicyArgs) -> Result<()> {
    let clock = Clock::get()?;

    let policy = &mut ctx.accounts.global_policy;

    // ---------------------------------------------
    // 1. Build the candidate final policy
    // ---------------------------------------------

    let new_enabled = args.enabled.unwrap_or(policy.enabled);

    let new_max_transfer_amount = args
        .max_transfer_amount
        .unwrap_or(policy.max_transfer_amount);

    let new_daily_transfer_limit = args
        .daily_transfer_limit
        .unwrap_or(policy.daily_transfer_limit);

    let new_expires_at = args.expires_at.unwrap_or(policy.expires_at);

    // ---------------------------------------------
    // 2. Validate the candidate policy
    // ---------------------------------------------

    require!(
        new_max_transfer_amount > 0,
        ComplianceError::InvalidMaxTransferAmount
    );

    require!(
        new_daily_transfer_limit >= new_max_transfer_amount,
        ComplianceError::InvalidDailyLimit
    );

    require!(
        new_expires_at == 0 || new_expires_at > clock.unix_timestamp,
        ComplianceError::InvalidExpiration
    );

    let new_policy_version = policy
        .policy_version
        .checked_add(1)
        .ok_or(ComplianceError::ArithmeticOverflow)?;

    // ---------------------------------------------
    // 3. Commit the validated policy
    // ---------------------------------------------

    policy.enabled = new_enabled;
    policy.max_transfer_amount = new_max_transfer_amount;
    policy.daily_transfer_limit = new_daily_transfer_limit;
    policy.expires_at = new_expires_at;
    policy.policy_version = new_policy_version;

    Ok(())
}
