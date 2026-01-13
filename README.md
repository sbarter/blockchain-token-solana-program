# Sbarter Token Program

The Solana program responsible for TGE and distribution of SBT tokens to categories
and investors according to the cliff & vesting schedule.

## How to test

1. Build the program:

```bash
anchor build
```

2. Run solana-test-validator:

```bash
solana-test-validator -r \
    --bpf-program 47D4TsSiMjG4s2ohbuvQXZEtwYeJ5VPDJaDiBUNxpm8y target/sbpf-solana-solana/release/master_token_program.so \
    --clone TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb \
    --clone ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL \
    --url devnet
```

3. Run tests:

```bash
anchor test --skip-local-validator --skip-deploy
```

(`anchor test` normally loads programs automagically, but it never ever works
for me.)

## Key flow sequences

1. Initial setup and TGE

```mermaid
sequenceDiagram
    title TGE – Setup, Minting & TGE Release (per allocation config)

    participant Admin as Master / Sbarter Association
    participant Program as TGE Program
    participant MasterPda as Master PDA Vault
    participant Mint as SBT Token Mint
    participant Category as Category PDAs
    participant Investor as Individual Investors

    %% 1. Pre-TGE: configure sale and allocations
    Admin->>Program: initialize(mint, categoryAddresses[PreSeed..Liquidity], vault)
    Program->>MasterPda: create_master_vault()

    %% 2. Pre-TGE: create vesting PDAs & escrow full allocations
    loop For each allocation category
        Program->>Category: initialize_category(cliff, vesting, monthlyAllocation,...)
        Category-->>Program: initialized
    end
    Program-->>Admin: initialized

    loop For each individual investor
        Admin->>Program: add_investor(category, investor_id, allocation)
        Program->>Investor: create_investor_pda(category, investor_id, allocation)
        Program->>Category: investor_count++
    end

    %% 3. At TGE moment
    Admin->>Program: trigger_TGE()
    Program-->>Program: check_all_investors_initialized(preSeed, Seed)
    Program->>Mint: mint_to_vault(25B)
    Mint-->>MasterPda: minted
    Program->>Mint: set_authority(None)
    Program->>Category: Marketing: transfer(initialSupply)
    Program->>Category: Reserve: transfer(initialSupply)
    Program->>Category: Liquidity: transfer(initialSupply)

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

    Cronjob->>Program: transfer_category_vestings(categories) (permissionless)
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
    Program-->>Cronjob: transfer_categorry_vesting successful

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

    participant Admin as Master / Sbarter Association
    participant Manager as Manager Wallet
    participant Program as TGE Program
    participant Category1 as Category 1 PDA
    participant Category2 as Category 2 PDA

    Admin->>Program: withdraw_category_tokens(category1, amount, recipient)
    Program->>Program: check(amount <= total_unallocated_tokens)
    Program->>Category1: get_account()
    Category1-->>Program: category_balance, category_unclaimed
    Program->>Program: check(amount <= (category_balance - category_unclaimed))
    Program->>Manager: transfer(amount)
    Program-->>Admin: withdraw_category_tokens successful

    Manager->>Program: deposit_category_tokens(category2, amount)
    Program->>Category2: transfer(amount)
    Program-->>Manager: deposit_category_tokens successful
```
