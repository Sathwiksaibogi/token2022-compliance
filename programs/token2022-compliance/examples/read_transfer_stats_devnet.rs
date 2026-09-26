use std::{
    rc::Rc,
    str::FromStr,
};

use anchor_client::{
    Client,
    Cluster,
};

use anchor_lang::prelude::Pubkey;

use solana_keypair::read_keypair_file;

use token2022_compliance::state::TransferStats;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url =
        std::env::var("DEVNET_RPC")
            .expect("DEVNET_RPC is not set");

    let cluster = Cluster::from_str(&rpc_url)?;

    let home =
        std::env::var("HOME")
            .expect("HOME is not set");

    let wallet_path =
        format!("{home}/.config/solana/id.json");

    let payer =
        read_keypair_file(wallet_path)
            .map_err(|err| {
                format!("failed to read wallet: {err}")
            })?;

    let client =
        Client::new(
            cluster,
            Rc::new(payer),
        );

    let program =
        client.program(token2022_compliance::ID)?;

    let mint = Pubkey::from_str(
        "CqLtUZQBK9phoD33ye6sQQG2EdoMrEvraHgijwpy9ZDF",
    )?;

    let alice = Pubkey::from_str(
        "28eQwy3xZsPmMNUeHmvSp2no4aAndmk4EQjXoTm8KHo2",
    )?;

    let (stats_pda, bump) =
        Pubkey::find_program_address(
            &[
                b"stats",
                mint.as_ref(),
                alice.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    println!("Alice TransferStats PDA: {}", stats_pda);
    println!("Derived bump: {}", bump);

    let stats: TransferStats =
        program.account(stats_pda)?;

    println!();
    println!("========== ALICE TRANSFER STATS ==========");
    println!("Mint:              {}", stats.mint);
    println!("Wallet:            {}", stats.wallet);
    println!("Day index:         {}", stats.day_index);
    println!("Amount today:      {}", stats.amount_today);
    println!("Total transferred: {}", stats.total_transferred);
    println!("Transfer count:    {}", stats.transfer_count);
    println!("Stored bump:       {}", stats.bump);

    assert_eq!(stats.mint, mint);
    assert_eq!(stats.wallet, alice);

    assert_eq!(
        stats.amount_today,
        40_000_000,
        "unexpected amount_today"
    );

    assert_eq!(
        stats.total_transferred,
        40_000_000,
        "unexpected total_transferred"
    );

    assert_eq!(
        stats.transfer_count,
        1,
        "unexpected transfer_count"
    );

    println!();
    println!("✅ TransferStats verified successfully");

    Ok(())
}