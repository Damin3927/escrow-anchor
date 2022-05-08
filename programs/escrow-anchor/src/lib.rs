pub mod account_data;
pub mod constants;
pub mod instructions;

use crate::instructions::cancel::*;
use crate::instructions::exchange::*;
use crate::instructions::initialize::*;
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod escrow_anchor {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        vault_account_bump: u8,
        initializer_amount: u64,
        taker_amount: u64,
    ) -> Result<()> {
        process_initialize(ctx, vault_account_bump, initializer_amount, taker_amount)
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        process_cancel(ctx)
    }

    pub fn exchange(ctx: Context<Exchange>) -> Result<()> {
        process_exchange(ctx)
    }
}
