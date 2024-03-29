use bitcoin::Network;
use honey_badger::asynchronous::Auth;
use honey_badger::secrets::{derive_keys, generate_keypair, generate_mnemonic};
use honey_badger::AuthLevel;
use pigeon::assign_lightning_address;
use simplelog::TestLogger;
use std::env;
use std::sync::Once;

static INIT_LOGGER_ONCE: Once = Once::new();

#[cfg(test)]
#[ctor::ctor]
fn init() {
    INIT_LOGGER_ONCE.call_once(|| {
        TestLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default()).unwrap();
    });
}

#[tokio::test]
async fn test_assigning_lightning_address() {
    let (backend_url, auth) = build_client();
    let address = assign_lightning_address(&backend_url, &auth).await.unwrap();
    println!("Assigned address is: {address}");
    assert!(!address.is_empty());
    let address_from_another_call = assign_lightning_address(&backend_url, &auth).await.unwrap();
    assert_eq!(address, address_from_another_call);

    let (backend_url, another_auth) = build_client();
    let address_for_another_user = assign_lightning_address(&backend_url, &another_auth)
        .await
        .unwrap();
    assert_ne!(address, address_for_another_user);
}

fn build_client() -> (String, Auth) {
    println!("Generating keys ...");
    let mnemonic = generate_mnemonic();
    println!("mnemonic: {mnemonic:?}");
    let wallet_keys = derive_keys(Network::Testnet, mnemonic).wallet_keypair;
    let auth_keys = generate_keypair();

    let auth = Auth::new(
        get_backend_url(),
        AuthLevel::Pseudonymous,
        wallet_keys,
        auth_keys,
    )
    .unwrap();

    (get_backend_url(), auth)
}

fn get_backend_url() -> String {
    env::var("GRAPHQL_API_URL").expect("GRAPHQL_API_URL environment variable is not set")
}
