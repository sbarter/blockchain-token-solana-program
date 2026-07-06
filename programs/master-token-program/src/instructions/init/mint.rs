use anchor_lang::{prelude::*, solana_program::sysvar};
use anchor_spl::{token_2022::Token2022, token_interface::Mint};
use mpl_token_metadata::{
    instructions::CreateV1CpiBuilder,
    types::{PrintSupply, TokenStandard},
};

use crate::{SBT_DECIMALS, SBT_METADATA_URL, TOTAL_MINT_SUPPLY};

pub fn initialize_mint<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeMint<'info>>,
) -> Result<()> {
    let master_seeds = &[b"master".as_ref(), &[ctx.bumps.master_pda]];
    let signer_seeds = &[&master_seeds[..]];
    #[cfg(feature = "local-testing")]
    CreateV1CpiBuilder::new(&ctx.accounts.mpl_metadata_program.to_account_info())
        .name("Sbarter".to_string())
        .symbol("SBT".to_string())
        .uri(SBT_METADATA_URL.to_string())
        .metadata(&ctx.accounts.metadata)
        .mint(&ctx.accounts.mint.to_account_info(), true)
        .authority(&ctx.accounts.master_pda)
        .payer(&ctx.accounts.authority)
        .update_authority(&ctx.accounts.authority, true)
        .sysvar_instructions(&ctx.accounts.sysvar_instructions)
        .system_program(&ctx.accounts.system_program.to_account_info())
        .spl_token_program(Some(&ctx.accounts.token_program))
        .seller_fee_basis_points(0)
        .token_standard(TokenStandard::Fungible)
        .print_supply(PrintSupply::Limited(TOTAL_MINT_SUPPLY))
        .decimals(SBT_DECIMALS)
        .invoke_signed(signer_seeds)?;
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMint<'info> {
    #[account(mut, signer)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"master"],
        bump
    )]
    /// CHECK: pda authority
    pub master_pda: AccountInfo<'info>,

    /// CHECK: unintilialized metadata account PDA
    #[account(
        mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            mint.key().as_ref(),
        ],
        bump,
        seeds::program = mpl_token_metadata::ID,
    )]
    pub metadata: AccountInfo<'info>,

    #[account(
        init,
        payer = authority,
        mint::decimals = SBT_DECIMALS,
        mint::authority = master_pda,
        mint::token_program = token_program
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    /// CHECK: mpl_token_metadata program
    #[account(address = mpl_token_metadata::ID)]
    pub mpl_metadata_program: AccountInfo<'info>,
    /// CHECK: sysvar instructions account
    #[account(address = sysvar::instructions::ID)]
    pub sysvar_instructions: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
