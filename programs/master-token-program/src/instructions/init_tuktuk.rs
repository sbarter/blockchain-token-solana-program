use anchor_lang::prelude::*;
use tuktuk_program::{
    tuktuk::{
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
    task_queue_name: String,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
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
        queue_authority: ctx.accounts.master_pda.to_account_info(),
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

    create_tuktuk_task_queue(&ctx, task_queue_name, signer_seeds)?;

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
    /// CHECK: initialized by init
    pub master_ata: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: will be created
    pub task_queue: AccountInfo<'info>,
    /// CHECK: should be created by the init task queue ix,
    /// but doesn't exist before invocation.
    #[account(mut)]
    /// CHECK: should be created by the init task queue ix,
    /// but doesn't exist before invocation.
    pub task_queue_name_mapping: UncheckedAccount<'info>,
    /// CHECK: should be created by the init task queue ix,
    /// but doesn't exist before invocation.
    #[account(mut)]
    /// CHECK: will be created
    pub task_queue_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"tuktuk_config"],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    pub tuktuk_config: Account<'info, TuktukConfigV0>,
    pub tuktuk_program: Program<'info, Tuktuk>,
    pub system_program: Program<'info, System>,
}
