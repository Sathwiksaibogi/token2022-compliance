use std::{
    rc::Rc,
    str::FromStr,
};

use anchor_client::{
    Client,
    Cluster,
    Signer,
};

use anchor_lang::prelude::Pubkey;

use solana_keypair::read_keypair_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ------------------------------------------------------------
    // 1. Read RPC URL
    // ------------------------------------------------------------

    let rpc_url =
        std::env::var("DEVNET_RPC")
            .expect("DEVNET_RPC environment variable is not set");

    println!("Using RPC endpoint");

    let cluster = Cluster::from_str(&rpc_url)?;

    // ------------------------------------------------------------
    // 2. Load admin/deployer wallet
    // ------------------------------------------------------------

    let home =
        std::env::var("HOME")
            .expect("HOME environment variable is not set");

    let wallet_path =
        format!("{home}/.config/solana/id.json");

    let payer =
        read_keypair_file(&wallet_path)
            .map_err(|err| {
                format!("failed to read wallet {wallet_path}: {err}")
            })?;

    let admin_pubkey = payer.pubkey();

    println!("Admin: {}", admin_pubkey);

    // ------------------------------------------------------------
    // 3. Create Anchor client
    // ------------------------------------------------------------

    let client =
        Client::new(
            cluster,
            Rc::new(payer),
        );

    let program =
        client.program(token2022_compliance::ID)?;

    println!(
        "Compliance program: {}",
        token2022_compliance::ID
    );

    // ------------------------------------------------------------
    // 4. Main Token-2022 mint
    // ------------------------------------------------------------

    let mint = Pubkey::from_str(
        "CqLtUZQBK9phoD33ye6sQQG2EdoMrEvraHgijwpy9ZDF",
    )?;

    println!("Mint: {}", mint);

    // ------------------------------------------------------------
    // 5. Derive canonical GlobalPolicy PDA
    //
    // seeds = ["policy", mint]
    // ------------------------------------------------------------

    let (global_policy_pda, bump) =
        Pubkey::find_program_address(
            &[
                b"policy",
                mint.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    println!(
        "GlobalPolicy PDA: {}",
        global_policy_pda
    );

    println!("GlobalPolicy bump: {}", bump);

    // Safety check against the PDA we already derived using CLI.
    let expected_policy_pda = Pubkey::from_str(
        "EmoBiCLorusZGANzxWfhb2brvTMu2pQFGMquxu466Yh8",
    )?;

    assert_eq!(
        global_policy_pda,
        expected_policy_pda,
        "derived GlobalPolicy PDA does not match expected PDA"
    );

    // ------------------------------------------------------------
    // 6. Initial compliance configuration
    //
    // decimals = 6
    //
    // 100 tokens = 100_000_000 base units
    // 250 tokens = 250_000_000 base units
    // ------------------------------------------------------------

    let max_transfer_amount: u64 =
        100 * 1_000_000;

    let daily_transfer_limit: u64 =
        250 * 1_000_000;

    let expires_at: i64 = 0;

    println!(
        "Initializing policy:"
    );

    println!(
        "  max transfer = {}",
        max_transfer_amount
    );

    println!(
        "  daily limit = {}",
        daily_transfer_limit
    );

    println!(
        "  expires_at = {}",
        expires_at
    );

    // ------------------------------------------------------------
    // 7. Send initialize_policy to Devnet
    // ------------------------------------------------------------

    let signature = program
        .request()
        .accounts(
            token2022_compliance::accounts::InitializePolicy {
                admin: admin_pubkey,
                mint,
                global_policy: global_policy_pda,
                token_program: anchor_spl::token_2022::ID,
                system_program: anchor_lang::system_program::ID,
            },
        )
        .args(
            token2022_compliance::instruction::InitializePolicy {
                max_transfer_amount,
                daily_transfer_limit,
                expires_at,
            },
        )
        .send()?;

    println!();
    println!("GlobalPolicy initialized successfully.");
    println!("Signature: {}", signature);
    println!("Policy PDA: {}", global_policy_pda);

    Ok(())
}