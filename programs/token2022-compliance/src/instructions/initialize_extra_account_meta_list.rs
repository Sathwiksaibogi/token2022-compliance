use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds=[b"extra-account-metas",mint.key().as_ref()],
        bump,
    )]
    /// CHECK: This is the canonical ExtraAccountMetaList PDA derived from
    /// [b"extra-account-metas", mint]. Its address is validated by PDA seeds,
    /// and its TLV data will be initialized manually by this program.
    pub extra_account_meta_list: UncheckedAccount<'info>,

    #[account(
        owner=token_program.key(),
        mint::authority=mint_authority,
        mint::token_program=token_program,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    pub mint_authority: Signer<'info>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}
