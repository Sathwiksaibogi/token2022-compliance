use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::{
    token_2022::{
        spl_token_2022::extension::transfer_fee::{
            instruction as transfer_fee_instruction, MAX_FEE_BASIS_POINTS,
        },
        Token2022,
    },
    token_interface::Mint,
};

use crate::{error::ComplianceError, state::GlobalPolicy};

#[derive(Accounts)]
pub struct UpdateTransferFee<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
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

pub fn update_transfer_fee_handler(
    ctx: Context<UpdateTransferFee>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    require!(
        transfer_fee_basis_points <= MAX_FEE_BASIS_POINTS,
        ComplianceError::InvalidTransferFeeBasisPoints
    );

    let mint_key = ctx.accounts.mint.key();
    let policy_key = ctx.accounts.global_policy.key();
    let token_program_key = ctx.accounts.token_program.key();

    let set_transfer_fee_ix = transfer_fee_instruction::set_transfer_fee(
        &token_program_key,
        &mint_key,
        &policy_key,
        &[],
        transfer_fee_basis_points,
        maximum_fee,
    )?;

    let bump_seed = [ctx.accounts.global_policy.bump];
    let signer_seeds: &[&[u8]] = &[b"policy", mint_key.as_ref(), &bump_seed];

    invoke_signed(
        &set_transfer_fee_ix,
        &[
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.global_policy.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[signer_seeds],
    )?;

    ctx.accounts.global_policy.policy_version = ctx
        .accounts
        .global_policy
        .policy_version
        .checked_add(1)
        .ok_or(ComplianceError::ArithmeticOverflow)?;

    Ok(())
}
