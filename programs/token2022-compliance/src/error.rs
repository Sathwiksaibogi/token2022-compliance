use anchor_lang::prelude::*;

#[error_code]
pub enum ComplianceError {
    #[msg("Maximum transfer amount must be greater than zero")]
    InvalidMaxTransferAmount,

    #[msg("Daily transfer limit must be at least the maximum transfer amount")]
    InvalidDailyLimit,

    #[msg("Expiration must be zero or a future Unix timestamp")]
    InvalidExpiration,
}
