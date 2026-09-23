use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::Token2022,
    token_interface::Mint,
};

use crate::state::GlobalPolicy;

#[derive(Accounts)]
pub struct InitializePolicy<'info>{
    #[account(mut)]
    pub admin:Signer<'info>,

    #[account(
        owner=token_program.key(),
    )]
    pub mint:InterfaceAccount<'info,Mint>,

    #[account(
        init,
        payer=admin,
        space=8+GlobalPolicy::INIT_SPACE,
        seeds=[b"policy",mint.key().as_ref()],
        bump,
    )]
    pub global_policy:Account<'info,GlobalPolicy>,

    pub token_program:Program<'info,Token2022>,

    pub system_program:Program<'info,System>,
}