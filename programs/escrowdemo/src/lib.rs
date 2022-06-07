//! A cashiers check example. The funds are immediately withdrawn from a user's
//! account and sent to a program controlled `Check` account, where the funds
//! reside until they are "cashed" by the intended recipient. The creator of
//! the check can cancel the check at any time to get back the funds.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::convert::Into;

declare_id!("HadGeJ6KCGiyzVTe13n5TY1rCSEyd9Akgx7cYBvYx5Dt");

#[program]
pub mod escrowdemo {
    use super::*;

    #[access_control(CreateCheck::accounts(&ctx, nonce))]
    pub fn create_check(
        ctx: Context<CreateCheck>,
        amount: u64,
        memo: Option<String>,
        nonce: u8,
    ) -> Result<()> {
        // Transfer funds to the check.
        let cpi_accounts = Transfer {
            from: ctx.accounts.from.to_account_info().clone(),
            to: ctx.accounts.vault.to_account_info().clone(),
            authority: ctx.accounts.owner.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Print the check.
        let escrow = &mut ctx.accounts.escrow;
        escrow.amount = amount;
        escrow.from = *ctx.accounts.from.to_account_info().key;
        escrow.to = *ctx.accounts.to.to_account_info().key;
        escrow.vault = *ctx.accounts.vault.to_account_info().key;
        escrow.nonce = nonce;
        escrow.memo = memo;

        Ok(())
    }

    #[access_control(not_burned(&ctx.accounts.escrow))]
    pub fn cash_check(ctx: Context<CashCheck>) -> Result<()> {
        let seeds = &[
            ctx.accounts.escrow.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info().clone(),
            to: ctx.accounts.to.to_account_info().clone(),
            authority: ctx.accounts.check_signer.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, ctx.accounts.escrow.amount)?;
        // Burn the check for one time use.
        ctx.accounts.escrow.burned = true;
        Ok(())
    }

    #[access_control(not_burned(&ctx.accounts.escrow))]
    pub fn cancel_check(ctx: Context<CancelCheck>) -> Result<()> {
        let seeds = &[
            ctx.accounts.escrow.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info().clone(),
            to: ctx.accounts.from.to_account_info().clone(),
            authority: ctx.accounts.check_signer.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, ctx.accounts.escrow.amount)?;
        ctx.accounts.escrow.burned = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateCheck<'info> {
    // Check being created.
    #[account(zero)]
    escrow: Account<'info, Escrow>,
    // Check's token vault.
    #[account(mut, constraint = &vault.owner == check_signer.key)]
    vault: Account<'info, TokenAccount>,
    // Program derived address for the check.
    /// CHECK: no check
    check_signer: AccountInfo<'info>,
    // Token account the check is made from.
    #[account(mut, has_one = owner)]
    from: Account<'info, TokenAccount>,
    // Token account the check is made to.
    #[account(constraint = from.mint == to.mint)]
    to: Account<'info, TokenAccount>,
    // Owner of the `from` token account.
    /// CHECK: no check
    owner: AccountInfo<'info>,
    /// CHECK: no check
    token_program: AccountInfo<'info>,
}

impl<'info> CreateCheck<'info> {
    pub fn accounts(ctx: &Context<CreateCheck>, nonce: u8) -> Result<()> {
        let signer = Pubkey::create_program_address(
            &[ctx.accounts.escrow.to_account_info().key.as_ref(), &[nonce]],
            ctx.program_id,
        )
        .map_err(|_| error!(ErrorCode::InvalidCheckNonce))?;
        if &signer != ctx.accounts.check_signer.to_account_info().key {
            return err!(ErrorCode::InvalidCheckSigner);
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CashCheck<'info> {
    #[account(mut, has_one = vault, has_one = to)]
    escrow: Account<'info, Escrow>,
    #[account(mut)]
    /// CHECK: no check
    vault: AccountInfo<'info>,
    #[account(
        seeds = [escrow.to_account_info().key.as_ref()],
        bump = escrow.nonce,
    )]
    /// CHECK: no check
    check_signer: AccountInfo<'info>,
    #[account(mut, has_one = owner)]
    to: Account<'info, TokenAccount>,
    owner: Signer<'info>,
    /// CHECK: no check
    token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CancelCheck<'info> {
    #[account(mut, has_one = vault, has_one = from)]
    escrow: Account<'info, Escrow>,
    #[account(mut)]
    /// CHECK: no check
    vault: AccountInfo<'info>,
    #[account(
        seeds = [escrow.to_account_info().key.as_ref()],
        bump = escrow.nonce,
    )]
    /// CHECK: no check
    check_signer: AccountInfo<'info>,
    #[account(mut, has_one = owner)]
    from: Account<'info, TokenAccount>,
    owner: Signer<'info>,
    /// CHECK: no chck
    token_program: AccountInfo<'info>,
}

#[account]
pub struct Escrow {
    from: Pubkey,
    to: Pubkey,
    amount: u64,
    memo: Option<String>,
    vault: Pubkey,
    nonce: u8,
    burned: bool,
}

/*
#[account]
pub struct Escrow {
    buyer: Pubkey,
    seller: Pubkey,
    amount: u64,
    vault: Pubkey,
    nonce: u8,
    dao_fee: u64,
    dao_fee_address: Pubkey,
    dao_authority: Pubkey,
    marketplace_fee: u64,
    marketplace_fee_address: Pubkey,
    status: u8, // switch to TradeStatus
}
*/

#[error_code]
pub enum ErrorCode {
    #[msg("The given nonce does not create a valid program derived address.")]
    InvalidCheckNonce,
    #[msg("The derived check signer does not match that which was given.")]
    InvalidCheckSigner,
    #[msg("The given check has already been burned.")]
    AlreadyBurned,
}

fn not_burned(escrow: &Escrow) -> Result<()> {
    if escrow.burned {
        return err!(ErrorCode::AlreadyBurned);
    }
    Ok(())
}
