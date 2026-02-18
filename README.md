# Sbarter Token Program

The Solana program responsible for TGE and distribution of SBT tokens to categories
and investors according to the cliff & vesting schedule.

Category PDAs are created on initialization, then investors are added
one-by-one, and the TGE instruction is triggered. All of the above are signed
by the Sbarter Multisig Wallet.

The program also exposes two permissionless instructions:
`category_transfer_vestings` and `investor_claim_tokens`, that can be run
manually by anybody incentivized or by the first-party Automation Cronjob.

## How to test

1. Build the program:

```bash
anchor build -- --features local-testing
```

2. Run solana-test-validator:

```bash
solana-test-validator -r \
    --bpf-program 5LnwuNSM9TKgr69YXoLCdCdoZ7SZ1kvtYAdknPGSJ3KX target/sbpf-solana-solana/release/master_token_program.so \
    --clone TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb \
    --clone ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL \
    --url devnet
```

3. Run tests:

```bash
anchor test --skip-local-validator --skip-deploy -- --features local-testing
```

(`anchor test` normally loads programs automagically, but it never ever works
for me.)

- You will need a keypair at `~/.config/solana/id.json` to run it.
  It will be used as an example master authority, instead of the Multisig.
- Also consider replacing the public RPC URL with a dedicated one.

## Key flow sequences

1. Initial setup and TGE

```mermaid
sequenceDiagram
    title TGE – Setup, Minting & TGE Release (per allocation config)

    participant Admin as Master / Sbarter Association Multisig
    participant Program as TGE Program
    participant MasterPda as Master PDA Vault
    participant Mint as SBT Token Mint
    participant Category as Category PDAs
    participant Investor as Individual Investors

    Admin->>Program: initialize_mint(mint)
    Program->>Mint: create_account()
    Program->>Mint: create_mint_metadata()
    Program-->>Admin: initialize_mint success
    Admin->>Program: initialize_investor_categories(mint, category_addresses[pre_seed..vgp], vault)
    Admin->>Program: initialize_func_categories(mint, category_addresses[marketing..liquidity], vault)
    Program->>MasterPda: create_master_vault()

    loop For each allocation category
        Program->>Category: initialize_category(cliff, vesting, monthly_allocation,...)
        Category-->>Program: initialized
    end
    Program-->>Admin: initialized

    loop For each individual investor
        Admin->>Program: category_add_investor(category, investor_id, allocation)
        Program->>Investor: create_investor_pda(category, investor_id, allocation)
        Program->>Category: investor_count++
    end

    %% 3. At TGE moment
    Admin->>Program: trigger_TGE()
    Program-->>Program: check_all_investors_initialized(pre_seed, seed)
    Program->>Mint: mint_to_vault(25B)
    Mint-->>MasterPda: minted
    Program->>Mint: set_authority(None)
    Program->>Category: Marketing: transfer(initial_supply)
    Program->>Category: Reserve: transfer(initial_supply)
    Program->>Category: Liquidity: transfer(initial_supply)

    Program-->>Admin: TGE successful
```

2. Token claims

```mermaid
sequenceDiagram
    title Category and Investor level – Automated Monthly Airdrops

    participant Cronjob as Claiming Cronjob
    participant Program as TGE Program
    participant Category as Category PDA
    participant Clock as Solana Clock Sysvar
    participant Investor as Individual Investor

    Cronjob->>Program: category_transfer_vestings(categories) (permissionless)
    loop For each category
        Program->>Clock: get_current_time()
        Clock-->>Program: now
        Program->>Category: get_account()
        Category-->>Program: TGE timestamp, allocation, months claimed
        Program-->>Program: calculate_available_claim(now, tge, months_claimed)
        opt available > 0
            Program->>Category: transfer(available)
        end
    end
    Program-->>Cronjob: category_transfer_vestings successful

    loop For each investor (category, investor_id)
        Cronjob->>Program: investor_claim_tokens(category, investor_id) (permissionless)
        Program->>Clock: get_current_time()
        Clock-->>Program: now
        Program->>Category: get_account()
        Category-->>Program: TGE timestamp
        Program->>Investor: get_account()
        Investor-->>Program: allocation, months claimed
        Program-->>Program: calculate_available_claim(now, tge, months_claimed)
        opt available > 0
            Program->>Investor: transfer(available)
        end
        Program-->>Cronjob: investor_claim_tokens successful
    end

    Note over Program,Investor: TGE portion was already transferred. This flow only handles the remaining locked tokens.
```

3. Manual token management

```mermaid
sequenceDiagram
    title Manual token management

    participant Admin as Master / Sbarter Association Multisig
    participant Manager as Manager Wallet
    participant Program as TGE Program
    participant Category1 as Category 1 PDA
    participant Category2 as Category 2 PDA

    Admin->>Program: category_withdraw_tokens(category1, amount, recipient)
    Program->>Program: check(amount <= total_unallocated_tokens)
    Program->>Category1: get_account()
    Category1-->>Program: category_balance, category_unclaimed
    Program->>Program: check(amount <= (category_balance - category_unclaimed))
    Program->>Manager: transfer(amount)
    Program-->>Admin: category_withdraw_tokens successful

    Manager->>Program: category_deposit_tokens(category2, amount)
    Program->>Category2: transfer(amount)
    Program-->>Manager: category_deposit_tokens successful
```
