use bitcoin::Network;
use crow::OfferManager;
use honey_badger::secrets::{derive_keys, generate_keypair, generate_mnemonic};
use honey_badger::{Auth, AuthLevel};
use isocountry::CountryCode;
use isolanguage_1::LanguageCode;
use std::env;
use std::sync::Arc;

#[test]
fn test_register_topup() {
    let manager = build_offer_manager();
    let node_pubkey = generate_keypair().public_key;
    manager
        .register_topup(
            "order_id_1".to_string(),
            node_pubkey,
            Some("satoshi@lipa.swiss".to_string()),
        )
        .unwrap();

    let node_pubkey = generate_keypair().public_key;
    manager
        .register_topup("order_id_2".to_string(), node_pubkey, None)
        .unwrap();
}

#[test]
fn test_query_uncompleted_topups() {
    let manager = build_offer_manager();
    manager.query_uncompleted_topups().unwrap();
}

#[test]
fn test_register_notification_token() {
    let manager = build_offer_manager();
    let notification_token = generate_keypair().public_key;
    manager
        .register_notification_token(notification_token, LanguageCode::En, CountryCode::GBR)
        .unwrap();
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
