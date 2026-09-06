use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    error::EscrowError,
    state::{EscrowState, EscrowStatus},
};

#[derive(Accounts)]
#[instruction(deal_id: u64)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        mut,
        seeds = [EscrowState::SEED_PREFIX, sender.key().as_ref(), &deal_id.to_le_bytes()],
        bump = escrow.bump,
        has_one = sender @ EscrowError::Unauthorized,
        has_one = mint @ EscrowError::MintMismatch,
        constraint = escrow.deal_id == deal_id,
        constraint = (escrow.status == EscrowStatus::Created
            || escrow.status == EscrowStatus::Funded) @ EscrowError::InvalidStatus,
        close = sender,
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

pub fn handler(ctx: Context<Cancel>, deal_id: u64) -> Result<()> {
    let decimals = ctx.accounts.mint.decimals;
    let bump = ctx.accounts.escrow.bump;
    let sender_key = ctx.accounts.escrow.sender;
    let deal_id_bytes = deal_id.to_le_bytes();
    let seeds: &[&[u8]] = &[
        EscrowState::SEED_PREFIX,
        sender_key.as_ref(),
        &deal_id_bytes,
        &[bump],
    ];
    let signer = &[seeds];

    if ctx.accounts.escrow.status == EscrowStatus::Funded {
        let amount = ctx.accounts.escrow.amount;
        let transfer_accounts = TransferChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.sender_token_account.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        };
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                transfer_accounts,
                signer,
            ),
            amount,
            decimals,
        )?;
    }

    let close_accounts = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.sender.to_account_info(),
        authority: ctx.accounts.escrow.to_account_info(),
    };
    token_interface::close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        close_accounts,
        signer,
    ))?;

    ctx.accounts.escrow.status = EscrowStatus::Cancelled;
    Ok(())
}
