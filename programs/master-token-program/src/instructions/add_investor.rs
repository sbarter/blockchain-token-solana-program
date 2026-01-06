use anchor_lang::{InstructionData, prelude::*, solana_program::{hash::hash, instruction::Instruction}};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};
use tuktuk_program::{TransactionSourceV0, TuktukConfigV0, compile_transaction, tuktuk::{cpi::{accounts::QueueTaskV0, queue_task_v0}, program::Tuktuk}, types::QueueTaskArgsV0};

use crate::{VESTING_MONTH, states::{Investor, InvestorCategoryData}};

fn schedule_autoclaim<'info>(
    ctx: Context<'_, '_, '_, 'info, AddInvestorToCategory<'info>>,
    timestamp: i64,
    category_seed: String,
    investor_index: u16,
    task_queue_name: String
) -> Result<()> {
    let master_seeds = &[b"master".as_ref(), &[ctx.bumps.master_pda]];
    let signer_seeds = &[&master_seeds[..]];
    
    let ix = crate::instruction::InvestorAutoClaim {
        category_seed,
        investor_index,
        task_queue_name
    };

    let (compiled_tx, _) = compile_transaction(
        vec![Instruction {
            program_id: crate::ID,
            accounts: ctx.accounts.to_account_metas(None).to_vec(),
            data: ix.data(),
        }],
        vec![vec![b"master".to_vec(), vec![ctx.bumps.master_pda]]],
    )
    .unwrap();

    let cpi_accounts = QueueTaskV0 {
        queue_authority: ctx.accounts.master_pda.to_account_info(),
        task_queue_authority: ctx.accounts.task_queue_authority.to_account_info(),
        task_queue: ctx.accounts.task_queue.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        payer: ctx.accounts.task_queue.to_account_info(),
        task: ctx.accounts.next_task.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    let args = QueueTaskArgsV0 {
        id: investor_index,
        trigger: tuktuk_program::TriggerV0::Timestamp(timestamp),
        transaction: TransactionSourceV0::CompiledV0(compiled_tx),
        crank_reward: Some(10000),
        free_tasks: 1,
        description: String::new(),
    };
    queue_task_v0(cpi_ctx, args)
}

pub fn add_investor_to_category<'info>(
    ctx: Context<'_, '_, '_, 'info, AddInvestorToCategory<'info>>,
    category_seed: String,
    new_investor_index: u16,
    monthly_allocation: u64,
    task_queue_name: String,
) -> Result<()> {
    let investor = &mut ctx.accounts.investor_pda;
    let category = &mut ctx.accounts.category;

    require!(category.is_open || category.cliff_started_at == 0, crate::error::ErrorCode::CategoryClosed);
    require_eq!(category.investor_count + 1, new_investor_index, crate::error::ErrorCode::InvestorIndex);
    require_gte!(
        category.unallocated_total_tokens,
        monthly_allocation * category.vesting_months_remaining as u64,
        crate::error::ErrorCode::TooManyTokensAllocated
    );

    
    investor.wallet = ctx.accounts.investor_wallet.key();
    // has to wait an extra month if joined during vesting
    investor.first_month_skipped = category.cliff_months_remaining > 0;
    investor.monthly_allocation = monthly_allocation;
    investor.months_claimed = 0;
    investor.cliff_months_remaining = category.cliff_months_remaining;
    investor.vesting_months_remaining = category.vesting_months_remaining;

    category.investor_count += 1;
    category.unallocated_total_tokens -= monthly_allocation * category.vesting_months_remaining as u64;

    let cliff_started_at = category.cliff_started_at;
    let next_cycle = {
        let now = Clock::get()?.unix_timestamp;
        let since_tge = now.saturating_sub(category.cliff_started_at as i64);
        let months_elapsed = (since_tge / VESTING_MONTH as i64) as u8;
        cliff_started_at as i64 + (VESTING_MONTH as i64 * (months_elapsed + 1) as i64)
    };
    
    if let Err(e) = schedule_autoclaim(
        ctx,
        next_cycle,
        category_seed,
        new_investor_index,
        task_queue_name
    ) {
        msg!("Failed to schedule autoclaim for investor!");
        msg!(&format!("{e}"));
    }
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, new_investor_index: u16, monthly_allocation: u64, task_queue_name: String)]
pub struct AddInvestorToCategory<'info> {
    #[account(mut, signer, address = crate::MASTER_WALLET)]
    pub master: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"master"],
        bump
    )]
    /// CHECK: pda authority
    pub master_pda: AccountInfo<'info>,

    #[account(
        init,
        payer = master,
        seeds = [category_seed.as_bytes(), new_investor_index.to_le_bytes().as_ref(),
        mint.key().as_ref()],
        space = Investor::LEN,
        bump
    )]
    pub investor_pda: Account<'info, Investor>,
    
    /// CHECK: Frankly we don't care if it's funded or anything.
    pub investor_wallet: UncheckedAccount<'info>,
    
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint, 
        associated_token::authority = investor_wallet, 
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
        seeds = [b"task", task_queue.key().as_ref(), &(new_investor_index).to_le_bytes()],
        bump
    )]
    /// CHECK: Will be created
    pub next_task: UncheckedAccount<'info>,
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
    #[account(
        seeds = [
            "task_queue_name_mapping".as_bytes(),
            tuktuk_config.key().as_ref(),
            &hash(task_queue_name.as_bytes()).to_bytes()
        ],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    pub task_queue_name_mapping: AccountInfo<'info>,
    #[account(
        seeds = [
            b"task_queue_authority",
            task_queue.key().as_ref(),
            master_pda.key().as_ref()
        ],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    pub task_queue_authority: AccountInfo<'info>,
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
