use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    AmountMustBePositive,
    #[msg("Sender and receiver must be different")]
    SenderEqualsReceiver,
    #[msg("Escrow is not in the required status for this instruction")]
    InvalidStatus,
    #[msg("Signer is not authorized for this escrow")]
    Unauthorized,
    #[msg("Mint does not match the escrow deal")]
    MintMismatch,
    #[msg("Receiver does not match the escrow deal")]
    ReceiverMismatch,
    #[msg("Token account balance is insufficient for this deposit")]
    InsufficientBalance,
}
