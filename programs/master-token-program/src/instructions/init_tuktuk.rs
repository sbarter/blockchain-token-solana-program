use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_spl::{associated_token::AssociatedToken, token_2022::Token2022, token_interface::Mint};
use tuktuk_program::{
    tuktuk::{
        self,
        cpi::{
            accounts::{AddQueueAuthorityV0, InitializeTaskQueueV0},
            add_queue_authority_v0, initialize_task_queue_v0,
        },
        program::Tuktuk,
    },
    types::InitializeTaskQueueArgsV0,
    TuktukConfigV0,
};

fn create_tuktuk_task_queue<'info>(
    ctx: &Context<'_, '_, '_, 'info, InitializeTuktuk<'info>>,
    tuktuk_config: &AccountInfo<'info>,
    task_queue_name: String,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.tuktuk_config.key(),
        Pubkey::find_program_address(&[b"tuktuk_config"], &tuktuk::ID).0
    );
    let tuktuk_config = {
        let data = tuktuk_config.try_borrow_data()?;
        TuktukConfigV0::try_deserialize(&mut &data[..])?
    };

    require_keys_eq!(
        ctx.accounts.task_queue.key(),
        Pubkey::find_program_address(
            &[
                b"task_queue",
                ctx.accounts.tuktuk_config.key().as_ref(),
                &tuktuk_config.next_task_queue_id.to_le_bytes()
            ],
            &tuktuk::ID
        )
        .0
    );
    require_keys_eq!(
        ctx.accounts.task_queue_name_mapping.key(),
        Pubkey::find_program_address(
            &[
                b"task_queue_name_mapping",
                ctx.accounts.tuktuk_config.key().as_ref(),
                &hash(task_queue_name.as_bytes()).to_bytes(),
            ],
            &tuktuk::ID
        )
        .0
    );
    require_keys_eq!(
        ctx.accounts.task_queue_authority.key(),
        Pubkey::find_program_address(
            &[
                b"task_queue_authority",
                ctx.accounts.task_queue.key().as_ref(),
                ctx.accounts.master_pda.key().as_ref(),
            ],
            &tuktuk::ID
        )
        .0
    );

    let cpi_accounts = InitializeTaskQueueV0 {
        payer: ctx.accounts.master.to_account_info(),
        tuktuk_config: ctx.accounts.tuktuk_config.to_account_info(),
        update_authority: ctx.accounts.master_pda.to_account_info(),
        task_queue: ctx.accounts.task_queue.to_account_info(),
        task_queue_name_mapping: ctx.accounts.task_queue_name_mapping.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };
    let cpi_prog = ctx.accounts.tuktuk_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_prog, cpi_accounts);
    let args = InitializeTaskQueueArgsV0 {
        min_crank_reward: 10000,
        name: task_queue_name,
        capacity: 5000,
        lookup_tables: vec![],
        // Cliff & Vesting schedule is 4 years, not sure this is the way
        stale_task_age: 4 * 365 * 24 * 60 * 60,
    };

    if let Err(e) = initialize_task_queue_v0(cpi_ctx, args) {
        msg!("Failed to initialize Tuktuk task queue!");
        return Err(e);
    }

    let cpi_accounts = AddQueueAuthorityV0 {
        payer: ctx.accounts.master.to_account_info(),
        update_authority: ctx.accounts.master_pda.to_account_info(),
        queue_authority: ctx.accounts.master.to_account_info(),
        task_queue_authority: ctx.accounts.task_queue_authority.to_account_info(),
        task_queue: ctx.accounts.task_queue.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };
    let cpi_prog = ctx.accounts.tuktuk_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_prog, cpi_accounts, signer_seeds);
    if let Err(e) = add_queue_authority_v0(cpi_ctx) {
        msg!("Failed to set master wallet as Tuktuk queue authority!");
        return Err(e);
    }

    Ok(())
}

pub fn initialize_tuktuk<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeTuktuk<'info>>,
    task_queue_name: String,
) -> Result<()> {
    let master_seeds = &[b"master".as_ref(), &[ctx.bumps.master_pda]];
    let signer_seeds = &[&master_seeds[..]];

    create_tuktuk_task_queue(
        &ctx,
        &ctx.accounts.tuktuk_config.to_account_info(),
        task_queue_name,
        signer_seeds,
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(task_queue_name: String)]
pub struct InitializeTuktuk<'info> {
    #[account(mut, signer, address = crate::MASTER_WALLET)]
    pub master: Signer<'info>,

    #[account(
        mut,
        seeds = [b"master"],
        bump
    )]
    /// CHECK: pda authority
    pub master_pda: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: will be initialized
    pub master_ata: UncheckedAccount<'info>,

    #[account(
        seeds = [
            b"task_queue",
            tuktuk_config.key().as_ref(),
            &tuktuk_config.next_task_queue_id.to_le_bytes()[..]
        ],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    pub task_queue: AccountInfo<'info>,
    /// CHECK: should be created by the init task queue ix,
    /// but doesn't exist before invocation.
    #[account(
        seeds = [
            "task_queue_name_mapping".as_bytes(),
            tuktuk_config.key().as_ref(),
            &hash(task_queue_name.as_bytes()).to_bytes()
        ],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    pub task_queue_name_mapping: UncheckedAccount<'info>,
    /// CHECK: should be created by the init task queue ix,
    /// but doesn't exist before invocation.
    #[account(
        seeds = [
            b"task_queue_authority",
            task_queue.key().as_ref(),
            master_pda.key().as_ref()
        ],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    pub task_queue_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"tuktuk_config"],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    pub tuktuk_config: Account<'info, TuktukConfigV0>,
    pub tuktuk_program: Program<'info, Tuktuk>,

    #[account(mut, mint::authority = master_pda)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
