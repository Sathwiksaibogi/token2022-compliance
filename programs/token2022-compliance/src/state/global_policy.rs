use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]

pub struct GlobalPolicy{
    pub admin:Pubkey,
    pub mint:Pubkey,
    pub enabled:bool,
    pub max_transfer_amount:u64,
    pub daily_transfer_limit:u64,
    pub policy_version:u64,
    pub expires_at:i64,
    pub bump:u8,
}