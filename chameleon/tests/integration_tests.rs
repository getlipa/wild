use bitcoin::Network;
use chameleon::ExchangeRateProvider;
use graphql::perro::Error;
use honey_badger::secrets::{derive_keys, generate_keypair, generate_mnemonic};
use honey_badger::{Auth, AuthLevel};
use simplelog::TestLogger;
use std::env;
use std::sync::{Arc, Once};
use std::time::{Duration, SystemTime};

static INIT_LOGGER_ONCE: Once = Once::new();

#[cfg(test)]
#[ctor::ctor]
fn init() {
    INIT_LOGGER_ONCE.call_once(|| {
        TestLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default()).unwrap();
    });
}

#[test]
fn test_list_currency_codes() {
    let provider = build_provider();
    let list = provider.list_currency_codes().unwrap();
    assert!(list.contains(&"EUR".to_string()));
    assert!(list.contains(&"USD".to_string()));
    assert!(list.contains(&"CHF".to_string()));
}

#[test]
fn test_get_exchange_rate() {
    let provider = build_provider();
    let rate = provider.query_exchange_rate("EUR".to_string()).unwrap();
    assert!(1000 < rate);
    assert!(rate < 10000);

    let result = provider.query_exchange_rate("XXX".to_string());
    assert!(matches!(result, Err(Error::InvalidInput { .. })));
}

#[test]
fn test_get_all_exchange_rates() {
    let provider = build_provider();
    let currency_list = provider.list_currency_codes().unwrap();
    let exchange_rate_list = provider.query_all_exchange_rates().unwrap();
    assert!(exchange_rate_list
        .iter()
        .all(|item| currency_list.contains(&item.currency_code)
            && item.sats_per_unit > 0
            && (SystemTime::now() - Duration::from_secs(60 * 30)) < item.updated_at));
}

fn build_provider() -> ExchangeRateProvider {
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

    ExchangeRateProvider::new(get_backend_url(), Arc::new(auth))
}

fn get_backend_url() -> String {
    env::var("GRAPHQL_API_URL").expect("GRAPHQL_API_URL environment variable is not set")
}
