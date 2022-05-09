use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::account_data::escrow_account::EscrowAccount;
use crate::constants::VAULT_AUTHORITY_SEED;

#[derive(Accounts)]
pub struct Exchange<'info> {
    pub taker: Signer<'info>,

    #[account(mut)]
    pub taker_deposit_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub taker_receive_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub initializer_deposit_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub initializer_receive_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub initializer: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = escrow_account.taker_amount <= taker_deposit_token_account.amount,
        constraint = escrow_account.initializer_deposit_token_account == initializer_deposit_token_account.key(),
        constraint = escrow_account.initializer_receive_token_account == initializer_receive_token_account.key(),
        constraint = escrow_account.initializer_key == *initializer.key,
    )]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,

    #[account(mut)]
    pub vault_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Exchange<'info> {
    fn into_tranfer_to_initializer_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_deposit_token_account.to_account_info().clone(),
            to: self
                .initializer_receive_token_account
                .to_account_info()
                .clone(),
            authority: self.taker.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn into_transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_account.to_account_info().clone(),
            to: self.taker_receive_token_account.to_account_info().clone(),
            authority: self.vault_authority.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn into_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_account.to_account_info().clone(),
            destination: self.initializer.to_account_info().clone(),
            authority: self.vault_authority.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

pub fn process_exchange(ctx: Context<Exchange>) -> Result<()> {
    msg!("Hi!");
    let (_vault_authority, vault_authority_bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED], ctx.program_id);

    let authority_seeds = &[&VAULT_AUTHORITY_SEED[..], &[vault_authority_bump]];

    token::transfer(
        ctx.accounts.into_tranfer_to_initializer_context(),
        ctx.accounts.escrow_account.taker_amount,
    )?;

    token::transfer(
        ctx.accounts
            .into_transfer_to_taker_context()
            .with_signer(&[&authority_seeds[..]]),
        ctx.accounts.escrow_account.initializer_amount,
    )?;

    token::close_account(
        ctx.accounts
            .into_close_context()
            .with_signer(&[&authority_seeds[..]]),
    )?;

    Ok(())
}
