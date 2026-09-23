use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]

pub struct TransferStats {
    pub mint: Pubkey,
    pub wallet: Pubkey,
    pub day_index: i64,
    pub amount_today: u64,
    pub total_transferred: u64,
    pub transfer_count: u64,
    pub bump: u8,
}
