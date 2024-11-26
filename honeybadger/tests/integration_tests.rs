use bdk::bitcoin::Network;
use graphql::errors::{Error, GraphQlRuntimeErrorCode};
use honeybadger::secrets::{derive_keys, generate_keypair, generate_mnemonic, KeyPair};
use honeybadger::{Auth, AuthLevel, TermsAndConditions};
use simplelog::TestLogger;
use std::env;
use std::sync::Once;
use std::thread::sleep;
use std::time::Duration;

static INIT_LOGGER_ONCE: Once = Once::new();

#[cfg(test)]
#[ctor::ctor]
fn init() {
    INIT_LOGGER_ONCE.call_once(|| {
        TestLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default()).unwrap();
    });
}

#[test]
fn test_invalid_url() {
    let (wallet_keypair, auth_keypair) = generate_keys();

    let auth = Auth::new(
        "localhost:9".to_string(),
        AuthLevel::Pseudonymous,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();

    let result = auth.get_wallet_pubkey_id();
    assert!(matches!(
        result,
        Err(Error::RuntimeError {
            code: GraphQlRuntimeErrorCode::NetworkError,
            ..
        })
    ));

    let result = auth.query_token();
    assert!(matches!(
        result,
        Err(Error::RuntimeError {
            code: GraphQlRuntimeErrorCode::NetworkError,
            ..
        })
    ));
}

#[test]
fn test_502_return() {
    let (wallet_keypair, auth_keypair) = generate_keys();

    let auth = Auth::new(
        "https://httpstat.us/502".to_string(),
        AuthLevel::Pseudonymous,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();

    let result = auth.get_wallet_pubkey_id();
    assert!(matches!(
        result,
        Err(Error::RuntimeError {
            code: GraphQlRuntimeErrorCode::RemoteServiceUnavailable,
            ..
        })
    ));

    let result = auth.query_token();
    assert!(matches!(
        result,
        Err(Error::RuntimeError {
            code: GraphQlRuntimeErrorCode::RemoteServiceUnavailable,
            ..
        })
    ));
}

#[test]
fn test_basic_auth() {
    let (wallet_keypair, auth_keypair) = generate_keys();

    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Pseudonymous,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();

    let _id = auth.get_wallet_pubkey_id().unwrap();

    let token = auth.query_token().unwrap();
    let next_token = auth.query_token().unwrap();
    assert_eq!(token, next_token);

    let id = auth.get_wallet_pubkey_id().unwrap();

    sleep(Duration::from_secs(1));
    let refreshed_token = auth.refresh_token().unwrap();
    assert_ne!(token, refreshed_token);

    assert_eq!(auth.get_wallet_pubkey_id().unwrap(), id);
}

#[test]
fn test_owner_auth() {
    let (wallet_keypair, auth_keypair) = generate_keys();

    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Owner,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();

    let _id = auth.get_wallet_pubkey_id().unwrap();

    let token = auth.query_token().unwrap();
    let next_token = auth.query_token().unwrap();
    assert_eq!(token, next_token);

    let id = auth.get_wallet_pubkey_id().unwrap();

    sleep(Duration::from_secs(1));
    let refreshed_token = auth.refresh_token().unwrap();
    assert_ne!(token, refreshed_token);

    assert_eq!(auth.get_wallet_pubkey_id().unwrap(), id);
}

#[test]
fn test_employee_with_no_owner_auth() {
    let (wallet_keypair, auth_keypair) = generate_keys();

    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Employee,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();

    let result = auth.get_wallet_pubkey_id();
    assert!(matches!(result, Err(Error::InvalidInput { .. })));

    let result = auth.query_token();
    assert!(matches!(result, Err(Error::InvalidInput { .. })));
}

#[test]
// Is being ignored because it involves preceding manual steps (being invited and accept the invite).
#[ignore]
fn test_employee_auth() {
    let (wallet_keypair, auth_keypair) = generate_keys();

    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Employee,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();

    let _id = auth.get_wallet_pubkey_id().unwrap();

    let token = auth.query_token().unwrap();
    let next_token = auth.query_token().unwrap();
    assert_eq!(token, next_token);

    let id = auth.get_wallet_pubkey_id().unwrap();

    sleep(Duration::from_secs(1));
    let refreshed_token = auth.refresh_token().unwrap();
    assert_ne!(token, refreshed_token);

    assert_eq!(auth.get_wallet_pubkey_id().unwrap(), id);
}

#[test]
fn test_accept_terms_and_conditions() {
    let (wallet_keypair, auth_keypair) = generate_keys();
    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Pseudonymous,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();
    auth.accept_terms_and_conditions(
        TermsAndConditions::Lipa,
        3,
        "b90025a5df2b7e45b458181289c74d74c4e74b2d7a5589b4af89d952c3e1181c".into(),
    )
    .unwrap();
    assert_eq!(
        auth.get_terms_and_conditions_status(TermsAndConditions::Lipa)
            .unwrap()
            .version,
        3
    );

    let (wallet_keypair, auth_keypair) = generate_keys();
    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Owner,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();
    let result =
        auth.accept_terms_and_conditions(TermsAndConditions::Lipa, 3, "fingerprint2".into());
    assert!(
        matches!(result, Err(Error::InvalidInput { msg }) if msg.contains("Accepting T&C not supported for auth levels other than Pseudonymous"))
    );

    let (wallet_keypair, auth_keypair) = generate_keys();
    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Pseudonymous,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();
    let result =
        auth.accept_terms_and_conditions(TermsAndConditions::Pocket, 3, "fingerprint3".into());
    assert!(
        matches!(result, Err(Error::InvalidInput { msg }) if msg.contains("The provided fingerprint is invalid"))
    );

    let (wallet_keypair, auth_keypair) = generate_keys();
    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Owner,
        wallet_keypair,
        auth_keypair,
    )
    .unwrap();
    let result =
        auth.accept_terms_and_conditions(TermsAndConditions::Pocket, 4, "fingerprint4".into());
    assert!(
        matches!(result, Err(Error::InvalidInput { msg }) if msg.contains("Accepting T&C not supported for auth levels other than Pseudonymous"))
    );
}

fn generate_keys() -> (KeyPair, KeyPair) {
    println!("Generating keys ...");
    let mnemonic = generate_mnemonic();
    println!("mnemonic: {mnemonic:?}");
    let wallet_keys = derive_keys(Network::Testnet, mnemonic).wallet_keypair;
    let auth_keys = generate_keypair();

    (wallet_keys, auth_keys)
}

fn get_backend_url() -> String {
    env::var("GRAPHQL_API_URL").expect("GRAPHQL_API_URL environment variable is not set")
}
