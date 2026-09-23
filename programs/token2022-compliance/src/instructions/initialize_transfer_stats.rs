use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

use crate::state::{Authorization, TransferStats};

#[derive(Accounts)]
#[instruction(wallet:Pubkey)]

pub struct InitializeTransferStats<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        owner=token_program.key(),
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds=[b"authorization",mint.key().as_ref(),wallet.as_ref()],
        bump=authorization.bump,
        has_one=mint,
        constraint = authorization.wallet == wallet,
    )]
    pub authorization: Account<'info, Authorization>,

    #[account(
        init,
        payer=payer,
        space=8+TransferStats::INIT_SPACE,
        seeds=[b"stats",mint.key().as_ref(),wallet.as_ref()],
        bump,
    )]
    pub transfer_stats: Account<'info, TransferStats>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}
pub fn initialize_transfer_stats_handler(
    ctx: Context<InitializeTransferStats>,
    wallet: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    let day_index = clock.unix_timestamp / 86_400;

    ctx.accounts.transfer_stats.mint = ctx.accounts.mint.key();
    ctx.accounts.transfer_stats.wallet = wallet;
    ctx.accounts.transfer_stats.day_index = day_index;
    ctx.accounts.transfer_stats.amount_today = 0;
    ctx.accounts.transfer_stats.total_transferred = 0;
    ctx.accounts.transfer_stats.transfer_count = 0;
    ctx.accounts.transfer_stats.bump = ctx.bumps.transfer_stats;
    Ok(())
}
