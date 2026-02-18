use anchor_lang::prelude::*;

const UPGRADE_AUTHORITY: Pubkey =
    Pubkey::from_str_const("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH");

pub fn close_accounts<'info>(ctx: Context<'_, '_, '_, 'info, CloseAccounts<'info>>) -> Result<()> {
    let authority = &ctx.accounts.authority;
    for account in ctx.remaining_accounts.iter() {
        require!(
            account.owner == ctx.program_id,
            crate::error::ErrorCode::AtaMismatch
        );

        let account_lamports = account.lamports();
        **account.try_borrow_mut_lamports()? -= account_lamports;
        **authority.try_borrow_mut_lamports()? += account_lamports;

        let mut data = account.try_borrow_mut_data()?;
        data.fill(0);
        data[0..8].fill(255);

        account.assign(&System::id());
    }

    Ok(())
}

#[derive(Accounts)]
pub struct CloseAccounts<'info> {
    #[account(mut, address = UPGRADE_AUTHORITY)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
