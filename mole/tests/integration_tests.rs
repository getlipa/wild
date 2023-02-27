use bitcoin::Network;
use honey_badger::secrets::{derive_keys, generate_keypair, generate_mnemonic};
use honey_badger::{Auth, AuthLevel};
use mole::ChannelStatePersistenceClient;
use rand::Rng;
use serial_test::{parallel, serial};
use simplelog::TestLogger;
use std::env;
use std::sync::{Arc, Once};

static INIT_LOGGER_ONCE: Once = Once::new();

#[cfg(test)]
#[ctor::ctor]
fn init() {
    INIT_LOGGER_ONCE.call_once(|| {
        TestLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default()).unwrap();
    });
}

#[test]
#[parallel]
fn test_health_positive_check() {
    let storage_client = build_storage_client();

    assert!(storage_client.check_health());
}

#[test]
#[serial]
fn test_health_negative_check() {
    let original_url = env::var("GRAPHQL_HEALTH_URL").unwrap();
    std::env::set_var("GRAPHQL_HEALTH_URL", "http://localhost:9");

    let storage_client = build_storage_client();
    assert!(!storage_client.check_health());

    std::env::set_var("GRAPHQL_HEALTH_URL", original_url);
}

#[test]
fn test_channel_monitor_field_exists() {
    let client = build_storage_client();

    assert!(client.verify_channel_monitor_field_exists().unwrap());
}

#[test]
fn test_channel_manager_field_exists() {
    let client = build_storage_client();

    assert!(client.verify_channel_manager_field_exists().unwrap());
}

#[test]
fn test_channel_monitor_persistence() {
    let client = build_storage_client();

    // Channel 1: Dummy data
    let channel_id_0 = "11111111111111111111111111111111111111111111111111111111111111110000";
    let channel_monitors_0 = vec![
        rand::thread_rng().gen::<[u8; 32]>().to_vec(),
        rand::thread_rng().gen::<[u8; 32]>().to_vec(),
        rand::thread_rng().gen::<[u8; 32]>().to_vec(),
    ]; // 3 encrypted channel monitors

    // Channel 2: Dummy data
    let channel_id_1 = "22222222222222222222222222222222222222222222222222222222222222228888";
    let channel_monitor_1 = rand::thread_rng().gen::<[u8; 32]>().to_vec(); // Encrypted channel monitor

    let inst_id = "integration_test"; // installation id = Usually the current version of 3L
    let device = rand::thread_rng().gen::<[u8; 32]>().to_vec(); // Encrypted device info

    // For channel 1, simulate 3 channel updates
    for monitor in &channel_monitors_0 {
        client
            .write_channel_monitor(channel_id_0, &monitor, inst_id, &device)
            .unwrap();
    }

    // For channel 2, only write 1 channel state
    client
        .write_channel_monitor(channel_id_1, &channel_monitor_1, inst_id, &device)
        .unwrap();

    let retrieved_channel_ids = client.get_channel_monitor_ids().unwrap();
    assert_eq!(retrieved_channel_ids.len(), 2);
    assert_ne!(retrieved_channel_ids[0], retrieved_channel_ids[1]);
    for channel_id in retrieved_channel_ids {
        if channel_id != channel_id_0 && channel_id != channel_id_1 {
            panic!("Unexpected channel id: {}", channel_id);
        }
    }

    // must return the latest channel state
    let retrieved_channel_0 = client.read_channel_monitor(channel_id_0).unwrap();
    assert_eq!(retrieved_channel_0, channel_monitors_0[2]);

    let retrieved_channel_1 = client.read_channel_monitor(channel_id_1).unwrap();
    assert_eq!(retrieved_channel_1, channel_monitor_1);
}

#[test]
fn test_channel_manager_persistence() {
    let client = build_storage_client();
    let encrypted_channel_manager_dummy_older = rand::thread_rng().gen::<[u8; 32]>().to_vec();
    let encrypted_channel_manager_dummy_newer = rand::thread_rng().gen::<[u8; 32]>().to_vec();

    client
        .write_channel_manager(&encrypted_channel_manager_dummy_older)
        .unwrap();
    client
        .write_channel_manager(&encrypted_channel_manager_dummy_newer)
        .unwrap();

    let retrieved_channel_manager = client.read_channel_manager().unwrap();

    assert_eq!(
        retrieved_channel_manager,
        encrypted_channel_manager_dummy_newer
    );
}

fn build_storage_client() -> ChannelStatePersistenceClient {
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

    ChannelStatePersistenceClient::new(get_backend_url(), get_backend_health_url(), Arc::new(auth))
}

fn get_backend_url() -> String {
    env::var("GRAPHQL_API_URL").expect("GRAPHQL_API_URL environment variable is not set")
}

fn get_backend_health_url() -> String {
    env::var("GRAPHQL_HEALTH_URL").expect("GRAPHQL_HEALTH_URL environment variable is not set")
}
