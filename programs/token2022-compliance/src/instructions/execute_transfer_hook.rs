use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::error::ComplianceError;
use crate::state::{Authorization, AuthorizationStatus, GlobalPolicy, TransferStats};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        transfer_hook::{TransferHook, TransferHookAccount},
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::{Account as Token2022Account, Mint as Token2022Mint},
};

#[derive(Accounts)]
pub struct ExecuteTransferHook<'info> {
    #[account(
        owner = Token2022::id(),
        constraint = source_token.mint == mint.key(),
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,

    #[account(
        owner = Token2022::id(),
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        owner = Token2022::id(),
        constraint = destination_token.mint == mint.key(),
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Token-2022 supplies the transfer authority as a read-only,
    /// non-signer account to the Transfer Hook. Transfer authorization has
    /// already been validated by Token-2022.
    pub transfer_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [
            b"extra-account-metas",
            mint.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Canonical Transfer Hook ExtraAccountMetaList PDA.
    /// Its address is verified using the standardized seeds.
    pub extra_account_meta_list: UncheckedAccount<'info>,

    #[account(
        seeds = [
            b"policy",
            mint.key().as_ref(),
        ],
        bump = global_policy.bump,
        has_one = mint,
    )]
    pub global_policy: Account<'info, GlobalPolicy>,

    #[account(
        seeds = [
            b"authorization",
            mint.key().as_ref(),
            source_token.owner.as_ref(),
        ],
        bump = sender_authorization.bump,
        has_one = mint,
        constraint = sender_authorization.wallet == source_token.owner,
    )]
    pub sender_authorization: Account<'info, Authorization>,

    #[account(
        seeds = [
            b"authorization",
            mint.key().as_ref(),
            destination_token.owner.as_ref(),
        ],
        bump = receiver_authorization.bump,
        has_one = mint,
        constraint = receiver_authorization.wallet == destination_token.owner,
    )]
    pub receiver_authorization: Account<'info, Authorization>,

    #[account(
        mut,
        seeds = [
            b"stats",
            mint.key().as_ref(),
            source_token.owner.as_ref(),
        ],
        bump = sender_stats.bump,
        has_one = mint,
        constraint = sender_stats.wallet == source_token.owner,
    )]
    pub sender_stats: Account<'info, TransferStats>,
}

fn verify_is_transferring(token_account_info: &AccountInfo) -> Result<()> {
    let data = token_account_info.try_borrow_data()?;
    let token_account = StateWithExtensions::<Token2022Account>::unpack(&data)?;
    let transfer_hook_account = token_account.get_extension::<TransferHookAccount>()?;
    require!(
        bool::from(transfer_hook_account.transferring),
        ComplianceError::InvalidTransferHookInvocation
    );
    Ok(())
}

fn verify_transfer_hook_program(mint_account_info: &AccountInfo) -> Result<()> {
    let data = mint_account_info.try_borrow_data()?;

    let mint = StateWithExtensions::<Token2022Mint>::unpack(&data)?;

    let transfer_hook = mint.get_extension::<TransferHook>()?;

    let hook_program_id: Option<Pubkey> = transfer_hook.program_id.into();

    require!(
        hook_program_id == Some(crate::ID),
        ComplianceError::InvalidTransferHookProgram
    );

    Ok(())
}
pub fn execute_transfer_hook_handler(ctx: Context<ExecuteTransferHook>, amount: u64) -> Result<()> {
    verify_transfer_hook_program(&ctx.accounts.mint.to_account_info())?;
    verify_is_transferring(&ctx.accounts.source_token.to_account_info())?;
    verify_is_transferring(&ctx.accounts.destination_token.to_account_info())?;

    require!(
        ctx.accounts.global_policy.enabled == true,
        ComplianceError::PolicyDisabled
    );

    let clock = Clock::get()?;
    require!(
        ctx.accounts.global_policy.expires_at == 0
            || ctx.accounts.global_policy.expires_at > clock.unix_timestamp,
        ComplianceError::PolicyExpired
    );

    match ctx.accounts.sender_authorization.status {
        AuthorizationStatus::Authorized => {}

        AuthorizationStatus::Unauthorized => {
            return err!(ComplianceError::SenderNotAuthorized);
        }

        AuthorizationStatus::Blocked => {
            return err!(ComplianceError::SenderBlocked);
        }
    }

    match ctx.accounts.receiver_authorization.status {
        AuthorizationStatus::Authorized => {}

        AuthorizationStatus::Unauthorized => {
            return err!(ComplianceError::ReceiverNotAuthorized);
        }

        AuthorizationStatus::Blocked => {
            return err!(ComplianceError::ReceiverBlocked);
        }
    }

    require!(
        amount <= ctx.accounts.global_policy.max_transfer_amount,
        ComplianceError::MaxTransferAmountExceeded
    );

    let current_day_index = clock.unix_timestamp.div_euclid(86_400);

    let amount_today = if ctx.accounts.sender_stats.day_index == current_day_index {
        ctx.accounts.sender_stats.amount_today
    } else {
        0
    };

    let new_amount_today = amount_today
        .checked_add(amount)
        .ok_or(ComplianceError::ArithmeticOverflow)?;

    require!(
        new_amount_today <= ctx.accounts.global_policy.daily_transfer_limit,
        ComplianceError::DailyLimitExceeded
    );

    let new_total_transferred = ctx
        .accounts
        .sender_stats
        .total_transferred
        .checked_add(amount)
        .ok_or(ComplianceError::ArithmeticOverflow)?;

    let new_transfer_count = ctx
        .accounts
        .sender_stats
        .transfer_count
        .checked_add(1)
        .ok_or(ComplianceError::ArithmeticOverflow)?;

    ctx.accounts.sender_stats.day_index = current_day_index;
    ctx.accounts.sender_stats.amount_today = new_amount_today;
    ctx.accounts.sender_stats.total_transferred = new_total_transferred;
    ctx.accounts.sender_stats.transfer_count = new_transfer_count;

    Ok(())
}
