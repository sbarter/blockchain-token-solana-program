use anchor_lang::{
    prelude::*,
    solana_program::{hash::hash, instruction::Instruction},
    InstructionData,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};
use tuktuk_program::{
    compile_transaction,
    tuktuk::{
        cpi::{accounts::QueueTaskV0, queue_task_v0},
        program::Tuktuk,
    },
    types::QueueTaskArgsV0,
    CompiledTransactionV0, TransactionSourceV0, TuktukConfigV0,
};

use crate::{
    states::{Investor, InvestorCategoryData},
    SBT_DECIMALS, VESTING_MONTH,
};

fn reschedule_itself<'info>(
    ctx: Context<'_, '_, '_, 'info, TuktukAutoClaim<'info>>,
    category_seed: String,
    investor_index: u32,
    task_id: u16,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let ix = crate::instruction::InvestorAutoClaim {
        category_seed,
        investor_index,
        task_id: task_id + 1,
    };
    let mut accounts = ctx.accounts;
    accounts.task = todo!();
    let (compiled_tx, _) = compile_transaction(
        vec![Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(None).to_vec(),
            data: ix.data(),
        }],
        vec![vec![b"master".to_vec(), vec![ctx.bumps.master_pda]]],
    )
    .unwrap();

    let cpi_accounts = QueueTaskV0 {
        queue_authority: ctx.accounts.master_pda.to_account_info(),
        task_queue_authority: ctx.accounts.master_pda.to_account_info(),
        task_queue: ctx.accounts.task_queue.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        payer: ctx.accounts.task_queue.to_account_info(),
        task: ctx.accounts.task.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    let args = QueueTaskArgsV0 {
        id: task_id + 1,
        trigger: tuktuk_program::TriggerV0::Timestamp(now + VESTING_MONTH as i64),
        transaction: TransactionSourceV0::CompiledV0(compiled_tx),
        crank_reward: Some(10000),
        free_tasks: 1,
        description: String::new(),
    };
    queue_task_v0(cpi_ctx, args)
}

pub fn tuktuk_claim_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, TuktukAutoClaim<'info>>,
    category_seed: String,
    investor_index: u32,
    task_id: u16,
) -> Result<()> {
    let category = &ctx.accounts.category;
    let investor = &mut ctx.accounts.investor_pda;

    let master_seeds = &[
        category_seed.as_bytes(),
        &ctx.accounts.mint.key().to_bytes(),
        &[ctx.bumps.category],
    ];
    let signer_seeds = &[&master_seeds[..]];

    let cliff_pre = category.cliff_months_remaining;
    let vesting_pre = category.vesting_months_remaining;

    let now = Clock::get()?.unix_timestamp as u64;
    require!(
        category.cliff_started_at != 0,
        crate::error::ErrorCode::TgeNotHappened
    );

    let since_tge = now.saturating_sub(category.cliff_started_at);
    let months_elapsed = (since_tge / VESTING_MONTH) as u8;
    let total_months = months_elapsed.saturating_sub(investor.months_claimed);

    if total_months == 0 {
        msg!("No claim available.");
        return Ok(());
    }

    let mut total_tokens = 0;
    for _ in 0..total_months {
        if !investor.first_month_skipped {
            investor.first_month_skipped = true;
            continue;
        }
        if investor.vesting_months_remaining == 0 {
            break;
        }
        if investor.cliff_months_remaining > 0 {
            investor.cliff_months_remaining -= 1;
            continue;
        }
        if investor.cliff_months_remaining == 0 && investor.vesting_months_remaining > 0 {
            total_tokens += investor.monthly_allocation;
            investor.vesting_months_remaining -= 1;
            continue;
        }
    }
    if total_tokens > 0 {
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.category_ata.to_account_info(),
            to: ctx.accounts.investor_ata.to_account_info(),
            authority: ctx.accounts.category.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        let transfer = token_2022::transfer_checked(cpi_ctx, total_tokens, SBT_DECIMALS as u8);
        if transfer.is_err() {
            msg!("Unable to transfer tokens from the category. You can always try again.");
            investor.cliff_months_remaining = cliff_pre;
            investor.vesting_months_remaining = vesting_pre;
            transfer?;
        }
    }
    investor.months_claimed += total_months;

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, investor_index: u32, task_queue_name: String, task_id: u16)]
pub struct TuktukAutoClaim<'info> {
    #[account(
        mut,
        seeds = [b"master"],
        bump
    )]
    /// CHECK: pda authority
    pub master_pda: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [category_seed.as_bytes(), investor_index.to_le_bytes().as_ref(),
        mint.key().as_ref()],
        bump
    )]
    pub investor_pda: Account<'info, Investor>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = investor_pda.wallet,
        associated_token::token_program = token_program
    )]
    pub investor_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [category_seed.as_bytes(), mint.key().as_ref()],
        bump
    )]
    pub category: Account<'info, InvestorCategoryData>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = category,
        associated_token::token_program = token_program
    )]
    pub category_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [b"task", task_queue.key().as_ref(), &(task_id + 1).to_le_bytes()],
        bump
    )]
    /// CHECK: Will be created
    pub task: UncheckedAccount<'info>,
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

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
