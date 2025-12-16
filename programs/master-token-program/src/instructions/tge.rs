use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{get_associated_token_address_with_program_id, AssociatedToken},
    token_2022::{
        self, spl_token_2022::instruction::AuthorityType, MintTo, SetAuthority, Token2022,
        TransferChecked,
    },
    token_interface::{Mint, TokenAccount},
};

use crate::{
    category::{CategoryData, LIQUIDITY_CATEGORY, MARKETING_CATEGORY, RESERVE_CATEGORY},
    LIQUIDITY_LIQUID_SUPPLY, MARKETING_LIQUID_SUPPLY, RESERVE_LIQUID_SUPPLY, RESERVE_PADDING,
    SBT_DECIMALS, TOTAL_MINT_SUPPLY,
};

pub fn start_tge<'info>(ctx: Context<'_, '_, '_, 'info, Tge<'info>>) -> Result<()> {
    // Already checked that `master` is the mint authority.

    require_keys_eq!(
        ctx.accounts.master_ata.key(),
        get_associated_token_address_with_program_id(
            &ctx.accounts.master.to_account_info().key(),
            &ctx.accounts.mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );
    require_keys_eq!(
        ctx.accounts.marketing_ata.key(),
        get_associated_token_address_with_program_id(
            &ctx.accounts.marketing_cat.to_account_info().key(),
            &ctx.accounts.mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );
    require_keys_eq!(
        ctx.accounts.liquidity_ata.key(),
        get_associated_token_address_with_program_id(
            &ctx.accounts.liquidity_cat.to_account_info().key(),
            &ctx.accounts.mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );

    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.master_ata.to_account_info(),
        authority: ctx.accounts.master.to_account_info(),
    };
    token_2022::mint_to(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
        TOTAL_MINT_SUPPLY,
    )?;

    let cpi_accounts_set = SetAuthority {
        account_or_mint: ctx.accounts.mint.to_account_info(),
        current_authority: ctx.accounts.master.to_account_info(),
    };
    token_2022::set_authority(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_set,
        ),
        AuthorityType::MintTokens,
        None,
    )?;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.master_ata.to_account_info(),
        to: ctx.accounts.marketing_ata.to_account_info(),
        authority: ctx.accounts.master.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_2022::transfer_checked(cpi_ctx, MARKETING_LIQUID_SUPPLY, SBT_DECIMALS as u8)?;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.master_ata.to_account_info(),
        to: ctx.accounts.liquidity_ata.to_account_info(),
        authority: ctx.accounts.master.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_2022::transfer_checked(cpi_ctx, LIQUIDITY_LIQUID_SUPPLY, SBT_DECIMALS as u8)?;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.master_ata.to_account_info(),
        to: ctx.accounts.reserve_ata.to_account_info(),
        authority: ctx.accounts.master.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_2022::transfer_checked(
        cpi_ctx,
        RESERVE_LIQUID_SUPPLY + RESERVE_PADDING,
        SBT_DECIMALS as u8,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct Tge<'info> {
    #[account(mut, signer, address = crate::MASTER_WALLET)]
    pub master: Signer<'info>,
    /// CHECK: created by Initialize, checked to be owned by marketing_cat
    #[account(mut)]
    pub master_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [MARKETING_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub marketing_cat: Account<'info, CategoryData>,
    /// CHECK: created by Initialize, checked to be owned by marketing_cat
    #[account(mut)]
    pub marketing_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [LIQUIDITY_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub liquidity_cat: Account<'info, CategoryData>,
    /// CHECK: created by Initialize, checked to be owned by liquidity_cat
    #[account(mut)]
    pub liquidity_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [RESERVE_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub reserve_cat: Account<'info, CategoryData>,
    /// CHECK: created by Initialize, checked to be owned by liquidity_cat
    #[account(mut)]
    pub reserve_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, mint::authority = master)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
