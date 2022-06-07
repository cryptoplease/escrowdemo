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

    #[access_control(CreateEscrow::accounts(&ctx, nonce))]
    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        amount: u64,
        nonce: u8,
    ) -> Result<()> {
        // Transfer funds to the escrow.
        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token.to_account_info().clone(),
            to: ctx.accounts.vault.to_account_info().clone(),
            authority: ctx.accounts.seller.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Print the escrow.
        let escrow = &mut ctx.accounts.escrow;
        escrow.amount = amount;
        escrow.seller = *ctx.accounts.seller.to_account_info().key;
        escrow.seller_token = *ctx.accounts.seller_token.to_account_info().key;
        escrow.buyer_token = *ctx.accounts.buyer_token.to_account_info().key;
        escrow.vault = *ctx.accounts.vault.to_account_info().key;
        escrow.nonce = nonce;

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
            to: ctx.accounts.buyer_token.to_account_info().clone(),
            authority: ctx.accounts.escrow_signer.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, ctx.accounts.escrow.amount)?;
        // Burn the check for one time use.
        ctx.accounts.escrow.burned = true;
        Ok(())
    }
/* Put in Comment CanCheck action
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
*/

}

#[derive(Accounts)]
pub struct CreateEscrow<'info> {
    // Check being created.
    #[account(zero)]
    escrow: Account<'info, Escrow>,
    // Check's token vault.
    #[account(mut, constraint = &vault.owner == escrow_signer.key)]
    vault: Account<'info, TokenAccount>,
    // Program derived address for the check.
    /// CHECK: no check
    escrow_signer: AccountInfo<'info>,
    // Token account the check is made from.
    #[account(mut, constraint = &seller_token.owner == seller.key)]
    seller_token: Account<'info, TokenAccount>,
    // Token account the check is made to.
    #[account(constraint = seller_token.mint == buyer_token.mint)]
    buyer_token: Account<'info, TokenAccount>,
    // Owner of the `from` token account.
    /// CHECK: no check
    seller: AccountInfo<'info>,
    /// CHECK: no check
    token_program: AccountInfo<'info>,
}

impl<'info> CreateEscrow<'info> {
    pub fn accounts(ctx: &Context<CreateEscrow>, nonce: u8) -> Result<()> {
        let signer = Pubkey::create_program_address(
            &[ctx.accounts.escrow.to_account_info().key.as_ref(), &[nonce]],
            ctx.program_id,
        )
        .map_err(|_| error!(ErrorCode::InvalidCheckNonce))?;
        if &signer != ctx.accounts.escrow_signer.to_account_info().key {
            return err!(ErrorCode::InvalidCheckSigner);
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CashCheck<'info> {
    #[account(mut, has_one = vault, has_one = buyer_token)]
    escrow: Account<'info, Escrow>,
    #[account(mut)]
    /// CHECK: no check
    vault: AccountInfo<'info>,
    #[account(
        seeds = [escrow.to_account_info().key.as_ref()],
        bump = escrow.nonce,
    )]
    /// CHECK: no check
    escrow_signer: AccountInfo<'info>,
    #[account(mut,  constraint = &buyer_token.owner == buyer.key)]
    buyer_token: Account<'info, TokenAccount>,
    buyer: Signer<'info>,
    /// CHECK: no check
    token_program: AccountInfo<'info>,
}

/* Put in Comment CanCheck action
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
*/

#[account]
pub struct Escrow {
    seller: Pubkey,
    seller_token: Pubkey,
    buyer_token: Pubkey,
    amount: u64,
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
