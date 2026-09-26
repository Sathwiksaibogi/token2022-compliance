use litesvm::LiteSVM;

use std::path::PathBuf;

use solana_clock::Clock;

use anchor_lang::prelude::Pubkey;

use anchor_lang::{InstructionData, ToAccountMetas};

use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};

use anchor_lang::AccountDeserialize;

use spl_tlv_account_resolution::state::ExtraAccountMetaList;

use solana_keypair::Keypair;

use solana_signer::Signer;

use solana_system_interface::instruction::create_account;

use solana_transaction::Transaction;

use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::{
    instruction as transfer_fee_instruction, TransferFeeAmount, TransferFeeConfig,
};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        transfer_hook::{
            instruction as transfer_hook_instruction, TransferHook, TransferHookAccount,
        },
        BaseStateWithExtensions, ExtensionType, StateWithExtensions,
    },
    instruction::{
        approve_checked, initialize_account3, initialize_mint2, mint_to_checked, transfer_checked,
    },
    state::{Account as Token2022Account, Mint as Token2022Mint},
};

#[test]

fn token2022_compliance_flow() {
    let mut svm = LiteSVM::new();

    let program_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/token2022_compliance.so");

    svm.add_program_from_file(token2022_compliance::ID, &program_path)
        .expect("failed to load token2022-compliance program");

    let token_2022_account = svm
        .get_account(&anchor_spl::token_2022::ID)
        .expect("Token-2022 program is not loaded in LiteSVM");

    assert!(
        token_2022_account.executable,
        "Token-2022 account exists but is not executable"
    );

    let payer = Keypair::new();

    let mint_authority = Keypair::new();

    let mint = Keypair::new();

    let mint_pubkey = mint.pubkey();

    let mint_authority_pubkey = mint_authority.pubkey();

    let transfer_fee_basis_points: u16 = 100; // 1%
    let maximum_transfer_fee: u64 = 5_000_000; // 5 tokens (6 decimals)

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("failed to fund payer");

    let mint_extensions = [
        ExtensionType::TransferHook,
        ExtensionType::TransferFeeConfig,
    ];

    let mint_size = ExtensionType::try_calculate_account_len::<Token2022Mint>(&mint_extensions)
        .expect("failed to calculate Token-2022 mint size");

    println!("Transfer-Hook + Transfer-Fee mint size: {mint_size} bytes");

    assert!(mint_size > 82);

    let mint_rent = svm.minimum_balance_for_rent_exemption(mint_size);

    println!("Mint rent-exempt lamports: {mint_rent}");

    let create_mint_account_ix = create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        mint_size as u64,
        &anchor_spl::token_2022::ID,
    );

    let initialize_transfer_hook_ix = transfer_hook_instruction::initialize(
        &anchor_spl::token_2022::ID,
        &mint_pubkey,
        Some(mint_authority_pubkey),
        Some(token2022_compliance::ID),
    )
    .expect("failed to build Transfer Hook initialization instruction");

    let initialize_transfer_fee_ix = transfer_fee_instruction::initialize_transfer_fee_config(
        &anchor_spl::token_2022::ID,
        &mint_pubkey,
        Some(&mint_authority_pubkey),
        Some(&mint_authority_pubkey),
        transfer_fee_basis_points,
        maximum_transfer_fee,
    )
    .expect("failed to build TransferFeeConfig initialization");

    let initialize_mint_ix = initialize_mint2(
        &anchor_spl::token_2022::ID,
        &mint_pubkey,
        &mint_authority_pubkey,
        None,
        6,
    )
    .expect("failed to build Token-2022 mint initialization instruction");

    let blockhash = svm.latest_blockhash();

    let transaction = Transaction::new_signed_with_payer(
        &[
            create_mint_account_ix,
            initialize_transfer_hook_ix,
            initialize_transfer_fee_ix,
            initialize_mint_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint],
        blockhash,
    );

    svm.send_transaction(transaction)
        .expect("failed to create Token-2022 mint account");

    let mint_account = svm
        .get_account(&mint.pubkey())
        .expect("mint account was not created");

    assert_eq!(mint_account.owner, anchor_spl::token_2022::ID);

    assert_eq!(mint_account.data.len(), mint_size);

    assert_eq!(mint_account.lamports, mint_rent);

    assert!(!mint_account.executable);

    println!("Mint account created successfully");

    println!("Mint owner: {}", mint_account.owner);

    println!("Mint data length: {}", mint_account.data.len());

    println!("Mint lamports: {}", mint_account.lamports);

    let parsed_mint = StateWithExtensions::<Token2022Mint>::unpack(&mint_account.data)
        .expect("failed to parse Token-2022 mint");

    assert!(parsed_mint.base.is_initialized);

    assert_eq!(parsed_mint.base.decimals, 6);

    assert_eq!(
        parsed_mint.base.mint_authority,
        Some(mint_authority_pubkey).into(),
    );

    let transfer_hook = parsed_mint
        .get_extension::<TransferHook>()
        .expect("TransferHook extension missing");

    let hook_authority: Option<_> = transfer_hook.authority.into();

    let hook_program_id: Option<_> = transfer_hook.program_id.into();

    assert_eq!(hook_authority, Some(mint_authority_pubkey),);

    assert_eq!(hook_program_id, Some(token2022_compliance::ID),);

    println!("Base mint initialized: {}", parsed_mint.base.is_initialized);

    println!("Mint decimals: {}", parsed_mint.base.decimals);

    println!("Transfer Hook authority: {:?}", hook_authority);

    println!("Transfer Hook program: {:?}", hook_program_id);

    let transfer_fee_config = parsed_mint
        .get_extension::<TransferFeeConfig>()
        .expect("TransferFeeConfig extension missing");

    let fee_config_authority: Option<Pubkey> =
        transfer_fee_config.transfer_fee_config_authority.into();
    let withdraw_withheld_authority: Option<Pubkey> =
        transfer_fee_config.withdraw_withheld_authority.into();

    assert_eq!(fee_config_authority, Some(mint_authority_pubkey));
    assert_eq!(withdraw_withheld_authority, Some(mint_authority_pubkey));
    assert_eq!(u64::from(transfer_fee_config.withheld_amount), 0);

    assert_eq!(
        u16::from(
            transfer_fee_config
                .older_transfer_fee
                .transfer_fee_basis_points
        ),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(transfer_fee_config.older_transfer_fee.maximum_fee),
        maximum_transfer_fee
    );
    assert_eq!(
        u16::from(
            transfer_fee_config
                .newer_transfer_fee
                .transfer_fee_basis_points
        ),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(transfer_fee_config.newer_transfer_fee.maximum_fee),
        maximum_transfer_fee
    );

    println!(
        "Transfer fee: {} bps, max {} base units",
        transfer_fee_basis_points, maximum_transfer_fee
    );

    let mint_extension_types = parsed_mint
        .get_extension_types()
        .expect("failed to read mint extension types");

    println!("Mint extensions: {:?}", mint_extension_types);

    let required_account_extensions =
        ExtensionType::get_required_init_account_extensions(&mint_extension_types);

    println!(
        "Required token-account extensions: {:?}",
        required_account_extensions
    );

    assert!(
        required_account_extensions.contains(&ExtensionType::TransferHookAccount),
        "TransferHookAccount must be required"
    );
    assert!(
        required_account_extensions.contains(&ExtensionType::TransferFeeAmount),
        "TransferFeeAmount must be required"
    );

    let token_account_size =
        ExtensionType::try_calculate_account_len::<Token2022Account>(&required_account_extensions)
            .expect("failed to calculate Token-2022 account size");

    println!("Transfer-Hook + Transfer-Fee token account size: {token_account_size} bytes");

    let alice = Keypair::new();

    let bob = Keypair::new();

    let alice_token = Keypair::new();

    let bob_token = Keypair::new();

    let alice_pubkey = alice.pubkey();

    let bob_pubkey = bob.pubkey();

    let alice_token_pubkey = alice_token.pubkey();

    let bob_token_pubkey = bob_token.pubkey();

    let token_account_rent = svm.minimum_balance_for_rent_exemption(token_account_size);

    println!("Token account rent: {token_account_rent}");

    let create_alice_token_ix = create_account(
        &payer.pubkey(),
        &alice_token_pubkey,
        token_account_rent,
        token_account_size as u64,
        &anchor_spl::token_2022::ID,
    );

    let initialize_alice_token_ix = initialize_account3(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &alice_pubkey,
    )
    .expect("failed to build Alice token-account initialization");

    let create_bob_token_ix = create_account(
        &payer.pubkey(),
        &bob_token_pubkey,
        token_account_rent,
        token_account_size as u64,
        &anchor_spl::token_2022::ID,
    );

    let initialize_bob_token_ix = initialize_account3(
        &anchor_spl::token_2022::ID,
        &bob_token_pubkey,
        &mint_pubkey,
        &bob_pubkey,
    )
    .expect("failed to build Bob token-account initialization");

    let blockhash = svm.latest_blockhash();

    let create_token_accounts_tx = Transaction::new_signed_with_payer(
        &[
            create_alice_token_ix,
            initialize_alice_token_ix,
            create_bob_token_ix,
            initialize_bob_token_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer, &alice_token, &bob_token],
        blockhash,
    );

    svm.send_transaction(create_token_accounts_tx)
        .expect("failed to create Alice and Bob token accounts");

    let alice_token_account = svm
        .get_account(&alice_token_pubkey)
        .expect("Alice token account missing");

    let bob_token_account = svm
        .get_account(&bob_token_pubkey)
        .expect("Bob token account missing");

    let parsed_alice = StateWithExtensions::<Token2022Account>::unpack(&alice_token_account.data)
        .expect("failed to parse Alice token account");

    let parsed_bob = StateWithExtensions::<Token2022Account>::unpack(&bob_token_account.data)
        .expect("failed to parse Bob token account");

    assert_eq!(parsed_alice.base.mint, mint_pubkey);

    assert_eq!(parsed_alice.base.owner, alice_pubkey);

    assert_eq!(parsed_alice.base.amount, 0);

    assert_eq!(parsed_bob.base.mint, mint_pubkey);

    assert_eq!(parsed_bob.base.owner, bob_pubkey);

    assert_eq!(parsed_bob.base.amount, 0);

    let alice_transfer_hook = parsed_alice
        .get_extension::<TransferHookAccount>()
        .expect("Alice TransferHookAccount extension missing");

    assert!(!bool::from(alice_transfer_hook.transferring));

    let bob_transfer_hook = parsed_bob
        .get_extension::<TransferHookAccount>()
        .expect("Bob TransferHookAccount extension missing");

    assert!(!bool::from(bob_transfer_hook.transferring));

    let alice_transfer_fee = parsed_alice
        .get_extension::<TransferFeeAmount>()
        .expect("Alice TransferFeeAmount extension missing");

    let bob_transfer_fee = parsed_bob
        .get_extension::<TransferFeeAmount>()
        .expect("Bob TransferFeeAmount extension missing");

    assert_eq!(u64::from(alice_transfer_fee.withheld_amount), 0);
    assert_eq!(u64::from(bob_transfer_fee.withheld_amount), 0);

    println!("Alice token owner: {}", parsed_alice.base.owner);

    println!("Alice token balance: {}", parsed_alice.base.amount);

    println!(
        "Alice transferring: {}",
        bool::from(alice_transfer_hook.transferring)
    );

    println!("Bob token owner: {}", parsed_bob.base.owner);

    println!("Bob token balance: {}", parsed_bob.base.amount);

    println!(
        "Bob transferring: {}",
        bool::from(bob_transfer_hook.transferring)
    );

    let amount_to_mint: u64 = 1_000_000_000;

    let mint_to_alice_ix = mint_to_checked(
        &anchor_spl::token_2022::ID,
        &mint_pubkey,
        &alice_token_pubkey,
        &mint_authority_pubkey,
        &[],
        amount_to_mint,
        6,
    )
    .expect("failed to build mint-to instruction");

    let blockhash = svm.latest_blockhash();

    let mint_tx = Transaction::new_signed_with_payer(
        &[mint_to_alice_ix],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        blockhash,
    );

    svm.send_transaction(mint_tx)
        .expect("failed to mint tokens to Alice");

    let alice_after_mint_account = svm
        .get_account(&alice_token_pubkey)
        .expect("Alice token account missing after mint");

    let alice_after_mint =
        StateWithExtensions::<Token2022Account>::unpack(&alice_after_mint_account.data)
            .expect("failed to parse Alice token account after mint");

    assert_eq!(alice_after_mint.base.amount, amount_to_mint);

    println!(
        "Alice balance after mint: {} base units",
        alice_after_mint.base.amount
    );

    let bob_after_mint_account = svm
        .get_account(&bob_token_pubkey)
        .expect("Bob token account missing after mint");

    let bob_after_mint =
        StateWithExtensions::<Token2022Account>::unpack(&bob_after_mint_account.data)
            .expect("failed to parse Bob token account after mint");

    assert_eq!(bob_after_mint.base.amount, 0);

    let mint_after_minting_account = svm
        .get_account(&mint_pubkey)
        .expect("mint missing after minting");

    let mint_after_minting =
        StateWithExtensions::<Token2022Mint>::unpack(&mint_after_minting_account.data)
            .expect("failed to parse mint after minting");

    assert_eq!(mint_after_minting.base.supply, amount_to_mint);

    println!(
        "Total mint supply: {} base units",
        mint_after_minting.base.supply
    );

    let admin = Keypair::new();

    let admin_pubkey = admin.pubkey();

    svm.airdrop(&admin_pubkey, 2_000_000_000)
        .expect("failed to fund policy admin");

    let (global_policy_pda, global_policy_bump) = Pubkey::find_program_address(
        &[b"policy", mint_pubkey.as_ref()],
        &token2022_compliance::ID,
    );

    println!("GlobalPolicy PDA: {}", global_policy_pda);

    println!("GlobalPolicy bump: {}", global_policy_bump);

    let max_transfer_amount: u64 = 100 * 1_000_000;

    let daily_transfer_limit: u64 = 250 * 1_000_000;

    let initialize_policy_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializePolicy {
            admin: admin_pubkey,

            mint: mint_pubkey,

            global_policy: global_policy_pda,

            token_program: anchor_spl::token_2022::ID,

            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializePolicy {
            max_transfer_amount,

            daily_transfer_limit,

            expires_at: 0,
        }
        .data(),
    };

    let blockhash = svm.latest_blockhash();

    let initialize_policy_tx = Transaction::new_signed_with_payer(
        &[initialize_policy_ix],
        Some(&admin_pubkey),
        &[&admin],
        blockhash,
    );

    svm.send_transaction(initialize_policy_tx)
        .expect("failed to initialize GlobalPolicy");

    let global_policy_account = svm
        .get_account(&global_policy_pda)
        .expect("GlobalPolicy account was not created");

    assert_eq!(global_policy_account.owner, token2022_compliance::ID);

    let mut policy_data = global_policy_account.data.as_slice();

    let policy = token2022_compliance::state::GlobalPolicy::try_deserialize(&mut policy_data)
        .expect("failed to deserialize GlobalPolicy");

    assert_eq!(policy.admin, admin_pubkey);

    assert_eq!(policy.mint, mint_pubkey);

    assert!(policy.enabled);

    assert_eq!(policy.max_transfer_amount, max_transfer_amount);

    assert_eq!(policy.daily_transfer_limit, daily_transfer_limit);

    assert_eq!(policy.policy_version, 1);

    assert_eq!(policy.expires_at, 0);

    assert_eq!(policy.bump, global_policy_bump);

    println!("GlobalPolicy initialized");

    println!("Policy admin: {}", policy.admin);

    println!("Max transfer: {} base units", policy.max_transfer_amount);

    println!("Daily limit: {} base units", policy.daily_transfer_limit);

    println!("Policy version: {}", policy.policy_version);

    println!("Policy enabled: {}", policy.enabled);

    let (alice_authorization_pda, alice_authorization_bump) = Pubkey::find_program_address(
        &[
            b"authorization",
            mint_pubkey.as_ref(),
            alice_pubkey.as_ref(),
        ],
        &token2022_compliance::ID,
    );

    let (bob_authorization_pda, bob_authorization_bump) = Pubkey::find_program_address(
        &[b"authorization", mint_pubkey.as_ref(), bob_pubkey.as_ref()],
        &token2022_compliance::ID,
    );

    println!("Alice Authorization PDA: {}", alice_authorization_pda);

    println!("Bob Authorization PDA: {}", bob_authorization_pda);

    let initialize_alice_authorization_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeAuthorization {
            admin: admin_pubkey,

            mint: mint_pubkey,

            global_policy: global_policy_pda,

            authorization: alice_authorization_pda,

            token_program: anchor_spl::token_2022::ID,

            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeAuthorization {
            wallet: alice_pubkey,
        }
        .data(),
    };

    let initialize_bob_authorization_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeAuthorization {
            admin: admin_pubkey,

            mint: mint_pubkey,

            global_policy: global_policy_pda,

            authorization: bob_authorization_pda,

            token_program: anchor_spl::token_2022::ID,

            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeAuthorization { wallet: bob_pubkey }
            .data(),
    };

    let blockhash = svm.latest_blockhash();

    let initialize_authorizations_tx = Transaction::new_signed_with_payer(
        &[
            initialize_alice_authorization_ix,
            initialize_bob_authorization_ix,
        ],
        Some(&admin_pubkey),
        &[&admin],
        blockhash,
    );

    svm.send_transaction(initialize_authorizations_tx)
        .expect("failed to initialize authorization accounts");

    let alice_authorization_account = svm
        .get_account(&alice_authorization_pda)
        .expect("Alice Authorization account missing");

    assert_eq!(alice_authorization_account.owner, token2022_compliance::ID);

    let mut alice_auth_data = alice_authorization_account.data.as_slice();

    let alice_authorization =
        token2022_compliance::state::Authorization::try_deserialize(&mut alice_auth_data)
            .expect("failed to deserialize Alice Authorization");

    assert_eq!(alice_authorization.mint, mint_pubkey);

    assert_eq!(alice_authorization.wallet, alice_pubkey);

    assert_eq!(
        alice_authorization.status,
        token2022_compliance::state::AuthorizationStatus::Unauthorized
    );

    assert_eq!(alice_authorization.bump, alice_authorization_bump);

    let bob_authorization_account = svm
        .get_account(&bob_authorization_pda)
        .expect("Bob Authorization account missing");

    let mut bob_auth_data = bob_authorization_account.data.as_slice();

    let bob_authorization =
        token2022_compliance::state::Authorization::try_deserialize(&mut bob_auth_data)
            .expect("failed to deserialize Bob Authorization");

    assert_eq!(bob_authorization.mint, mint_pubkey);

    assert_eq!(bob_authorization.wallet, bob_pubkey);

    assert_eq!(
        bob_authorization.status,
        token2022_compliance::state::AuthorizationStatus::Unauthorized
    );

    assert_eq!(bob_authorization.bump, bob_authorization_bump);

    println!(
        "Alice authorization status: {:?}",
        alice_authorization.status
    );

    println!("Bob authorization status: {:?}", bob_authorization.status);

    let authorize_alice_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::SetAuthorizationStatus {
            admin: admin_pubkey,

            mint: mint_pubkey,

            global_policy: global_policy_pda,

            authorization: alice_authorization_pda,

            token_program: anchor_spl::token_2022::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::SetAuthorizationStatus {
            new_status: token2022_compliance::state::AuthorizationStatus::Authorized,
        }
        .data(),
    };

    let authorize_bob_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::SetAuthorizationStatus {
            admin: admin_pubkey,

            mint: mint_pubkey,

            global_policy: global_policy_pda,

            authorization: bob_authorization_pda,

            token_program: anchor_spl::token_2022::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::SetAuthorizationStatus {
            new_status: token2022_compliance::state::AuthorizationStatus::Authorized,
        }
        .data(),
    };

    let blockhash = svm.latest_blockhash();

    let authorize_users_tx = Transaction::new_signed_with_payer(
        &[authorize_alice_ix, authorize_bob_ix],
        Some(&admin_pubkey),
        &[&admin],
        blockhash,
    );

    svm.send_transaction(authorize_users_tx)
        .expect("failed to authorize Alice and Bob");

    let alice_authorization_account = svm
        .get_account(&alice_authorization_pda)
        .expect("Alice Authorization missing after update");

    let mut alice_auth_data = alice_authorization_account.data.as_slice();

    let alice_authorization_after =
        token2022_compliance::state::Authorization::try_deserialize(&mut alice_auth_data)
            .expect("failed to deserialize updated Alice Authorization");

    assert_eq!(
        alice_authorization_after.status,
        token2022_compliance::state::AuthorizationStatus::Authorized
    );

    let bob_authorization_account = svm
        .get_account(&bob_authorization_pda)
        .expect("Bob Authorization missing after update");

    let mut bob_auth_data = bob_authorization_account.data.as_slice();

    let bob_authorization_after =
        token2022_compliance::state::Authorization::try_deserialize(&mut bob_auth_data)
            .expect("failed to deserialize updated Bob Authorization");

    assert_eq!(
        bob_authorization_after.status,
        token2022_compliance::state::AuthorizationStatus::Authorized
    );

    println!(
        "Alice authorization after approval: {:?}",
        alice_authorization_after.status
    );

    println!(
        "Bob authorization after approval: {:?}",
        bob_authorization_after.status
    );

    let (alice_stats_pda, alice_stats_bump) = Pubkey::find_program_address(
        &[b"stats", mint_pubkey.as_ref(), alice_pubkey.as_ref()],
        &token2022_compliance::ID,
    );

    println!("Alice TransferStats PDA: {}", alice_stats_pda);

    let initialize_alice_stats_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeTransferStats {
            payer: payer.pubkey(),

            mint: mint_pubkey,

            authorization: alice_authorization_pda,

            transfer_stats: alice_stats_pda,

            token_program: anchor_spl::token_2022::ID,

            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeTransferStats {
            wallet: alice_pubkey,
        }
        .data(),
    };

    let blockhash = svm.latest_blockhash();

    let initialize_alice_stats_tx = Transaction::new_signed_with_payer(
        &[initialize_alice_stats_ix],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    svm.send_transaction(initialize_alice_stats_tx)
        .expect("failed to initialize Alice TransferStats");

    let alice_stats_account = svm
        .get_account(&alice_stats_pda)
        .expect("Alice TransferStats account missing");

    assert_eq!(alice_stats_account.owner, token2022_compliance::ID);

    let mut alice_stats_data = alice_stats_account.data.as_slice();

    let alice_stats =
        token2022_compliance::state::TransferStats::try_deserialize(&mut alice_stats_data)
            .expect("failed to deserialize Alice TransferStats");

    assert_eq!(alice_stats.mint, mint_pubkey);

    assert_eq!(alice_stats.wallet, alice_pubkey);

    assert_eq!(alice_stats.amount_today, 0);

    assert_eq!(alice_stats.total_transferred, 0);

    assert_eq!(alice_stats.transfer_count, 0);

    assert_eq!(alice_stats.bump, alice_stats_bump);

    println!("Alice TransferStats initialized");

    println!("Stats wallet: {}", alice_stats.wallet);

    println!("Day index: {}", alice_stats.day_index);

    println!("Amount today: {}", alice_stats.amount_today);

    println!("Total transferred: {}", alice_stats.total_transferred);

    println!("Transfer count: {}", alice_stats.transfer_count);

    let (extra_account_meta_list_pda, extra_account_meta_list_bump) = Pubkey::find_program_address(
        &[b"extra-account-metas", mint_pubkey.as_ref()],
        &token2022_compliance::ID,
    );

    println!("ExtraAccountMetaList PDA: {}", extra_account_meta_list_pda);

    println!(
        "ExtraAccountMetaList bump: {}",
        extra_account_meta_list_bump
    );

    let initialize_extra_account_meta_list_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeExtraAccountMetaList {
            payer: payer.pubkey(),

            extra_account_meta_list: extra_account_meta_list_pda,

            mint: mint_pubkey,

            mint_authority: mint_authority_pubkey,

            token_program: anchor_spl::token_2022::ID,

            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeExtraAccountMetaList {}.data(),
    };

    let blockhash = svm.latest_blockhash();

    let initialize_extra_account_meta_list_tx = Transaction::new_signed_with_payer(
        &[initialize_extra_account_meta_list_ix],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        blockhash,
    );

    svm.send_transaction(initialize_extra_account_meta_list_tx)
        .expect("failed to initialize ExtraAccountMetaList");

    let extra_account_meta_list_account = svm
        .get_account(&extra_account_meta_list_pda)
        .expect("ExtraAccountMetaList account missing");

    assert_eq!(
        extra_account_meta_list_account.owner,
        token2022_compliance::ID
    );

    let expected_extra_account_meta_list_size =
        ExtraAccountMetaList::size_of(4).expect("failed to calculate ExtraAccountMetaList size");

    assert_eq!(
        extra_account_meta_list_account.data.len(),
        expected_extra_account_meta_list_size
    );

    println!("ExtraAccountMetaList initialized");

    println!(
        "ExtraAccountMetaList owner: {}",
        extra_account_meta_list_account.owner
    );

    println!(
        "ExtraAccountMetaList size: {} bytes",
        extra_account_meta_list_account.data.len()
    );

    let transfer_amount: u64 = 40 * 1_000_000;

    let mut transfer_ix = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        &alice_pubkey,
        &[],
        transfer_amount,
        6,
    )
    .expect("failed to build transfer_checked instruction");

    transfer_ix.accounts.extend([
        AccountMeta::new_readonly(extra_account_meta_list_pda, false),
        AccountMeta::new_readonly(global_policy_pda, false),
        AccountMeta::new_readonly(alice_authorization_pda, false),
        AccountMeta::new_readonly(bob_authorization_pda, false),
        AccountMeta::new(alice_stats_pda, false),
        // Token-2022 needs the executable hook program available

        // when it CPIs into our compliance program.
        AccountMeta::new_readonly(token2022_compliance::ID, false),
    ]);

    let blockhash = svm.latest_blockhash();

    let transfer_tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        blockhash,
    );

    svm.send_transaction(transfer_tx)
        .expect("Alice -> Bob compliant transfer failed");

    let alice_after_transfer_account = svm
        .get_account(&alice_token_pubkey)
        .expect("Alice token account missing");

    let alice_after_transfer =
        StateWithExtensions::<Token2022Account>::unpack(&alice_after_transfer_account.data)
            .expect("failed to parse Alice after transfer");

    let bob_after_transfer_account = svm
        .get_account(&bob_token_pubkey)
        .expect("Bob token account missing");

    let bob_after_transfer =
        StateWithExtensions::<Token2022Account>::unpack(&bob_after_transfer_account.data)
            .expect("failed to parse Bob after transfer");

    assert_eq!(alice_after_transfer.base.amount, 960_000_000,);

    assert_eq!(bob_after_transfer.base.amount, 39_600_000,);

    println!(
        "Alice balance after transfer: {}",
        alice_after_transfer.base.amount
    );

    println!(
        "Bob balance after transfer: {}",
        bob_after_transfer.base.amount
    );

    let bob_fee_after_transfer = bob_after_transfer
        .get_extension::<TransferFeeAmount>()
        .expect("Bob TransferFeeAmount missing after transfer");

    assert_eq!(u64::from(bob_fee_after_transfer.withheld_amount), 400_000);

    println!(
        "Bob withheld transfer fee: {}",
        u64::from(bob_fee_after_transfer.withheld_amount)
    );

    let alice_stats_after_account = svm
        .get_account(&alice_stats_pda)
        .expect("Alice TransferStats missing after transfer");

    let mut alice_stats_after_data = alice_stats_after_account.data.as_slice();

    let alice_stats_after =
        token2022_compliance::state::TransferStats::try_deserialize(&mut alice_stats_after_data)
            .expect("failed to deserialize Alice stats after transfer");

    assert_eq!(alice_stats_after.amount_today, 40_000_000);

    assert_eq!(alice_stats_after.total_transferred, 40_000_000);

    assert_eq!(alice_stats_after.transfer_count, 1);

    println!(
        "Amount today after transfer: {}",
        alice_stats_after.amount_today
    );

    println!(
        "Total transferred after transfer: {}",
        alice_stats_after.total_transferred
    );

    println!(
        "Transfer count after transfer: {}",
        alice_stats_after.transfer_count
    );

    let alice_hook_after = alice_after_transfer
        .get_extension::<TransferHookAccount>()
        .expect("Alice TransferHookAccount missing");

    let bob_hook_after = bob_after_transfer
        .get_extension::<TransferHookAccount>()
        .expect("Bob TransferHookAccount missing");

    assert!(!bool::from(alice_hook_after.transferring));

    assert!(!bool::from(bob_hook_after.transferring));

    println!(
        "Alice transferring after transaction: {}",
        bool::from(alice_hook_after.transferring)
    );

    println!(
        "Bob transferring after transaction: {}",
        bool::from(bob_hook_after.transferring)
    );

    // ...................test 2 max transfer amount

    let rejected_transfer_amount: u64 = 101 * 1_000_000;

    let mut rejected_transfer_ix = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        &alice_pubkey,
        &[],
        rejected_transfer_amount,
        6,
    )
    .expect("failed to build rejected transfer instruction");

    rejected_transfer_ix.accounts.extend([
        AccountMeta::new_readonly(extra_account_meta_list_pda, false),
        AccountMeta::new_readonly(global_policy_pda, false),
        AccountMeta::new_readonly(alice_authorization_pda, false),
        AccountMeta::new_readonly(bob_authorization_pda, false),
        AccountMeta::new(alice_stats_pda, false),
        AccountMeta::new_readonly(token2022_compliance::ID, false),
    ]);

    let blockhash = svm.latest_blockhash();

    let rejected_transfer_tx = Transaction::new_signed_with_payer(
        &[rejected_transfer_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        blockhash,
    );

    let rejected_result = svm.send_transaction(rejected_transfer_tx);

    assert!(
        rejected_result.is_err(),
        "101-token transfer should have been rejected"
    );

    println!(
        "101-token transfer rejected as expected: {:?}",
        rejected_result.err().unwrap()
    );

    let alice_after_rejection_account = svm
        .get_account(&alice_token_pubkey)
        .expect("Alice account missing after rejected transfer");

    let alice_after_rejection =
        StateWithExtensions::<Token2022Account>::unpack(&alice_after_rejection_account.data)
            .expect("failed to parse Alice after rejected transfer");

    assert_eq!(alice_after_rejection.base.amount, 960_000_000);

    let bob_after_rejection_account = svm
        .get_account(&bob_token_pubkey)
        .expect("Bob account missing after rejected transfer");

    let bob_after_rejection =
        StateWithExtensions::<Token2022Account>::unpack(&bob_after_rejection_account.data)
            .expect("failed to parse Bob after rejected transfer");

    assert_eq!(bob_after_rejection.base.amount, 39_600_000);

    let stats_after_rejection_account = svm
        .get_account(&alice_stats_pda)
        .expect("Alice stats missing after rejected transfer");

    let mut stats_after_rejection_data = stats_after_rejection_account.data.as_slice();

    let stats_after_rejection = token2022_compliance::state::TransferStats::try_deserialize(
        &mut stats_after_rejection_data,
    )
    .expect("failed to deserialize stats after rejected transfer");

    assert_eq!(stats_after_rejection.amount_today, 40_000_000);

    assert_eq!(stats_after_rejection.total_transferred, 40_000_000);

    assert_eq!(stats_after_rejection.transfer_count, 1);

    let alice_hook_after_rejection = alice_after_rejection
        .get_extension::<TransferHookAccount>()
        .expect("Alice TransferHookAccount missing");

    let bob_hook_after_rejection = bob_after_rejection
        .get_extension::<TransferHookAccount>()
        .expect("Bob TransferHookAccount missing");

    assert!(!bool::from(alice_hook_after_rejection.transferring));

    assert!(!bool::from(bob_hook_after_rejection.transferring));

    println!("Rejected transfer rollback verified");

    println!(
        "Alice balance unchanged: {}",
        alice_after_rejection.base.amount
    );

    println!("Bob balance unchanged: {}", bob_after_rejection.base.amount);

    println!(
        "Amount today unchanged: {}",
        stats_after_rejection.amount_today
    );

    println!(
        "Transfer count unchanged: {}",
        stats_after_rejection.transfer_count
    );

    // .....................test 3  per-transfer limits and cumulative daily limits.

    let build_hooked_transfer = |amount: u64| {
        let mut ix = transfer_checked(
            &anchor_spl::token_2022::ID,
            &alice_token_pubkey,
            &mint_pubkey,
            &bob_token_pubkey,
            &alice_pubkey,
            &[],
            amount,
            6,
        )
        .expect("failed to build transfer_checked");

        ix.accounts.extend([
            AccountMeta::new_readonly(extra_account_meta_list_pda, false),
            AccountMeta::new_readonly(global_policy_pda, false),
            AccountMeta::new_readonly(alice_authorization_pda, false),
            AccountMeta::new_readonly(bob_authorization_pda, false),
            AccountMeta::new(alice_stats_pda, false),
            AccountMeta::new_readonly(token2022_compliance::ID, false),
        ]);

        ix
    };

    let transfer_90_ix = build_hooked_transfer(90 * 1_000_000);

    let blockhash = svm.latest_blockhash();

    let transfer_90_tx = Transaction::new_signed_with_payer(
        &[transfer_90_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        blockhash,
    );

    svm.send_transaction(transfer_90_tx)
        .expect("90-token transfer should succeed");

    let transfer_100_ix = build_hooked_transfer(100 * 1_000_000);

    let blockhash = svm.latest_blockhash();

    let transfer_100_tx = Transaction::new_signed_with_payer(
        &[transfer_100_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        blockhash,
    );

    svm.send_transaction(transfer_100_tx)
        .expect("100-token transfer should succeed");

    let stats_before_daily_rejection_account = svm
        .get_account(&alice_stats_pda)
        .expect("Alice stats missing");

    let mut stats_before_daily_rejection_data =
        stats_before_daily_rejection_account.data.as_slice();

    let stats_before_daily_rejection = token2022_compliance::state::TransferStats::try_deserialize(
        &mut stats_before_daily_rejection_data,
    )
    .expect("failed to deserialize stats");

    assert_eq!(stats_before_daily_rejection.amount_today, 230_000_000);

    assert_eq!(stats_before_daily_rejection.total_transferred, 230_000_000);

    assert_eq!(stats_before_daily_rejection.transfer_count, 3);

    println!(
        "Amount today before daily-limit rejection: {}",
        stats_before_daily_rejection.amount_today
    );

    let daily_limit_rejection_ix = build_hooked_transfer(30 * 1_000_000);

    let blockhash = svm.latest_blockhash();

    let daily_limit_rejection_tx = Transaction::new_signed_with_payer(
        &[daily_limit_rejection_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        blockhash,
    );

    let daily_limit_result = svm.send_transaction(daily_limit_rejection_tx);

    assert!(
        daily_limit_result.is_err(),
        "30-token transfer should exceed the daily limit"
    );

    println!(
        "Daily-limit transfer rejected as expected: {:?}",
        daily_limit_result.err().unwrap()
    );

    let alice_after_daily_rejection_account = svm
        .get_account(&alice_token_pubkey)
        .expect("Alice account missing");

    let alice_after_daily_rejection =
        StateWithExtensions::<Token2022Account>::unpack(&alice_after_daily_rejection_account.data)
            .expect("failed to parse Alice");

    let bob_after_daily_rejection_account = svm
        .get_account(&bob_token_pubkey)
        .expect("Bob account missing");

    let bob_after_daily_rejection =
        StateWithExtensions::<Token2022Account>::unpack(&bob_after_daily_rejection_account.data)
            .expect("failed to parse Bob");

    assert_eq!(alice_after_daily_rejection.base.amount, 770_000_000);

    assert_eq!(bob_after_daily_rejection.base.amount, 227_700_000);

    let stats_after_daily_rejection_account = svm
        .get_account(&alice_stats_pda)
        .expect("Alice stats missing");

    let mut stats_after_daily_rejection_data = stats_after_daily_rejection_account.data.as_slice();

    let stats_after_daily_rejection = token2022_compliance::state::TransferStats::try_deserialize(
        &mut stats_after_daily_rejection_data,
    )
    .expect("failed to deserialize Alice stats");

    assert_eq!(stats_after_daily_rejection.amount_today, 230_000_000);

    assert_eq!(stats_after_daily_rejection.total_transferred, 230_000_000);

    assert_eq!(stats_after_daily_rejection.transfer_count, 3);

    println!("Daily-limit rollback verified");

    println!("Alice balance: {}", alice_after_daily_rejection.base.amount);

    println!("Bob balance: {}", bob_after_daily_rejection.base.amount);

    println!("Amount today: {}", stats_after_daily_rejection.amount_today);

    println!(
        "Transfer count: {}",
        stats_after_daily_rejection.transfer_count
    );

    // ..................................test 4   blocked-wallet policy

    let block_bob_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::SetAuthorizationStatus {
            admin: admin_pubkey,

            mint: mint_pubkey,

            global_policy: global_policy_pda,

            authorization: bob_authorization_pda,

            token_program: anchor_spl::token_2022::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::SetAuthorizationStatus {
            new_status: token2022_compliance::state::AuthorizationStatus::Blocked,
        }
        .data(),
    };

    let blockhash = svm.latest_blockhash();

    let block_bob_tx = Transaction::new_signed_with_payer(
        &[block_bob_ix],
        Some(&admin_pubkey),
        &[&admin],
        blockhash,
    );

    svm.send_transaction(block_bob_tx)
        .expect("failed to block Bob");

    let bob_auth_account = svm
        .get_account(&bob_authorization_pda)
        .expect("Bob Authorization missing");

    let mut bob_auth_data = bob_auth_account.data.as_slice();

    let bob_auth_blocked =
        token2022_compliance::state::Authorization::try_deserialize(&mut bob_auth_data)
            .expect("failed to deserialize Bob Authorization");

    assert_eq!(
        bob_auth_blocked.status,
        token2022_compliance::state::AuthorizationStatus::Blocked
    );

    println!("Bob authorization status: {:?}", bob_auth_blocked.status);

    let blocked_receiver_ix = build_hooked_transfer(1 * 1_000_000);

    let blockhash = svm.latest_blockhash();

    let blocked_receiver_tx = Transaction::new_signed_with_payer(
        &[blocked_receiver_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        blockhash,
    );

    let blocked_receiver_result = svm.send_transaction(blocked_receiver_tx);

    assert!(
        blocked_receiver_result.is_err(),
        "transfer to blocked Bob should fail"
    );

    println!(
        "Blocked receiver transfer rejected as expected: {:?}",
        blocked_receiver_result.err().unwrap()
    );

    // Re-fetch Alice after the rejected transfer

    let alice_after_blocked_attempt_account = svm
        .get_account(&alice_token_pubkey)
        .expect("Alice token account missing after blocked transfer");

    let alice_after_blocked_attempt =
        StateWithExtensions::<Token2022Account>::unpack(&alice_after_blocked_attempt_account.data)
            .expect("failed to parse Alice after blocked transfer");

    // Re-fetch Bob after the rejected transfer

    let bob_after_blocked_attempt_account = svm
        .get_account(&bob_token_pubkey)
        .expect("Bob token account missing after blocked transfer");

    let bob_after_blocked_attempt =
        StateWithExtensions::<Token2022Account>::unpack(&bob_after_blocked_attempt_account.data)
            .expect("failed to parse Bob after blocked transfer");

    // Balances must remain unchanged

    assert_eq!(alice_after_blocked_attempt.base.amount, 770_000_000);

    assert_eq!(bob_after_blocked_attempt.base.amount, 227_700_000);

    // Re-fetch Alice's TransferStats

    let stats_after_blocked_attempt_account = svm
        .get_account(&alice_stats_pda)
        .expect("Alice TransferStats missing after blocked transfer");

    let mut stats_after_blocked_attempt_data = stats_after_blocked_attempt_account.data.as_slice();

    let stats_after_blocked_attempt = token2022_compliance::state::TransferStats::try_deserialize(
        &mut stats_after_blocked_attempt_data,
    )
    .expect("failed to deserialize stats after blocked transfer");

    // Stats must also remain unchanged

    assert_eq!(stats_after_blocked_attempt.amount_today, 230_000_000);

    assert_eq!(stats_after_blocked_attempt.total_transferred, 230_000_000);

    assert_eq!(stats_after_blocked_attempt.transfer_count, 3);

    // Optional: verify TransferHookAccount flags returned to false

    let alice_hook_after_blocked = alice_after_blocked_attempt
        .get_extension::<TransferHookAccount>()
        .expect("Alice TransferHookAccount missing");

    let bob_hook_after_blocked = bob_after_blocked_attempt
        .get_extension::<TransferHookAccount>()
        .expect("Bob TransferHookAccount missing");

    assert!(!bool::from(alice_hook_after_blocked.transferring));

    assert!(!bool::from(bob_hook_after_blocked.transferring));

    println!("Blocked-receiver rollback verified");

    println!(
        "Alice balance unchanged: {}",
        alice_after_blocked_attempt.base.amount
    );

    println!(
        "Bob balance unchanged: {}",
        bob_after_blocked_attempt.base.amount
    );

    println!(
        "Amount today unchanged: {}",
        stats_after_blocked_attempt.amount_today
    );

    println!(
        "Transfer count unchanged: {}",
        stats_after_blocked_attempt.transfer_count
    );

    // ------------------------------------------------------------------------
    // From this point onward many negative tests intentionally submit the same
    // 1-token transfer shape. LiteSVM can otherwise reuse the same recent
    // blockhash and produce the same transaction signature, yielding
    // AlreadyProcessed before Token-2022 or the hook executes.
    let fresh_blockhash = |svm: &mut LiteSVM| {
        svm.expire_blockhash();
        svm.latest_blockhash()
    };

    let set_auth_status =
        |svm: &mut LiteSVM,

         authorization_pda: Pubkey,

         new_status: token2022_compliance::state::AuthorizationStatus| {
            let ix = Instruction {
                program_id: token2022_compliance::ID,

                accounts: token2022_compliance::accounts::SetAuthorizationStatus {
                    admin: admin_pubkey,

                    mint: mint_pubkey,

                    global_policy: global_policy_pda,

                    authorization: authorization_pda,

                    token_program: anchor_spl::token_2022::ID,
                }
                .to_account_metas(None),

                data: token2022_compliance::instruction::SetAuthorizationStatus { new_status }
                    .data(),
            };

            let blockhash = fresh_blockhash(svm);

            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&admin_pubkey),
                &[&admin],
                blockhash,
            );

            svm.send_transaction(tx)
                .expect("failed to update authorization status");
        };

    let assert_flow_state = |svm: &LiteSVM,

                             expected_alice: u64,

                             expected_bob: u64,

                             expected_today: u64,

                             expected_total: u64,

                             expected_count: u64| {
        let alice_account = svm
            .get_account(&alice_token_pubkey)
            .expect("Alice token account missing");

        let alice_state = StateWithExtensions::<Token2022Account>::unpack(&alice_account.data)
            .expect("failed to parse Alice token account");

        let bob_account = svm
            .get_account(&bob_token_pubkey)
            .expect("Bob token account missing");

        let bob_state = StateWithExtensions::<Token2022Account>::unpack(&bob_account.data)
            .expect("failed to parse Bob token account");

        let stats_account = svm
            .get_account(&alice_stats_pda)
            .expect("Alice TransferStats missing");

        let mut stats_data = stats_account.data.as_slice();

        let stats = token2022_compliance::state::TransferStats::try_deserialize(&mut stats_data)
            .expect("failed to deserialize TransferStats");

        assert_eq!(alice_state.base.amount, expected_alice);

        assert_eq!(bob_state.base.amount, expected_bob);

        assert_eq!(stats.amount_today, expected_today);

        assert_eq!(stats.total_transferred, expected_total);

        assert_eq!(stats.transfer_count, expected_count);

        let alice_fee = alice_state
            .get_extension::<TransferFeeAmount>()
            .expect("Alice TransferFeeAmount missing");

        let bob_fee = bob_state
            .get_extension::<TransferFeeAmount>()
            .expect("Bob TransferFeeAmount missing");

        assert_eq!(u64::from(alice_fee.withheld_amount), 0);

        // All successful transfers in this flow originate from Alice's original
        // 1_000-token allocation and terminate at Bob. Source balance falls by
        // the gross transfer amount, while Bob receives the post-fee amount.
        let expected_bob_withheld = 1_000_000_000u64
            .checked_sub(expected_alice)
            .and_then(|value| value.checked_sub(expected_bob))
            .expect("invalid expected balance accounting");

        assert_eq!(u64::from(bob_fee.withheld_amount), expected_bob_withheld);

        let alice_hook = alice_state
            .get_extension::<TransferHookAccount>()
            .expect("Alice TransferHookAccount missing");

        let bob_hook = bob_state
            .get_extension::<TransferHookAccount>()
            .expect("Bob TransferHookAccount missing");

        assert!(!bool::from(alice_hook.transferring));

        assert!(!bool::from(bob_hook.transferring));
    };

    // ---------------------------Test 5 — Unauthorized receiver

    set_auth_status(
        &mut svm,
        bob_authorization_pda,
        token2022_compliance::state::AuthorizationStatus::Unauthorized,
    );

    let ix = build_hooked_transfer(1_000_000);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(tx)
        .expect_err("Unauthorized receiver transfer should fail");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("ReceiverNotAuthorized"),
        "expected ReceiverNotAuthorized, got: {err_text}"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("Unauthorized receiver test passed");

    set_auth_status(
        &mut svm,
        bob_authorization_pda,
        token2022_compliance::state::AuthorizationStatus::Authorized,
    );

    // --------------------------------------Test 6 — Unauthorized sender

    set_auth_status(
        &mut svm,
        alice_authorization_pda,
        token2022_compliance::state::AuthorizationStatus::Unauthorized,
    );

    let ix = build_hooked_transfer(1_000_000);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(tx)
        .expect_err("Unauthorized sender transfer should fail");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("SenderNotAuthorized"),
        "expected SenderNotAuthorized, got: {err_text}"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("Unauthorized sender test passed");

    // ----------------------------------------------Test 7 — Blocked sender

    set_auth_status(
        &mut svm,
        alice_authorization_pda,
        token2022_compliance::state::AuthorizationStatus::Blocked,
    );

    let ix = build_hooked_transfer(1_000_000);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(tx)
        .expect_err("Blocked sender transfer should fail");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("SenderBlocked"),
        "expected SenderBlocked, got: {err_text}"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("Blocked sender test passed");

    set_auth_status(
        &mut svm,
        alice_authorization_pda,
        token2022_compliance::state::AuthorizationStatus::Authorized,
    );

    // -------------------------Test 8 — Fake admin cannot modify authorization

    let attacker = Keypair::new();

    let attacker_pubkey = attacker.pubkey();

    let fake_admin_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::SetAuthorizationStatus {
            admin: attacker_pubkey,

            mint: mint_pubkey,

            global_policy: global_policy_pda,

            authorization: bob_authorization_pda,

            token_program: anchor_spl::token_2022::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::SetAuthorizationStatus {
            new_status: token2022_compliance::state::AuthorizationStatus::Blocked,
        }
        .data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[fake_admin_ix],
        Some(&payer.pubkey()),
        &[&payer, &attacker],
        fresh_blockhash(&mut svm),
    );

    let result = svm.send_transaction(tx);

    assert!(
        result.is_err(),
        "non-admin should not be able to modify authorization"
    );

    let bob_auth_account = svm
        .get_account(&bob_authorization_pda)
        .expect("Bob Authorization missing");

    let mut bob_auth_data = bob_auth_account.data.as_slice();

    let bob_auth = token2022_compliance::state::Authorization::try_deserialize(&mut bob_auth_data)
        .expect("failed to deserialize Bob Authorization");

    assert_eq!(
        bob_auth.status,
        token2022_compliance::state::AuthorizationStatus::Authorized
    );

    println!("Unauthorized-admin attack rejected");

    // --------------------------Test 9 — Direct hook invocation attack

    let direct_execute_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::ExecuteTransferHook {
            source_token: alice_token_pubkey,

            mint: mint_pubkey,

            destination_token: bob_token_pubkey,

            transfer_authority: alice_pubkey,

            extra_account_meta_list: extra_account_meta_list_pda,

            global_policy: global_policy_pda,

            sender_authorization: alice_authorization_pda,

            receiver_authorization: bob_authorization_pda,

            sender_stats: alice_stats_pda,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::Execute { amount: 1_000_000 }.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[direct_execute_ix],
        Some(&payer.pubkey()),
        &[&payer],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(tx)
        .expect_err("direct hook invocation should fail");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("InvalidTransferHookInvocation"),
        "expected InvalidTransferHookInvocation, got: {err_text}"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("Direct-hook attack rejected");

    // ----------------------Test 10 — User cannot bypass hook by omitting hook accounts

    let transfer_without_hook_accounts = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        &alice_pubkey,
        &[],
        1_000_000,
        6,
    )
    .expect("failed to build transfer");

    let tx = Transaction::new_signed_with_payer(
        &[transfer_without_hook_accounts],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let result = svm.send_transaction(tx);

    assert!(
        result.is_err(),
        "transfer must not bypass Transfer Hook by omitting accounts"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("Missing-hook-accounts bypass rejected");

    // ---------------------------Test 11 — Fake/wrong ExtraAccountMetaList

    let mut wrong_eaml_ix = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        &alice_pubkey,
        &[],
        1_000_000,
        6,
    )
    .expect("failed to build transfer");

    wrong_eaml_ix.accounts.extend([
        // WRONG: canonical EAML intentionally omitted
        AccountMeta::new_readonly(global_policy_pda, false),
        AccountMeta::new_readonly(global_policy_pda, false),
        AccountMeta::new_readonly(alice_authorization_pda, false),
        AccountMeta::new_readonly(bob_authorization_pda, false),
        AccountMeta::new(alice_stats_pda, false),
        AccountMeta::new_readonly(token2022_compliance::ID, false),
    ]);

    let tx = Transaction::new_signed_with_payer(
        &[wrong_eaml_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "fake EAML must not be accepted"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("Fake ExtraAccountMetaList rejected");

    // ----------------Test 12 — Authorization-account substitution attack

    let mut substituted_auth_ix = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        &alice_pubkey,
        &[],
        1_000_000,
        6,
    )
    .expect("failed to build transfer");

    substituted_auth_ix.accounts.extend([
        AccountMeta::new_readonly(extra_account_meta_list_pda, false),
        AccountMeta::new_readonly(global_policy_pda, false),
        // WRONG sender authorization
        AccountMeta::new_readonly(bob_authorization_pda, false),
        AccountMeta::new_readonly(bob_authorization_pda, false),
        AccountMeta::new(alice_stats_pda, false),
        AccountMeta::new_readonly(token2022_compliance::ID, false),
    ]);

    let tx = Transaction::new_signed_with_payer(
        &[substituted_auth_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "authorization substitution must fail"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("Authorization substitution rejected");

    // ---------------------Test 13 — SenderStats substitution attack

    let mut substituted_stats_ix = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        &alice_pubkey,
        &[],
        1_000_000,
        6,
    )
    .expect("failed to build transfer");

    substituted_stats_ix.accounts.extend([
        AccountMeta::new_readonly(extra_account_meta_list_pda, false),
        AccountMeta::new_readonly(global_policy_pda, false),
        AccountMeta::new_readonly(alice_authorization_pda, false),
        AccountMeta::new_readonly(bob_authorization_pda, false),
        // WRONG account instead of Alice stats
        AccountMeta::new(bob_authorization_pda, false),
        AccountMeta::new_readonly(token2022_compliance::ID, false),
    ]);

    let tx = Transaction::new_signed_with_payer(
        &[substituted_stats_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "TransferStats substitution must fail"
    );

    assert_flow_state(&svm, 770_000_000, 227_700_000, 230_000_000, 230_000_000, 3);

    println!("TransferStats substitution rejected");

    // --------------------------------Test 14 — Exact daily-limit boundary

    let ix = build_hooked_transfer(20_000_000);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(tx)
        .expect("transfer reaching exact daily limit should succeed");

    assert_flow_state(&svm, 750_000_000, 247_500_000, 250_000_000, 250_000_000, 4);

    println!("Exact daily-limit boundary accepted");

    let ix = build_hooked_transfer(1_000_000);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(tx)
        .expect_err("transfer above exact daily limit must fail");

    assert!(format!("{err:?}").contains("DailyLimitExceeded"));

    assert_flow_state(&svm, 750_000_000, 247_500_000, 250_000_000, 250_000_000, 4);

    println!("Daily-limit + 1 rejection passed");

    // ------------------------------Test 15 — Multiple token accounts cannot bypass daily limit

    let alice_token_2 = Keypair::new();

    let alice_token_2_pubkey = alice_token_2.pubkey();

    let create_alice_token_2_ix = create_account(
        &payer.pubkey(),
        &alice_token_2_pubkey,
        token_account_rent,
        token_account_size as u64,
        &anchor_spl::token_2022::ID,
    );

    let initialize_alice_token_2_ix = initialize_account3(
        &anchor_spl::token_2022::ID,
        &alice_token_2_pubkey,
        &mint_pubkey,
        &alice_pubkey,
    )
    .expect("failed to initialize Alice second token account");

    let tx = Transaction::new_signed_with_payer(
        &[create_alice_token_2_ix, initialize_alice_token_2_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice_token_2],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(tx)
        .expect("failed to create Alice second token account");

    let mint_second_account_ix = mint_to_checked(
        &anchor_spl::token_2022::ID,
        &mint_pubkey,
        &alice_token_2_pubkey,
        &mint_authority_pubkey,
        &[],
        10_000_000,
        6,
    )
    .expect("failed to build mint instruction");

    let tx = Transaction::new_signed_with_payer(
        &[mint_second_account_ix],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(tx)
        .expect("failed to mint to Alice second account");

    let mut second_account_transfer = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_2_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        &alice_pubkey,
        &[],
        1_000_000,
        6,
    )
    .expect("failed to build second-account transfer");

    second_account_transfer.accounts.extend([
        AccountMeta::new_readonly(extra_account_meta_list_pda, false),
        AccountMeta::new_readonly(global_policy_pda, false),
        AccountMeta::new_readonly(alice_authorization_pda, false),
        AccountMeta::new_readonly(bob_authorization_pda, false),
        // SAME wallet-level stats PDA
        AccountMeta::new(alice_stats_pda, false),
        AccountMeta::new_readonly(token2022_compliance::ID, false),
    ]);

    let tx = Transaction::new_signed_with_payer(
        &[second_account_transfer],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(tx)
        .expect_err("second token account must not bypass daily limit");

    assert!(format!("{err:?}").contains("DailyLimitExceeded"));

    let alice_2_account = svm
        .get_account(&alice_token_2_pubkey)
        .expect("second Alice token account missing");

    let alice_2 = StateWithExtensions::<Token2022Account>::unpack(&alice_2_account.data)
        .expect("failed to parse second Alice token account");

    assert_eq!(alice_2.base.owner, alice_pubkey);

    assert_eq!(alice_2.base.amount, 10_000_000);

    assert_flow_state(&svm, 750_000_000, 247_500_000, 250_000_000, 250_000_000, 4);

    println!("Multiple-token-account daily-limit bypass rejected");

    // ---------------------------Test 16 — Daily limit resets on the next UTC day

    let mut clock = svm.get_sysvar::<Clock>();

    clock.unix_timestamp = 86_400;

    svm.set_sysvar::<Clock>(&clock);

    let ix = build_hooked_transfer(1_000_000);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(tx)
        .expect("first transfer on new day should succeed");

    let stats_account = svm
        .get_account(&alice_stats_pda)
        .expect("Alice stats missing");

    let mut stats_data = stats_account.data.as_slice();

    let stats = token2022_compliance::state::TransferStats::try_deserialize(&mut stats_data)
        .expect("failed to deserialize stats");

    assert_eq!(stats.day_index, 1);

    assert_eq!(stats.amount_today, 1_000_000);

    // Lifetime counter does NOT reset

    assert_eq!(stats.total_transferred, 251_000_000);

    assert_eq!(stats.transfer_count, 5);

    println!("Daily reset verified");

    println!("New day index: {}", stats.day_index);

    println!("New day's amount: {}", stats.amount_today);

    println!("Lifetime transferred: {}", stats.total_transferred);

    assert_flow_state(&svm, 749_000_000, 248_490_000, 1_000_000, 251_000_000, 5);

    // ---------------------------------------Test 17 — Delegate cannot change compliance identity

    let delegate = Keypair::new();

    let delegate_pubkey = delegate.pubkey();

    let approve_delegate_ix = approve_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &delegate_pubkey,
        &alice_pubkey,
        &[],
        5_000_000,
        6,
    )
    .expect("failed to build delegate approval");

    let tx = Transaction::new_signed_with_payer(
        &[approve_delegate_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(tx)
        .expect("failed to approve delegate");

    let mut delegated_transfer_ix = transfer_checked(
        &anchor_spl::token_2022::ID,
        &alice_token_pubkey,
        &mint_pubkey,
        &bob_token_pubkey,
        // transfer authority is the delegate
        &delegate_pubkey,
        &[],
        5_000_000,
        6,
    )
    .expect("failed to build delegated transfer");

    delegated_transfer_ix.accounts.extend([
        AccountMeta::new_readonly(extra_account_meta_list_pda, false),
        AccountMeta::new_readonly(global_policy_pda, false),
        // Compliance identity is still Alice
        AccountMeta::new_readonly(alice_authorization_pda, false),
        AccountMeta::new_readonly(bob_authorization_pda, false),
        // Stats are still Alice's
        AccountMeta::new(alice_stats_pda, false),
        AccountMeta::new_readonly(token2022_compliance::ID, false),
    ]);

    let tx = Transaction::new_signed_with_payer(
        &[delegated_transfer_ix],
        Some(&payer.pubkey()),
        // Alice does NOT sign the transfer
        &[&payer, &delegate],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(tx)
        .expect("valid delegated transfer should succeed");

    assert_flow_state(
        &svm,
        744_000_000,
        253_440_000,
        // Day 1: previous 1 + delegated 5
        6_000_000,
        // lifetime: 251 + 5
        256_000_000,
        6,
    );

    println!("Delegate identity test passed");

    // ========================================================================
    // DYNAMIC POLICY ENGINE TESTS
    // Tests 18 onward
    // ========================================================================

    // ------------------------------------------------------------
    // Helper: read current GlobalPolicy
    // ------------------------------------------------------------
    let read_policy = |svm: &LiteSVM| {
        let policy_account = svm
            .get_account(&global_policy_pda)
            .expect("GlobalPolicy missing");

        let mut policy_data = policy_account.data.as_slice();

        token2022_compliance::state::GlobalPolicy::try_deserialize(&mut policy_data)
            .expect("failed to deserialize GlobalPolicy")
    };

    // ------------------------------------------------------------
    // Helper: build update_policy instruction
    // ------------------------------------------------------------
    let build_update_policy_ix =
        |admin_key: Pubkey,
         args: token2022_compliance::instructions::update_policy::UpdatePolicyArgs| {
            Instruction {
                program_id: token2022_compliance::ID,

                accounts: token2022_compliance::accounts::UpdatePolicy {
                    admin: admin_key,
                    mint: mint_pubkey,
                    global_policy: global_policy_pda,
                    token_program: anchor_spl::token_2022::ID,
                }
                .to_account_metas(None),

                data: token2022_compliance::instruction::UpdatePolicy { args }.data(),
            }
        };

    // ========================================================================
    // TEST 18
    // Partial policy update:
    // max transfer 100 -> 50
    // other fields must remain unchanged
    // version 1 -> 2
    // ========================================================================

    let update_max_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: None,
            max_transfer_amount: Some(50_000_000),
            daily_transfer_limit: None,
            expires_at: None,
        },
    );

    let update_max_tx = Transaction::new_signed_with_payer(
        &[update_max_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(update_max_tx)
        .expect("failed to update max transfer amount");

    let policy = read_policy(&svm);

    assert!(policy.enabled);
    assert_eq!(policy.max_transfer_amount, 50_000_000);
    assert_eq!(policy.daily_transfer_limit, 250_000_000);
    assert_eq!(policy.expires_at, 0);
    assert_eq!(policy.policy_version, 2);

    println!("Policy max-transfer update passed");
    println!("New max transfer: {}", policy.max_transfer_amount);
    println!("Policy version: {}", policy.policy_version);

    // ========================================================================
    // TEST 19
    // New max-transfer rule must affect the Transfer Hook immediately.
    // 40 <= 50 -> succeeds
    // ========================================================================

    let transfer_40_after_update_ix = build_hooked_transfer(40_000_000);

    let transfer_40_after_update_tx = Transaction::new_signed_with_payer(
        &[transfer_40_after_update_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(transfer_40_after_update_tx)
        .expect("40-token transfer should succeed under new max");

    assert_flow_state(&svm, 704_000_000, 293_040_000, 46_000_000, 296_000_000, 7);

    println!("Updated max-transfer rule accepted valid 40-token transfer");

    // ========================================================================
    // TEST 20
    // 60 > new max of 50 -> must fail
    // ========================================================================

    let transfer_60_ix = build_hooked_transfer(60_000_000);

    let transfer_60_tx = Transaction::new_signed_with_payer(
        &[transfer_60_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(transfer_60_tx)
        .expect_err("60-token transfer must exceed updated maximum");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("MaxTransferAmountExceeded"),
        "expected MaxTransferAmountExceeded, got: {err_text}"
    );

    assert_flow_state(&svm, 704_000_000, 293_040_000, 46_000_000, 296_000_000, 7);

    println!("Updated max-transfer rejection passed");

    // ========================================================================
    // TEST 21
    // Non-admin cannot update policy
    // ========================================================================

    let policy_attacker = Keypair::new();
    let policy_attacker_pubkey = policy_attacker.pubkey();

    let attacker_update_ix = build_update_policy_ix(
        policy_attacker_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: Some(false),
            max_transfer_amount: None,
            daily_transfer_limit: None,
            expires_at: None,
        },
    );

    let attacker_update_tx = Transaction::new_signed_with_payer(
        &[attacker_update_ix],
        Some(&payer.pubkey()),
        &[&payer, &policy_attacker],
        fresh_blockhash(&mut svm),
    );

    let attacker_result = svm.send_transaction(attacker_update_tx);

    assert!(
        attacker_result.is_err(),
        "non-admin must not be able to update policy"
    );

    let policy = read_policy(&svm);

    assert!(policy.enabled);
    assert_eq!(policy.max_transfer_amount, 50_000_000);
    assert_eq!(policy.daily_transfer_limit, 250_000_000);
    assert_eq!(policy.policy_version, 2);

    println!("Unauthorized policy-admin attack rejected");

    // ========================================================================
    // TEST 22A
    // max_transfer_amount = 0 must fail
    // ========================================================================

    let invalid_max_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: None,
            max_transfer_amount: Some(0),
            daily_transfer_limit: None,
            expires_at: None,
        },
    );

    let invalid_max_tx = Transaction::new_signed_with_payer(
        &[invalid_max_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(invalid_max_tx)
        .expect_err("zero max transfer must fail");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("InvalidMaxTransferAmount"),
        "expected InvalidMaxTransferAmount, got: {err_text}"
    );

    assert_eq!(read_policy(&svm).policy_version, 2);

    println!("Invalid zero max-transfer update rejected");

    // ========================================================================
    // TEST 22B
    // daily limit < current max must fail
    //
    // current max = 50
    // attempted daily = 40
    // ========================================================================

    let invalid_daily_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: None,
            max_transfer_amount: None,
            daily_transfer_limit: Some(40_000_000),
            expires_at: None,
        },
    );

    let invalid_daily_tx = Transaction::new_signed_with_payer(
        &[invalid_daily_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(invalid_daily_tx)
        .expect_err("daily limit below max must fail");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("InvalidDailyLimit"),
        "expected InvalidDailyLimit, got: {err_text}"
    );

    assert_eq!(read_policy(&svm).policy_version, 2);

    println!("Invalid daily-limit update rejected");

    // ========================================================================
    // TEST 22C
    // expiration <= current time must fail
    // ========================================================================

    let current_timestamp = svm.get_sysvar::<Clock>().unix_timestamp;

    let invalid_expiry_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: None,
            max_transfer_amount: None,
            daily_transfer_limit: None,
            expires_at: Some(current_timestamp),
        },
    );

    let invalid_expiry_tx = Transaction::new_signed_with_payer(
        &[invalid_expiry_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(invalid_expiry_tx)
        .expect_err("non-future expiration must fail");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("InvalidExpiration"),
        "expected InvalidExpiration, got: {err_text}"
    );

    assert_eq!(read_policy(&svm).policy_version, 2);

    println!("Invalid expiration update rejected");

    // ========================================================================
    // TEST 23
    // Disable policy
    // version 2 -> 3
    // ========================================================================

    let disable_policy_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: Some(false),
            max_transfer_amount: None,
            daily_transfer_limit: None,
            expires_at: None,
        },
    );

    let disable_policy_tx = Transaction::new_signed_with_payer(
        &[disable_policy_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(disable_policy_tx)
        .expect("failed to disable policy");

    let policy = read_policy(&svm);

    assert!(!policy.enabled);
    assert_eq!(policy.policy_version, 3);

    println!("Policy disabled successfully");

    // ========================================================================
    // TEST 23B
    // Transfer while policy disabled must fail
    // ========================================================================

    let disabled_transfer_ix = build_hooked_transfer(1_000_000);

    let disabled_transfer_tx = Transaction::new_signed_with_payer(
        &[disabled_transfer_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(disabled_transfer_tx)
        .expect_err("transfer must fail while policy disabled");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("PolicyDisabled"),
        "expected PolicyDisabled, got: {err_text}"
    );

    assert_flow_state(&svm, 704_000_000, 293_040_000, 46_000_000, 296_000_000, 7);

    println!("Disabled-policy transfer rejected");

    // ========================================================================
    // TEST 24
    // Re-enable policy
    // version 3 -> 4
    // ========================================================================

    let enable_policy_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: Some(true),
            max_transfer_amount: None,
            daily_transfer_limit: None,
            expires_at: None,
        },
    );

    let enable_policy_tx = Transaction::new_signed_with_payer(
        &[enable_policy_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(enable_policy_tx)
        .expect("failed to re-enable policy");

    let policy = read_policy(&svm);

    assert!(policy.enabled);
    assert_eq!(policy.policy_version, 4);

    println!("Policy re-enabled successfully");

    // ========================================================================
    // TEST 25
    // Dynamically lower daily limit from 250 -> 50
    // version 4 -> 5
    //
    // amount_today currently = 46
    // ========================================================================

    let update_daily_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: None,
            max_transfer_amount: None,
            daily_transfer_limit: Some(50_000_000),
            expires_at: None,
        },
    );

    let update_daily_tx = Transaction::new_signed_with_payer(
        &[update_daily_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(update_daily_tx)
        .expect("failed to update daily limit");

    let policy = read_policy(&svm);

    assert_eq!(policy.max_transfer_amount, 50_000_000);
    assert_eq!(policy.daily_transfer_limit, 50_000_000);
    assert_eq!(policy.policy_version, 5);

    println!("Daily limit dynamically updated to 50 tokens");

    // ========================================================================
    // TEST 25B
    // 46 + 4 = exactly 50
    // Exact updated daily-limit boundary must succeed
    // ========================================================================

    let exact_daily_ix = build_hooked_transfer(4_000_000);

    let exact_daily_tx = Transaction::new_signed_with_payer(
        &[exact_daily_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(exact_daily_tx)
        .expect("exact updated daily limit should succeed");

    assert_flow_state(&svm, 700_000_000, 297_000_000, 50_000_000, 300_000_000, 8);

    println!("Updated daily-limit exact boundary passed");

    // ========================================================================
    // TEST 25C
    // One more token would make 51 > 50
    // ========================================================================

    let daily_plus_one_ix = build_hooked_transfer(1_000_000);

    let daily_plus_one_tx = Transaction::new_signed_with_payer(
        &[daily_plus_one_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(daily_plus_one_tx)
        .expect_err("updated daily limit must reject 51st token");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("DailyLimitExceeded"),
        "expected DailyLimitExceeded, got: {err_text}"
    );

    assert_flow_state(&svm, 700_000_000, 297_000_000, 50_000_000, 300_000_000, 8);

    println!("Updated daily-limit rejection passed");

    // ========================================================================
    // TEST 26
    // Set future expiry AND raise daily limit together
    //
    // daily 50 -> 100
    // expiry = now + 100 seconds
    // version 5 -> 6
    // ========================================================================

    let now = svm.get_sysvar::<Clock>().unix_timestamp;

    let future_expiry = now + 100;

    let expiry_update_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: None,
            max_transfer_amount: None,
            daily_transfer_limit: Some(100_000_000),
            expires_at: Some(future_expiry),
        },
    );

    let expiry_update_tx = Transaction::new_signed_with_payer(
        &[expiry_update_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(expiry_update_tx)
        .expect("failed to set policy expiration");

    let policy = read_policy(&svm);

    assert_eq!(policy.daily_transfer_limit, 100_000_000);
    assert_eq!(policy.expires_at, future_expiry);
    assert_eq!(policy.policy_version, 6);

    println!("Future policy expiration configured");

    // ========================================================================
    // TEST 26B
    // Transfer BEFORE expiry should succeed
    // ========================================================================

    let pre_expiry_transfer_ix = build_hooked_transfer(1_000_000);

    let pre_expiry_transfer_tx = Transaction::new_signed_with_payer(
        &[pre_expiry_transfer_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(pre_expiry_transfer_tx)
        .expect("transfer before expiry should succeed");

    assert_flow_state(&svm, 699_000_000, 297_990_000, 51_000_000, 301_000_000, 9);

    println!("Pre-expiry transfer passed");

    // ========================================================================
    // TEST 26C
    // Move Clock beyond expiration
    // ========================================================================

    let mut expired_clock = svm.get_sysvar::<Clock>();

    expired_clock.unix_timestamp = future_expiry + 1;

    svm.set_sysvar::<Clock>(&expired_clock);

    println!(
        "Clock moved beyond policy expiration: {}",
        expired_clock.unix_timestamp
    );

    // ========================================================================
    // TEST 26D
    // Transfer AFTER expiry must fail
    // ========================================================================

    let expired_transfer_ix = build_hooked_transfer(1_000_000);

    let expired_transfer_tx = Transaction::new_signed_with_payer(
        &[expired_transfer_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(expired_transfer_tx)
        .expect_err("expired policy must reject transfer");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("PolicyExpired"),
        "expected PolicyExpired, got: {err_text}"
    );

    assert_flow_state(&svm, 699_000_000, 297_990_000, 51_000_000, 301_000_000, 9);

    println!("Expired-policy transfer rejected");

    // ========================================================================
    // TEST 27
    // Clear expiration:
    // expires_at = 0
    //
    // version 6 -> 7
    // ========================================================================

    let clear_expiry_ix = build_update_policy_ix(
        admin_pubkey,
        token2022_compliance::instructions::update_policy::UpdatePolicyArgs {
            enabled: None,
            max_transfer_amount: None,
            daily_transfer_limit: None,
            expires_at: Some(0),
        },
    );

    let clear_expiry_tx = Transaction::new_signed_with_payer(
        &[clear_expiry_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(clear_expiry_tx)
        .expect("failed to clear policy expiration");

    let policy = read_policy(&svm);

    assert_eq!(policy.expires_at, 0);
    assert_eq!(policy.policy_version, 7);

    println!("Policy expiration cleared");

    // ========================================================================
    // TEST 27B
    // Transfer should work again even though Clock is past old expiry.
    // ========================================================================

    let after_clear_expiry_ix = build_hooked_transfer(1_000_000);

    let after_clear_expiry_tx = Transaction::new_signed_with_payer(
        &[after_clear_expiry_ix],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(after_clear_expiry_tx)
        .expect("transfer should work after clearing expiry");

    assert_flow_state(&svm, 698_000_000, 298_980_000, 52_000_000, 302_000_000, 10);

    println!("Transfer after clearing expiry passed");

    // ========================================================================
    // FINAL DYNAMIC POLICY ASSERTIONS
    // ========================================================================

    let final_policy = read_policy(&svm);

    assert!(final_policy.enabled);

    assert_eq!(final_policy.max_transfer_amount, 50_000_000);

    assert_eq!(final_policy.daily_transfer_limit, 100_000_000);

    assert_eq!(final_policy.expires_at, 0);

    assert_eq!(final_policy.policy_version, 7);

    println!("========================================");
    println!("Dynamic Policy Engine tests completed");
    println!("Final policy version: {}", final_policy.policy_version);
    println!("Final max transfer: {}", final_policy.max_transfer_amount);
    println!("Final daily limit: {}", final_policy.daily_transfer_limit);
    println!("Final enabled state: {}", final_policy.enabled);
    println!("Final expiration: {}", final_policy.expires_at);
    println!("========================================");

    let bob_final_account = svm
        .get_account(&bob_token_pubkey)
        .expect("Bob token account missing at final fee check");

    let bob_final_state = StateWithExtensions::<Token2022Account>::unpack(&bob_final_account.data)
        .expect("failed to parse Bob at final fee check");

    let bob_final_fee = bob_final_state
        .get_extension::<TransferFeeAmount>()
        .expect("Bob TransferFeeAmount missing at final fee check");

    assert_eq!(bob_final_state.base.amount, 298_980_000);
    assert_eq!(u64::from(bob_final_fee.withheld_amount), 3_020_000);

    println!(
        "Final Bob spendable balance: {}, withheld fees: {}",
        bob_final_state.base.amount,
        u64::from(bob_final_fee.withheld_amount)
    );

    // ========================================================================
    // SECURITY TEST — WRONG TRANSFER-HOOK PROGRAM
    //
    // Create a second Token-2022 mint whose TransferHook.program_id does NOT
    // point to our compliance program.
    //
    // Then directly invoke our Execute instruction.
    //
    // Expected:
    // InvalidTransferHookProgram
    // ========================================================================

    println!("Starting wrong Transfer-Hook program security test");

    // ------------------------------------------------------------------------
    // 1. Create second mint
    // ------------------------------------------------------------------------

    let wrong_hook_mint = Keypair::new();
    let wrong_hook_mint_pubkey = wrong_hook_mint.pubkey();

    // Deliberately configure this mint with another non-zero address
    // instead of our compliance program.
    let wrong_hook_program = Keypair::new();
    let wrong_hook_program_id = wrong_hook_program.pubkey();

    assert_ne!(wrong_hook_program_id, token2022_compliance::ID);

    assert_ne!(wrong_hook_program_id, anchor_spl::token_2022::ID);

    let create_wrong_hook_mint_ix = create_account(
        &payer.pubkey(),
        &wrong_hook_mint_pubkey,
        mint_rent,
        mint_size as u64,
        &anchor_spl::token_2022::ID,
    );

    let initialize_wrong_transfer_hook_ix = transfer_hook_instruction::initialize(
        &anchor_spl::token_2022::ID,
        &wrong_hook_mint_pubkey,
        Some(mint_authority_pubkey),
        Some(wrong_hook_program_id),
    )
    .expect("failed to build wrong-hook TransferHook initialization");

    let initialize_wrong_transfer_fee_ix =
        transfer_fee_instruction::initialize_transfer_fee_config(
            &anchor_spl::token_2022::ID,
            &wrong_hook_mint_pubkey,
            Some(&mint_authority_pubkey),
            Some(&mint_authority_pubkey),
            transfer_fee_basis_points,
            maximum_transfer_fee,
        )
        .expect("failed to build wrong-hook TransferFeeConfig initialization");

    let initialize_wrong_mint_ix = initialize_mint2(
        &anchor_spl::token_2022::ID,
        &wrong_hook_mint_pubkey,
        &mint_authority_pubkey,
        None,
        6,
    )
    .expect("failed to build wrong-hook mint initialization");

    let create_wrong_hook_mint_tx = Transaction::new_signed_with_payer(
        &[
            create_wrong_hook_mint_ix,
            initialize_wrong_transfer_hook_ix,
            initialize_wrong_transfer_fee_ix,
            initialize_wrong_mint_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer, &wrong_hook_mint],
        fresh_blockhash(&mut svm),
    );
    svm.send_transaction(create_wrong_hook_mint_tx)
        .expect("failed to create wrong-hook mint");

    // ------------------------------------------------------------------------
    // 2. Verify the second mint really points somewhere else
    // ------------------------------------------------------------------------

    let wrong_hook_mint_account = svm
        .get_account(&wrong_hook_mint_pubkey)
        .expect("wrong-hook mint missing");

    let parsed_wrong_hook_mint =
        StateWithExtensions::<Token2022Mint>::unpack(&wrong_hook_mint_account.data)
            .expect("failed to parse wrong-hook mint");

    let wrong_transfer_hook = parsed_wrong_hook_mint
        .get_extension::<TransferHook>()
        .expect("TransferHook extension missing on wrong-hook mint");

    let configured_wrong_hook: Option<Pubkey> = wrong_transfer_hook.program_id.into();

    assert_eq!(configured_wrong_hook, Some(wrong_hook_program_id));

    assert_ne!(configured_wrong_hook, Some(token2022_compliance::ID));

    println!(
        "Wrong-hook mint configured with program: {:?}",
        configured_wrong_hook
    );

    // ------------------------------------------------------------------------
    // 3. Create source + destination Token-2022 accounts for second mint
    // ------------------------------------------------------------------------

    let wrong_source_token = Keypair::new();
    let wrong_destination_token = Keypair::new();

    let wrong_source_token_pubkey = wrong_source_token.pubkey();

    let wrong_destination_token_pubkey = wrong_destination_token.pubkey();

    let create_wrong_source_ix = create_account(
        &payer.pubkey(),
        &wrong_source_token_pubkey,
        token_account_rent,
        token_account_size as u64,
        &anchor_spl::token_2022::ID,
    );

    let initialize_wrong_source_ix = initialize_account3(
        &anchor_spl::token_2022::ID,
        &wrong_source_token_pubkey,
        &wrong_hook_mint_pubkey,
        &alice_pubkey,
    )
    .expect("failed to initialize wrong-hook source account");

    let create_wrong_destination_ix = create_account(
        &payer.pubkey(),
        &wrong_destination_token_pubkey,
        token_account_rent,
        token_account_size as u64,
        &anchor_spl::token_2022::ID,
    );

    let initialize_wrong_destination_ix = initialize_account3(
        &anchor_spl::token_2022::ID,
        &wrong_destination_token_pubkey,
        &wrong_hook_mint_pubkey,
        &bob_pubkey,
    )
    .expect("failed to initialize wrong-hook destination account");

    let create_wrong_token_accounts_tx = Transaction::new_signed_with_payer(
        &[
            create_wrong_source_ix,
            initialize_wrong_source_ix,
            create_wrong_destination_ix,
            initialize_wrong_destination_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer, &wrong_source_token, &wrong_destination_token],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(create_wrong_token_accounts_tx)
        .expect("failed to create wrong-hook token accounts");

    // ------------------------------------------------------------------------
    // 4. Create GlobalPolicy PDA for second mint
    // ------------------------------------------------------------------------

    let (wrong_policy_pda, _) = Pubkey::find_program_address(
        &[b"policy", wrong_hook_mint_pubkey.as_ref()],
        &token2022_compliance::ID,
    );

    let initialize_wrong_policy_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializePolicy {
            admin: admin_pubkey,
            mint: wrong_hook_mint_pubkey,
            global_policy: wrong_policy_pda,
            token_program: anchor_spl::token_2022::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializePolicy {
            max_transfer_amount: 100_000_000,
            daily_transfer_limit: 250_000_000,
            expires_at: 0,
        }
        .data(),
    };

    let initialize_wrong_policy_tx = Transaction::new_signed_with_payer(
        &[initialize_wrong_policy_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(initialize_wrong_policy_tx)
        .expect("failed to initialize wrong-hook policy");

    // ------------------------------------------------------------------------
    // 5. Create Alice + Bob Authorization PDAs for second mint
    // ------------------------------------------------------------------------

    let (wrong_alice_auth_pda, _) = Pubkey::find_program_address(
        &[
            b"authorization",
            wrong_hook_mint_pubkey.as_ref(),
            alice_pubkey.as_ref(),
        ],
        &token2022_compliance::ID,
    );

    let (wrong_bob_auth_pda, _) = Pubkey::find_program_address(
        &[
            b"authorization",
            wrong_hook_mint_pubkey.as_ref(),
            bob_pubkey.as_ref(),
        ],
        &token2022_compliance::ID,
    );

    let initialize_wrong_alice_auth_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeAuthorization {
            admin: admin_pubkey,
            mint: wrong_hook_mint_pubkey,
            global_policy: wrong_policy_pda,
            authorization: wrong_alice_auth_pda,
            token_program: anchor_spl::token_2022::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeAuthorization {
            wallet: alice_pubkey,
        }
        .data(),
    };

    let initialize_wrong_bob_auth_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeAuthorization {
            admin: admin_pubkey,
            mint: wrong_hook_mint_pubkey,
            global_policy: wrong_policy_pda,
            authorization: wrong_bob_auth_pda,
            token_program: anchor_spl::token_2022::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeAuthorization { wallet: bob_pubkey }
            .data(),
    };

    let initialize_wrong_auth_tx = Transaction::new_signed_with_payer(
        &[initialize_wrong_alice_auth_ix, initialize_wrong_bob_auth_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(initialize_wrong_auth_tx)
        .expect("failed to initialize wrong-hook authorization accounts");

    // ------------------------------------------------------------------------
    // 6. Authorize Alice and Bob
    // ------------------------------------------------------------------------

    let authorize_wrong_alice_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::SetAuthorizationStatus {
            admin: admin_pubkey,
            mint: wrong_hook_mint_pubkey,
            global_policy: wrong_policy_pda,
            authorization: wrong_alice_auth_pda,
            token_program: anchor_spl::token_2022::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::SetAuthorizationStatus {
            new_status: token2022_compliance::state::AuthorizationStatus::Authorized,
        }
        .data(),
    };

    let authorize_wrong_bob_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::SetAuthorizationStatus {
            admin: admin_pubkey,
            mint: wrong_hook_mint_pubkey,
            global_policy: wrong_policy_pda,
            authorization: wrong_bob_auth_pda,
            token_program: anchor_spl::token_2022::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::SetAuthorizationStatus {
            new_status: token2022_compliance::state::AuthorizationStatus::Authorized,
        }
        .data(),
    };

    let authorize_wrong_users_tx = Transaction::new_signed_with_payer(
        &[authorize_wrong_alice_ix, authorize_wrong_bob_ix],
        Some(&admin_pubkey),
        &[&admin],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(authorize_wrong_users_tx)
        .expect("failed to authorize wrong-hook users");

    // ------------------------------------------------------------------------
    // 7. Initialize Alice TransferStats for second mint
    // ------------------------------------------------------------------------

    let (wrong_alice_stats_pda, _) = Pubkey::find_program_address(
        &[
            b"stats",
            wrong_hook_mint_pubkey.as_ref(),
            alice_pubkey.as_ref(),
        ],
        &token2022_compliance::ID,
    );

    let initialize_wrong_stats_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeTransferStats {
            payer: payer.pubkey(),
            mint: wrong_hook_mint_pubkey,
            authorization: wrong_alice_auth_pda,
            transfer_stats: wrong_alice_stats_pda,
            token_program: anchor_spl::token_2022::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeTransferStats {
            wallet: alice_pubkey,
        }
        .data(),
    };

    let initialize_wrong_stats_tx = Transaction::new_signed_with_payer(
        &[initialize_wrong_stats_ix],
        Some(&payer.pubkey()),
        &[&payer],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(initialize_wrong_stats_tx)
        .expect("failed to initialize wrong-hook stats");

    // ------------------------------------------------------------------------
    // 8. Initialize canonical ExtraAccountMetaList for second mint
    // ------------------------------------------------------------------------

    let (wrong_eaml_pda, _) = Pubkey::find_program_address(
        &[b"extra-account-metas", wrong_hook_mint_pubkey.as_ref()],
        &token2022_compliance::ID,
    );

    let initialize_wrong_eaml_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::InitializeExtraAccountMetaList {
            payer: payer.pubkey(),
            extra_account_meta_list: wrong_eaml_pda,
            mint: wrong_hook_mint_pubkey,
            mint_authority: mint_authority_pubkey,
            token_program: anchor_spl::token_2022::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::InitializeExtraAccountMetaList {}.data(),
    };

    let initialize_wrong_eaml_tx = Transaction::new_signed_with_payer(
        &[initialize_wrong_eaml_ix],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        fresh_blockhash(&mut svm),
    );

    svm.send_transaction(initialize_wrong_eaml_tx)
        .expect("failed to initialize wrong-hook EAML");

    // ------------------------------------------------------------------------
    // 9. Directly invoke our compliance Execute instruction
    //
    // Because verify_transfer_hook_program() runs BEFORE
    // verify_is_transferring(), the expected failure is specifically:
    //
    // InvalidTransferHookProgram
    // ------------------------------------------------------------------------

    let direct_wrong_hook_execute_ix = Instruction {
        program_id: token2022_compliance::ID,

        accounts: token2022_compliance::accounts::ExecuteTransferHook {
            source_token: wrong_source_token_pubkey,
            mint: wrong_hook_mint_pubkey,
            destination_token: wrong_destination_token_pubkey,
            transfer_authority: alice_pubkey,
            extra_account_meta_list: wrong_eaml_pda,
            global_policy: wrong_policy_pda,
            sender_authorization: wrong_alice_auth_pda,
            receiver_authorization: wrong_bob_auth_pda,
            sender_stats: wrong_alice_stats_pda,
        }
        .to_account_metas(None),

        data: token2022_compliance::instruction::Execute { amount: 1_000_000 }.data(),
    };

    let direct_wrong_hook_execute_tx = Transaction::new_signed_with_payer(
        &[direct_wrong_hook_execute_ix],
        Some(&payer.pubkey()),
        &[&payer],
        fresh_blockhash(&mut svm),
    );

    let err = svm
        .send_transaction(direct_wrong_hook_execute_tx)
        .expect_err("mint configured with another hook program must be rejected");

    let err_text = format!("{err:?}");

    assert!(
        err_text.contains("InvalidTransferHookProgram"),
        "expected InvalidTransferHookProgram, got: {err_text}"
    );

    println!("Wrong Transfer-Hook program attack rejected");
}
