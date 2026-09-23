use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

use crate::state::{Authorization, AuthorizationStatus, GlobalPolicy};

#[derive(Accounts)]
#[instruction(wallet:Pubkey)]

pub struct InitializeAuthorization<'info> {
    #[account(mut)]
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
        init,
        payer=admin,
        space=8+Authorization::INIT_SPACE,
        seeds=[b"authorization",mint.key().as_ref(),wallet.as_ref()],
        bump,
    )]
    pub authorization: Account<'info, Authorization>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_authorization_handler(
    ctx: Context<InitializeAuthorization>,
    wallet: Pubkey,
) -> Result<()> {
    ctx.accounts.authorization.mint = ctx.accounts.mint.key();
    ctx.accounts.authorization.wallet = wallet;
    ctx.accounts.authorization.status = AuthorizationStatus::Unauthorized;
    ctx.accounts.authorization.bump = ctx.bumps.authorization;
    Ok(())
    
}
