use graphql::errors::*;
use graphql::perro::{MapToError, OptionToError};
use graphql::schema::*;
use graphql::{build_async_client, post};
use honey_badger::asynchronous::Auth;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub struct Backup {
    pub encrypted_backup: Vec<u8>,
    pub schema_name: String,
    pub schema_version: String,
}

pub struct RemoteBackupClient {
    backend_url: String,
    auth: Arc<Auth>,
}

impl RemoteBackupClient {
    pub fn new(backend_url: String, auth: Arc<Auth>) -> Self {
        Self { backend_url, auth }
    }

    pub async fn create_backup(&self, backup: &Backup) -> Result<()> {
        let token = self.auth.query_token().await?;
        let client = build_async_client(Some(&token))?;
        let variables = create_backup::Variables {
            encrypted_backup: graphql_hex_encode(&backup.encrypted_backup),
            schema_name: backup.schema_name.clone(),
            schema_version: backup.schema_version.clone(),
        };
        post::<CreateBackup>(&client, &self.backend_url, variables).await?;

        Ok(())
    }

    pub async fn recover_backup(&self, schema_name: &str) -> Result<Backup> {
        let token = self.auth.query_token().await?;
        let client = build_async_client(Some(&token))?;
        let variables = recover_backup::Variables {
            schema_name: schema_name.to_string(),
        };
        let data = post::<RecoverBackup>(&client, &self.backend_url, variables).await?;

        let d = data.recover_backup.ok_or_runtime_error(
            GraphQlRuntimeErrorCode::ObjectNotFound,
            "No backup found with the provided schema name",
        )?;
        let encrypted_backup = graphql_hex_decode(&d.encrypted_backup.ok_or_runtime_error(
            GraphQlRuntimeErrorCode::ObjectNotFound,
            "No backup found with the provided schema name",
        )?)
        .map_to_runtime_error(
            GraphQlRuntimeErrorCode::CorruptData,
            "Encrypted backup invalid hex",
        )?;
        let schema_version = d
            .schema_version
            .ok_or_permanent_failure("Backend returned encrypted backup but no schema version")?;
        Ok(Backup {
            encrypted_backup,
            schema_name: schema_name.to_string(),
            schema_version,
        })
    }
}

fn graphql_hex_encode(data: &Vec<u8>) -> String {
    format!("\\x{}", hex::encode(data))
}

fn graphql_hex_decode(data: &str) -> Result<Vec<u8>> {
    hex::decode(data.replacen("\\x", "", 1)).map_to_runtime_error(
        GraphQlRuntimeErrorCode::CorruptData,
        "Could not decode hex encoded binary",
    )
}
