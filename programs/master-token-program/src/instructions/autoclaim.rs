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
        self,
        cpi::{accounts::QueueTaskV0, queue_task_v0},
        program::Tuktuk,
    },
    types::QueueTaskArgsV0,
    TransactionSourceV0, TuktukConfigV0,
};

use crate::{
    states::{Investor, InvestorCategoryData, INVESTOR_CATEGORY_SEEDS},
    SBT_DECIMALS, VESTING_MONTH,
};

pub fn derive_task_pubkey(
    category_seed: &str,
    investor_index: u16,
    task_queue: &Pubkey,
    flipped: bool,
) -> Result<Pubkey> {
    const U16_MSB: u16 = 0x8000;
    const CATEGORY_BITMASK: u16 = 0x7000;

    let Some(category_id) = INVESTOR_CATEGORY_SEEDS
        .iter()
        .position(|&seed| seed == category_seed.as_bytes())
    else {
        return Err(crate::error::ErrorCode::CategorySeed.into());
    };

    // bit structure: FCCC0000 00000000
    // F - flipped, so that a task can queue itself while existing
    // C - 0-7 unique category id, so that task ids between categories don't clash
    // the rest is investor index
    let task_id = {
        let mut task_id = investor_index;
        if flipped {
            task_id ^= U16_MSB;
        }
        task_id |= ((category_id as u16) << 12) & CATEGORY_BITMASK;
        task_id
    };
    Ok(Pubkey::find_program_address(
        &[b"task", task_queue.as_ref(), &task_id.to_le_bytes()],
        &tuktuk::ID,
    )
    .0)
}

fn schedule_autoclaim<'info>(
    ctx: Context<'_, '_, '_, 'info, TuktukAutoClaim<'info>>,
    timestamp: i64,
    category_seed: String,
    investor_index: u16,
    flipped: bool,
    task_queue_name: String,
) -> Result<()> {
    let master_seeds = &[b"master".as_ref(), &[ctx.bumps.master_pda]];
    let signer_seeds = &[&master_seeds[..]];

    let ix = crate::instruction::InvestorAutoClaim {
        category_seed,
        investor_index,
        flipped: !flipped,
        task_queue_name,
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

    let next_task = if flipped {
        ctx.accounts.next_task_flipped.to_account_info()
    } else {
        ctx.accounts.next_task.to_account_info()
    };
    let cpi_accounts = QueueTaskV0 {
        queue_authority: ctx.accounts.master_pda.to_account_info(),
        task_queue_authority: ctx.accounts.task_queue_authority.to_account_info(),
        task_queue: ctx.accounts.task_queue.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        payer: ctx.accounts.master_pda.to_account_info(),
        task: next_task,
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.tuktuk_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    let args = QueueTaskArgsV0 {
        id: investor_index,
        trigger: tuktuk_program::TriggerV0::Timestamp(timestamp),
        transaction: TransactionSourceV0::CompiledV0(compiled_tx),
        crank_reward: None,
        free_tasks: 1,
        description: String::new(),
    };
    queue_task_v0(cpi_ctx, args)
}

pub fn tuktuk_claim_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, TuktukAutoClaim<'info>>,
    category_seed: String,
    investor_index: u16,
    flipped: bool,
    task_queue_name: String,
) -> Result<()> {
    let seed_bytes = category_seed.clone();
    let category = &ctx.accounts.category;
    let investor = &mut ctx.accounts.investor_pda;

    let (expected_mapping, _) = Pubkey::find_program_address(
        &[
            "task_queue_name_mapping".as_bytes(),
            ctx.accounts.tuktuk_config.key().as_ref(),
            &hash(task_queue_name.as_bytes()).to_bytes(),
        ],
        &ctx.accounts.tuktuk_program.key(),
    );
    require_keys_eq!(ctx.accounts.task_queue_name_mapping.key(), expected_mapping);

    let Ok(expected_next_task) = derive_task_pubkey(
        &category_seed,
        investor_index,
        &ctx.accounts.task_queue.key(),
        flipped,
    ) else {
        return Err(crate::error::ErrorCode::TuktukTaskId.into());
    };
    require_keys_eq!(
        expected_next_task,
        ctx.accounts.next_task.key(),
        crate::error::ErrorCode::TuktukTaskId
    );

    let Ok(expected_next_task_flipped) = derive_task_pubkey(
        &category_seed,
        investor_index,
        &ctx.accounts.task_queue.key(),
        !flipped,
    ) else {
        return Err(crate::error::ErrorCode::TuktukTaskId.into());
    };
    require_keys_eq!(
        expected_next_task_flipped,
        ctx.accounts.next_task_flipped.key(),
        crate::error::ErrorCode::TuktukTaskId
    );

    let master_seeds = &[
        seed_bytes.as_bytes(),
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
        investor_index,
        flipped,
        task_queue_name,
    ) {
        msg!("Failed to reschedule autoclaim!");
        msg!(&format!("{e}"));
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, investor_index: u16, flipped: bool, task_queue_name: String)]
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

    #[account(mut)]
    /// CHECK: Will be created
    pub next_task: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Will be created
    pub next_task_flipped: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: created by init tuktuk
    pub task_queue: AccountInfo<'info>,
    #[account()]
    /// CHECK: created by init tuktuk
    pub task_queue_name_mapping: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            b"task_queue_authority",
            task_queue.key().as_ref(),
            master_pda.key().as_ref()
        ],
        bump,
        seeds::program = tuktuk_program.key()
    )]
    /// CHECK: created by init tuktuk
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
