use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{
    error::EscrowError,
    state::{EscrowState, EscrowStatus},
};

#[derive(Accounts)]
#[instruction(deal_id: u64)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        mut,
        seeds = [EscrowState::SEED_PREFIX, sender.key().as_ref(), &deal_id.to_le_bytes()],
        bump = escrow.bump,
        has_one = sender @ EscrowError::Unauthorized,
        has_one = mint @ EscrowError::MintMismatch,
        constraint = escrow.deal_id == deal_id,
        constraint = escrow.status == EscrowStatus::Created @ EscrowError::InvalidStatus,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = sender,
        token::token_program = token_program,
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = escrow,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Deposit>, _deal_id: u64) -> Result<()> {
    let amount = ctx.accounts.escrow.amount;
    require!(
        ctx.accounts.sender_token_account.amount >= amount,
        EscrowError::InsufficientBalance
    );

    let decimals = ctx.accounts.mint.decimals;
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.sender_token_account.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.sender.to_account_info(),
    };
    let cpi_context = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
    token_interface::transfer_checked(cpi_context, amount, decimals)?;

    ctx.accounts.escrow.status = EscrowStatus::Funded;
    Ok(())
}
