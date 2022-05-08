use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::account_data::escrow_account::EscrowAccount;
use crate::constants::VAULT_AUTHORITY_SEED;

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    #[account(mut)]
    pub vault_account: Account<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = escrow_account.initializer_key == *initializer.key,
        constraint = escrow_account.initializer_deposit_token_account == initializer_deposit_token_account.key(),
        close = initializer,
    )]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Cancel<'info> {
    fn into_transfer_to_initializer_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_account.to_account_info().clone(),
            to: self
                .initializer_deposit_token_account
                .to_account_info()
                .clone(),
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

pub fn process_cancel(ctx: Context<Cancel>) -> Result<()> {
    let (_vault_authority, vault_authority_bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED], ctx.program_id);

    let authority_seeds = &[&VAULT_AUTHORITY_SEED[..], &[vault_authority_bump]];

    token::transfer(
        ctx.accounts
            .into_transfer_to_initializer_context()
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
