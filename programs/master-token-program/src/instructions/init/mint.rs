use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};
use mpl_token_metadata::{
    instructions::{CreateMetadataAccountV3Cpi, CreateMetadataAccountV3InstructionArgs},
    types::DataV2,
};

use crate::{MASTER_WALLET, SBT_DECIMALS, TESTING};

pub fn initialize_mint<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeMint<'info>>,
) -> Result<()> {
    let data = DataV2 {
        name: "Sbarter".to_string(),
        symbol: "SBT".to_string(),
        uri: "".to_string(),
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    let create_metadata_accounts_v3_ix = CreateMetadataAccountV3Cpi {
        metadata: &ctx.accounts.metadata,
        mint: &ctx.accounts.mint.to_account_info(),
        mint_authority: &ctx.accounts.master,
        payer: &ctx.accounts.master,
        update_authority: (&ctx.accounts.master, true),
        system_program: &ctx.accounts.system_program.to_account_info(),
        rent: None,
        __program: &ctx.accounts.mpl_metadata_program.to_account_info(),
        __args: CreateMetadataAccountV3InstructionArgs {
            data,
            is_mutable: true,
            collection_details: None,
        },
    };
    create_metadata_accounts_v3_ix.invoke()?;
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMint<'info> {
    #[account(
        mut,
        signer,
        constraint = TESTING || master.key() == MASTER_WALLET
    )]
    pub master: Signer<'info>,

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
        payer = master,
        mint::decimals = SBT_DECIMALS,
        mint::authority = master,
        mint::token_program = token_program
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    /// CHECK: mpl_token_metadata program
    #[account(address = mpl_token_metadata::ID)]
    pub mpl_metadata_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
