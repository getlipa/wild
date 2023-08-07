use bitcoin::Network;
use crow::OfferManager;
use honey_badger::secrets::{derive_keys, generate_keypair, generate_mnemonic};
use honey_badger::{Auth, AuthLevel};
use std::env;
use std::sync::Arc;

#[test]
fn test_register_email() {
    let manager = build_offer_manager();
    manager
        .register_email("satoshi@lipa.swiss".to_string())
        .unwrap();
}

#[test]
fn test_query_available_topups() {
    let manager = build_offer_manager();
    manager.query_available_topups().unwrap();
}

fn build_offer_manager() -> OfferManager {
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

    OfferManager::new(get_backend_url(), Arc::new(auth))
}

fn get_backend_url() -> String {
    env::var("GRAPHQL_API_URL").expect("GRAPHQL_API_URL environment variable is not set")
}
