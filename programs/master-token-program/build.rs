fn main() {
    let local_testing = cfg!(feature = "local-testing");
    let mainnet_testing = cfg!(feature = "mainnet-testing");

    println!("cargo:warning=--- sbarter-token-programs build summary ---");

    // Program ID
    println!("cargo:warning=  program id: sbtBrARdYZQcL1JBKj3FDSQyvHrtFHoeww3yKY7Sqer");

    // Investor category pre_investors
    println!("cargo:warning=  preseed pre_investors: 68");
    println!("cargo:warning=  seed pre_investors: 96");

    // Functional category authorities
    println!(
        "cargo:warning=  marketing authority: 2GRnFCAkd8Smm8uJ2zFhZQgjCPgi341MzU9FS2U3De2q"
    );
    println!(
        "cargo:warning=  reserve authority: 3kGsEXbQxWjNoVTZ7og1CVivPkBxAjtBPJYuSUn69eWi"
    );
    println!(
        "cargo:warning=  liquidity authority: 6RQboL2DeTM8jUQubgCYLSHZMJSLbUTRQZTEL2jjDa1M"
    );

    // Vesting month epoch (conditional)
    let (vesting_month, label) = if local_testing {
        (10u64, "local-testing")
    } else if mainnet_testing {
        (900, "mainnet-testing")
    } else {
        (30 * 24 * 60 * 60, "production")
    };
    println!("cargo:warning=  vesting_month: {vesting_month}s ({label})");

    // Conditional features
    if local_testing {
        println!("cargo:warning=  [local-testing] initialize_mint instruction EXPOSED");
        println!("cargo:warning=  [local-testing] category claim sync enforcement DISABLED");
        println!("cargo:warning=  [local-testing] investor vesting calculation SKIPPED");
    }

    if mainnet_testing {
        println!("cargo:warning=  [mainnet-testing] using devnet metadata URL");
    } else {
        println!("cargo:warning=  using production metadata URL");
    }

    println!("cargo:warning=--- end build summary ---");
}
