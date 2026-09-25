use litesvm::LiteSVM;
use std::path::PathBuf;

#[test]
fn token2022_compliance_flow() {
    let mut svm = LiteSVM::new();

    let program_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/token2022_compliance.so");

    svm.add_program_from_file(
        token2022_compliance::ID,
        &program_path,
    )
    .expect("failed to load token2022-compliance program");

    let token_2022_account = svm
        .get_account(&anchor_spl::token_2022::ID)
        .expect("Token-2022 program is not loaded in LiteSVM");

    assert!(
        token_2022_account.executable,
        "Token-2022 account exists but is not executable"
    );
}