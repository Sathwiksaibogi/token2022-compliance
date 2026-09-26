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
    // RPC + admin wallet
    // ------------------------------------------------------------

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
        read_keypair_file(&wallet_path)
            .map_err(|err| {
                format!("failed to read admin wallet: {err}")
            })?;

    let admin = payer.pubkey();

    let client =
        Client::new(
            cluster,
            Rc::new(payer),
        );

    let program =
        client.program(token2022_compliance::ID)?;

    // ------------------------------------------------------------
    // Known Devnet addresses
    // ------------------------------------------------------------

    let mint = Pubkey::from_str(
        "CqLtUZQBK9phoD33ye6sQQG2EdoMrEvraHgijwpy9ZDF",
    )?;

    let alice = Pubkey::from_str(
        "28eQwy3xZsPmMNUeHmvSp2no4aAndmk4EQjXoTm8KHo2",
    )?;

    let bob = Pubkey::from_str(
        "4yGAXQV4saTujAxkHSCmw2F1HbcpF4zwfjRqaHy24e8H",
    )?;

    // ------------------------------------------------------------
    // Derive GlobalPolicy
    // ------------------------------------------------------------

    let (global_policy, _) =
        Pubkey::find_program_address(
            &[
                b"policy",
                mint.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    // ------------------------------------------------------------
    // Derive Authorization PDAs
    // ------------------------------------------------------------

    let (alice_authorization, _) =
        Pubkey::find_program_address(
            &[
                b"authorization",
                mint.as_ref(),
                alice.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    let (bob_authorization, _) =
        Pubkey::find_program_address(
            &[
                b"authorization",
                mint.as_ref(),
                bob.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    // ------------------------------------------------------------
    // Alice TransferStats PDA
    // ------------------------------------------------------------

    let (alice_stats, _) =
        Pubkey::find_program_address(
            &[
                b"stats",
                mint.as_ref(),
                alice.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    // ------------------------------------------------------------
    // ExtraAccountMetaList PDA
    // ------------------------------------------------------------

    let (extra_account_meta_list, _) =
        Pubkey::find_program_address(
            &[
                b"extra-account-metas",
                mint.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    println!("Admin: {}", admin);
    println!("Mint: {}", mint);
    println!("GlobalPolicy: {}", global_policy);

    println!(
        "Alice Authorization: {}",
        alice_authorization
    );

    println!(
        "Bob Authorization: {}",
        bob_authorization
    );

    println!(
        "Alice TransferStats: {}",
        alice_stats
    );

    println!(
        "ExtraAccountMetaList: {}",
        extra_account_meta_list
    );

    // ============================================================
    // 1. Initialize Alice Authorization
    // ============================================================

    let sig =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::InitializeAuthorization {
                    admin,
                    mint,
                    global_policy,
                    authorization: alice_authorization,
                    token_program: anchor_spl::token_2022::ID,
                    system_program: anchor_lang::system_program::ID,
                },
            )
            .args(
                token2022_compliance::instruction::InitializeAuthorization {
                    wallet: alice,
                },
            )
            .send()?;

    println!();
    println!("Alice Authorization initialized");
    println!("Signature: {}", sig);

    // ============================================================
    // 2. Initialize Bob Authorization
    // ============================================================

    let sig =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::InitializeAuthorization {
                    admin,
                    mint,
                    global_policy,
                    authorization: bob_authorization,
                    token_program: anchor_spl::token_2022::ID,
                    system_program: anchor_lang::system_program::ID,
                },
            )
            .args(
                token2022_compliance::instruction::InitializeAuthorization {
                    wallet: bob,
                },
            )
            .send()?;

    println!();
    println!("Bob Authorization initialized");
    println!("Signature: {}", sig);

    // ============================================================
    // 3. Authorize Alice
    // ============================================================

    let sig =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::SetAuthorizationStatus {
                    admin,
                    mint,
                    global_policy,
                    authorization: alice_authorization,
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
    println!("Alice status -> Authorized");
    println!("Signature: {}", sig);

    // ============================================================
    // 4. Authorize Bob
    // ============================================================

    let sig =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::SetAuthorizationStatus {
                    admin,
                    mint,
                    global_policy,
                    authorization: bob_authorization,
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
    println!("Bob status -> Authorized");
    println!("Signature: {}", sig);

    // ============================================================
    // 5. Initialize Alice TransferStats
    // ============================================================

    let sig =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::InitializeTransferStats {
                    payer: admin,
                    mint,
                    authorization: alice_authorization,
                    transfer_stats: alice_stats,
                    token_program: anchor_spl::token_2022::ID,
                    system_program: anchor_lang::system_program::ID,
                },
            )
            .args(
                token2022_compliance::instruction::InitializeTransferStats {
                    wallet: alice,
                },
            )
            .send()?;

    println!();
    println!("Alice TransferStats initialized");
    println!("Signature: {}", sig);

    // ============================================================
    // 6. Initialize ExtraAccountMetaList
    //
    // Admin wallet is currently the mint's TransferHook authority,
    // so it signs this instruction.
    // ============================================================

    let sig =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::InitializeExtraAccountMetaList {
                    payer: admin,
                    extra_account_meta_list,
                    mint,
                    mint_authority: admin,
                    token_program: anchor_spl::token_2022::ID,
                    system_program: anchor_lang::system_program::ID,
                },
            )
            .args(
                token2022_compliance::instruction::InitializeExtraAccountMetaList {},
            )
            .send()?;

    println!();
    println!("ExtraAccountMetaList initialized");
    println!("Signature: {}", sig);

    println!();
    println!("========================================");
    println!("DEVNET COMPLIANCE STATE READY");
    println!("========================================");
    println!("Alice Authorization: {}", alice_authorization);
    println!("Bob Authorization: {}", bob_authorization);
    println!("Alice Stats: {}", alice_stats);
    println!("ExtraAccountMetaList: {}", extra_account_meta_list);

    Ok(())
}