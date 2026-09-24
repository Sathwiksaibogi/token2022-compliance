use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};

use anchor_lang::solana_program::{program::invoke_signed, system_instruction};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

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

pub fn initialize_extra_account_meta_list_handler(
    ctx: Context<InitializeExtraAccountMetaList>,
) -> Result<()> {
    let extra_account_metas = vec![
        // GlobalPolicy
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"policy".to_vec(),
                },
                Seed::AccountKey { index: 1 },
            ],
            false, // signer?
            false, // writable?
        )?,
        // SenderAuthorization
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"authorization".to_vec(),
                },
                Seed::AccountKey { index: 1 },
                Seed::AccountData {
                    account_index: 0,
                    data_index: 32,
                    length: 32,
                },
            ],
            false,
            false,
        )?,
        // ReceiverAuthorization
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"authorization".to_vec(),
                },
                Seed::AccountKey { index: 1 },
                Seed::AccountData {
                    account_index: 2,
                    data_index: 32,
                    length: 32,
                },
            ],
            false,
            false,
        )?,
        // SenderStats
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"stats".to_vec(),
                },
                Seed::AccountKey { index: 1 },
                Seed::AccountData {
                    account_index: 0,
                    data_index: 32,
                    length: 32,
                },
            ],
            false,
            true,
        )?,
    ];

    let account_size = ExtraAccountMetaList::size_of(extra_account_metas.len())?;
    let lamports = Rent::get()?.minimum_balance(account_size);

    let mint_key = ctx.accounts.mint.key();
    let bump_seed = [ctx.bumps.extra_account_meta_list];

    let signer_seeds: &[&[u8]] = &[b"extra-account-metas", mint_key.as_ref(), &bump_seed];
    let create_account_ix = system_instruction::create_account(
        ctx.accounts.payer.key,
        ctx.accounts.extra_account_meta_list.key,
        lamports,
        account_size as u64,
        ctx.program_id,
    );

    invoke_signed(
        &create_account_ix,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.extra_account_meta_list.to_account_info(),
        ],
        &[signer_seeds],
    )?;

    let account_info = ctx.accounts.extra_account_meta_list.to_account_info();

    let mut data = account_info.try_borrow_mut_data()?;

    ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &extra_account_metas)?;
    
    Ok(())
}
