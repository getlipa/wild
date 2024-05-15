use bdk::bitcoin::Network;
use graphql::perro::Error::RuntimeError;
use graphql::GraphQlRuntimeErrorCode;
use honeybadger::asynchronous::Auth;
use honeybadger::secrets::{derive_keys, generate_keypair, generate_mnemonic};
use honeybadger::AuthLevel;
use rand::random;
use simplelog::TestLogger;
use squirrel::{Backup, RemoteBackupClient};
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

#[tokio::test]
async fn test_recovering_backup_when_there_is_none() {
    let client = build_backup_client();

    let result = client.recover_backup("a").await;

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(RuntimeError {
            code: GraphQlRuntimeErrorCode::ObjectNotFound,
            ..
        })
    ));
}

#[tokio::test]
async fn test_backup_persistence() {
    let client = build_backup_client();
    let dummy_backup_schema_a_version_1 = Backup {
        encrypted_backup: random::<[u8; 32]>().to_vec(),
        schema_name: "a".to_string(),
        schema_version: "1".to_string(),
    };
    let dummy_backup_schema_a_version_2 = Backup {
        encrypted_backup: random::<[u8; 32]>().to_vec(),
        schema_name: "a".to_string(),
        schema_version: "2".to_string(),
    };
    let dummy_backup_schema_b_version_1 = Backup {
        encrypted_backup: random::<[u8; 32]>().to_vec(),
        schema_name: "b".to_string(),
        schema_version: "1".to_string(),
    };
    let dummy_backup_schema_b_version_2 = Backup {
        encrypted_backup: random::<[u8; 32]>().to_vec(),
        schema_name: "b".to_string(),
        schema_version: "2".to_string(),
    };

    client
        .create_backup(&dummy_backup_schema_a_version_1)
        .await
        .unwrap();
    client
        .create_backup(&dummy_backup_schema_b_version_1)
        .await
        .unwrap();

    assert_eq!(
        client.recover_backup("a").await.unwrap(),
        dummy_backup_schema_a_version_1
    );
    assert_eq!(
        client.recover_backup("b").await.unwrap(),
        dummy_backup_schema_b_version_1
    );

    client
        .create_backup(&dummy_backup_schema_a_version_2)
        .await
        .unwrap();
    client
        .create_backup(&dummy_backup_schema_b_version_2)
        .await
        .unwrap();

    assert_eq!(
        client.recover_backup("a").await.unwrap(),
        dummy_backup_schema_a_version_2
    );
    assert_eq!(
        client.recover_backup("b").await.unwrap(),
        dummy_backup_schema_b_version_2
    );

    // non-existing schema name
    let result = client.recover_backup("c").await;

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(RuntimeError {
            code: GraphQlRuntimeErrorCode::ObjectNotFound,
            ..
        })
    ));
}

fn build_backup_client() -> RemoteBackupClient {
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

    RemoteBackupClient::new(get_backend_url(), Arc::new(auth))
}

fn get_backend_url() -> String {
    env::var("GRAPHQL_API_URL").expect("GRAPHQL_API_URL environment variable is not set")
}
