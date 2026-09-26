use anchor_lang::prelude::*;

#[error_code]
pub enum ComplianceError {
    #[msg("Maximum transfer amount must be greater than zero")]
    InvalidMaxTransferAmount,

    #[msg("Daily transfer limit must be at least the maximum transfer amount")]
    InvalidDailyLimit,

    #[msg("Expiration must be zero or a future Unix timestamp")]
    InvalidExpiration,

    #[msg("Invalid Transfer Hook invocation")]
    InvalidTransferHookInvocation,

    #[msg("Global Policy is not enabled")]
    PolicyDisabled,

    #[msg("Global Policy has Expired")]
    PolicyExpired,

    #[msg("Sender is not Authorized")]
    SenderNotAuthorized,

    #[msg("Sender is Blocked")]
    SenderBlocked,

    #[msg("Receiver is not Authorized")]
    ReceiverNotAuthorized,

    #[msg("Receiver is blocked")]
    ReceiverBlocked,

    #[msg("Maximum transfer amount has exceeded")]
    MaxTransferAmountExceeded,

    #[msg("Daily transfer limit exceeded")]
    DailyLimitExceeded,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    #[msg("Mint is not configured to use this compliance transfer hook")]
    InvalidTransferHookProgram,

    #[msg("Transfer fee basis points cannot exceed 10,000")]
    InvalidTransferFeeBasisPoints,
}
