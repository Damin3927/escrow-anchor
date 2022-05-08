use anchor_lang::prelude::*;
use anchor_spl::token::{
    self, spl_token::instruction::AuthorityType, Mint, SetAuthority, Token, TokenAccount, Transfer,
};

use crate::account_data::escrow_account::EscrowAccount;
use crate::constants::{VAULT_ACCOUNT_SEED, VAULT_AUTHORITY_SEED};

#[derive(Accounts)]
#[instruction(vault_account_bump: u8, initializer_amount: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [VAULT_ACCOUNT_SEED],
        bump,
        payer = initializer,
        token::mint = mint,
        token::authority = initializer,
    )]
    pub vault_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = initializer_deposit_token_account.amount >= initializer_amount
    )]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,

    pub initializer_receive_token_account: Account<'info, TokenAccount>,

    #[account(zero)]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Initialize<'info> {
    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.vault_account.to_account_info().clone(),
            current_authority: self.initializer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn into_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .initializer_deposit_token_account
                .to_account_info()
                .clone(),
            to: self.vault_account.to_account_info().clone(),
            authority: self.initializer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

pub fn process_initialize(
    ctx: Context<Initialize>,
    _vault_account_bump: u8,
    initializer_amount: u64,
    taker_amount: u64,
) -> Result<()> {
    // initialize escrow_account
    let escrow_account = &mut ctx.accounts.escrow_account;
    escrow_account.initializer_key = *ctx.accounts.initializer.key;
    escrow_account.initializer_deposit_token_account =
        ctx.accounts.initializer_deposit_token_account.key();
    escrow_account.initializer_receive_token_account =
        ctx.accounts.initializer_receive_token_account.key();
    escrow_account.initializer_amount = initializer_amount;
    escrow_account.taker_amount = taker_amount;

    // find pda and set as authority
    let (vault_authority, _vault_authority_bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED], ctx.program_id);

    token::set_authority(
        ctx.accounts.into_set_authority_context(),
        AuthorityType::AccountOwner,
        Some(vault_authority),
    )?;

    // transfer the initializer's token to the pda above
    token::transfer(
        ctx.accounts.into_transfer_to_pda_context(),
        ctx.accounts.escrow_account.initializer_amount,
    )?;

    Ok(())
}
