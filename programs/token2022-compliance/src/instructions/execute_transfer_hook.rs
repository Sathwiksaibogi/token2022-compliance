use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::state::{Authorization, GlobalPolicy, TransferStats};

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
