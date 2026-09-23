use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum AuthorizationStatus {
    Unauthorized,
    Authorized,
    Blocked,
}

#[account]
#[derive(InitSpace)]
pub struct Authorization {
    pub mint: Pubkey,
    pub wallet: Pubkey,
    pub status: AuthorizationStatus,
    pub bump: u8,
}
