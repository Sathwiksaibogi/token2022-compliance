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

use token2022_compliance::state::GlobalPolicy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ------------------------------------------------------------
    // RPC + admin
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
    // Mint
    // ------------------------------------------------------------

    let mint = Pubkey::from_str(
        "CqLtUZQBK9phoD33ye6sQQG2EdoMrEvraHgijwpy9ZDF",
    )?;

    // ------------------------------------------------------------
    // GlobalPolicy PDA
    // ------------------------------------------------------------

    let (global_policy, bump) =
        Pubkey::find_program_address(
            &[
                b"policy",
                mint.as_ref(),
            ],
            &token2022_compliance::ID,
        );

    println!("Admin:        {}", admin);
    println!("Mint:         {}", mint);
    println!("GlobalPolicy: {}", global_policy);
    println!("Policy bump:  {}", bump);

    // ------------------------------------------------------------
    // Read policy BEFORE update
    // ------------------------------------------------------------

    let before: GlobalPolicy =
        program.account(global_policy)?;

    println!();
    println!("Policy version before: {}", before.policy_version);

    // ------------------------------------------------------------
    // New fee configuration
    //
    // 200 basis points = 2%
    // 5_000_000 base units = 5 tokens
    // because mint decimals = 6
    // ------------------------------------------------------------

    let transfer_fee_basis_points: u16 = 200;
    let maximum_fee: u64 = 5_000_000;

    println!();
    println!("Requesting dynamic fee update:");
    println!(
        "  basis points = {}",
        transfer_fee_basis_points
    );
    println!(
        "  maximum fee  = {} base units",
        maximum_fee
    );

    // ------------------------------------------------------------
    // Call OUR program.
    //
    // Our program then performs:
    //
    // GlobalPolicy PDA
    //      ↓ invoke_signed
    // Token-2022 SetTransferFee
    // ------------------------------------------------------------

    let signature =
        program
            .request()
            .accounts(
                token2022_compliance::accounts::UpdateTransferFee {
                    admin,
                    mint,
                    global_policy,
                    token_program: anchor_spl::token_2022::ID,
                },
            )
            .args(
                token2022_compliance::instruction::UpdateTransferFee {
                    transfer_fee_basis_points,
                    maximum_fee,
                },
            )
            .send()?;

    println!();
    println!("✅ update_transfer_fee succeeded");
    println!("Signature: {}", signature);

    // ------------------------------------------------------------
    // Verify policy version changed
    // ------------------------------------------------------------

    let after: GlobalPolicy =
        program.account(global_policy)?;

    println!();
    println!("Policy version after: {}", after.policy_version);

    let expected_version =
        before
            .policy_version
            .checked_add(1)
            .expect("policy version overflow");

    assert_eq!(
        after.policy_version,
        expected_version,
        "policy version did not increment"
    );

    println!();
    println!("✅ Policy version increment verified");

    Ok(())
}