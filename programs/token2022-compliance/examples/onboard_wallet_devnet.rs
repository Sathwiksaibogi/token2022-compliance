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

use token2022_compliance::state::AuthorizationStatus;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ------------------------------------------------------------
    // Target browser wallet
    // ------------------------------------------------------------

    let wallet_string =
        std::env::var("BROWSER_WALLET")
            .expect("BROWSER_WALLET is not set");

    let target_wallet =
        Pubkey::from_str(&wallet_string)?;

    // ------------------------------------------------------------
    // RPC
    // ------------------------------------------------------------

    let rpc_url =
        std::env::var("DEVNET_RPC")
            .expect("DEVNET_RPC is not set");

    let cluster =
        Cluster::from_str(&rpc_url)?;

    // ------------------------------------------------------------
    // Admin wallet
    // ------------------------------------------------------------

    let home =
        std::env::var("HOME")
            .expect("HOME is not set");

    let admin_path =
        format!("{home}/.config/solana/id.json");

    let payer =
        read_keypair_file(&admin_path)
            .map_err(|err| {
                format!("failed to read admin wallet: {err}")
            })?;

    let admin =
        payer.pubkey();

    let client =
        Client::new(
            cluster,
            Rc::new(payer),
        );

    let program =
        client.program(token2022_compliance::ID)?;

    // ------------------------------------------------------------
    // Mint
    // ------------------------------------------------------------

    let mint =
        Pubkey::from_str(
            "CqLtUZQBK9phoD33ye6sQQG2EdoMrEvraHgijwpy9ZDF",
        )?;

    // ------------------------------------------------------------
    // PDAs
    // ------------------------------------------------------------

    let (global_policy, _) =
        Pubkey::find_program_address(
            &[
                b"policy",
                mint.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    let (authorization, _) =
        Pubkey::find_program_address(
            &[
                b"authorization",
                mint.as_ref(),
                target_wallet.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    let (transfer_stats, _) =
        Pubkey::find_program_address(
            &[
                b"stats",
                mint.as_ref(),
                target_wallet.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    println!("Admin:              {}", admin);
    println!("Target wallet:      {}", target_wallet);
    println!("GlobalPolicy:        {}", global_policy);
    println!("Authorization PDA:   {}", authorization);
    println!("TransferStats PDA:   {}", transfer_stats);

    // ============================================================
    // 1. Initialize Authorization
    // ============================================================

    let signature =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::InitializeAuthorization {
                    admin,
                    mint,
                    global_policy,
                    authorization,
                    token_program: anchor_spl::token_2022::ID,
                    system_program: anchor_lang::system_program::ID,
                },
            )
            .args(
                token2022_compliance::instruction::InitializeAuthorization {
                    wallet: target_wallet,
                },
            )
            .send()?;

    println!();
    println!("Authorization initialized");
    println!("Signature: {}", signature);

    // ============================================================
    // 2. Mark wallet Authorized
    // ============================================================

    let signature =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::SetAuthorizationStatus {
                    admin,
                    mint,
                    global_policy,
                    authorization,
                    token_program: anchor_spl::token_2022::ID,
                },
            )
            .args(
                token2022_compliance::instruction::SetAuthorizationStatus {
                    new_status: AuthorizationStatus::Authorized,
                },
            )
            .send()?;

    println!();
    println!("Wallet -> Authorized");
    println!("Signature: {}", signature);

    // ============================================================
    // 3. Initialize wallet-level TransferStats
    // ============================================================

    let signature =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::InitializeTransferStats {
                    payer: admin,
                    mint,
                    authorization,
                    transfer_stats,
                    token_program: anchor_spl::token_2022::ID,
                    system_program: anchor_lang::system_program::ID,
                },
            )
            .args(
                token2022_compliance::instruction::InitializeTransferStats {
                    wallet: target_wallet,
                },
            )
            .send()?;

    println!();
    println!("TransferStats initialized");
    println!("Signature: {}", signature);

    println!();
    println!("====================================");
    println!("BROWSER WALLET READY");
    println!("====================================");
    println!("Wallet:        {}", target_wallet);
    println!("Authorization: {}", authorization);
    println!("Stats:         {}", transfer_stats);

    Ok(())
}