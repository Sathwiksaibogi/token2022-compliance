use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

use crate::state::{Authorization, AuthorizationStatus, GlobalPolicy};

#[derive(Accounts)]
pub struct SetAuthorizationStatus<'info> {
    pub admin: Signer<'info>,

    #[account(
        owner=token_program.key(),
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds=[b"policy",mint.key().as_ref()],
        bump=global_policy.bump,
        has_one=admin,
        has_one=mint,
    )]
    pub global_policy: Account<'info, GlobalPolicy>,

    #[account(
        mut,
        seeds=[b"authorization",mint.key().as_ref(),authorization.wallet.as_ref()],
        bump=authorization.bump,
        has_one=mint,
    )]
    pub authorization: Account<'info, Authorization>,

    pub token_program: Program<'info, Token2022>,
}


