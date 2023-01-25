use bdk::bitcoin::Network;
use honey_badger::errors::{AuthError, AuthRuntimeErrorCode};
use honey_badger::secrets::{derive_keys, generate_keypair, generate_mnemonic, KeyPair};
use honey_badger::{Auth, AuthLevel};
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

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_none());

    let result = auth.query_token();
    assert!(matches!(
        result,
        Err(AuthError::RuntimeError {
            code: AuthRuntimeErrorCode::NetworkError,
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

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_none());

    let token = auth.query_token().unwrap();
    let next_token = auth.query_token().unwrap();
    assert_eq!(token, next_token);

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_some());

    sleep(Duration::from_secs(1));
    let refreshed_token = auth.refresh_token().unwrap();
    assert_ne!(token, refreshed_token);

    assert_eq!(auth.get_wallet_pubkey_id(), id);
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

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_none());

    let token = auth.query_token().unwrap();
    let next_token = auth.query_token().unwrap();
    assert_eq!(token, next_token);

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_some());

    sleep(Duration::from_secs(1));
    let refreshed_token = auth.refresh_token().unwrap();
    assert_ne!(token, refreshed_token);

    assert_eq!(auth.get_wallet_pubkey_id(), id);
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

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_none());

    let result = auth.query_token();
    assert!(matches!(result, Err(AuthError::InvalidInput { .. })));
}

#[test]
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

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_none());

    let token = auth.query_token().unwrap();
    let next_token = auth.query_token().unwrap();
    assert_eq!(token, next_token);

    let id = auth.get_wallet_pubkey_id();
    assert!(id.is_some());

    sleep(Duration::from_secs(1));
    let refreshed_token = auth.refresh_token().unwrap();
    assert_ne!(token, refreshed_token);

    assert_eq!(auth.get_wallet_pubkey_id(), id);
}

fn generate_keys() -> (KeyPair, KeyPair) {
    println!("Generating keys ...");
    let mnemonic = generate_mnemonic();
    println!("mnemonic: {:?}", mnemonic);
    let wallet_keys = derive_keys(Network::Testnet, mnemonic).wallet_keypair;
    let auth_keys = generate_keypair();

    (wallet_keys, auth_keys)
}

fn get_backend_url() -> String {
    env::var("GRAPHQL_API_URL").expect("GRAPHQL_API_URL environment variable is not set")
}
