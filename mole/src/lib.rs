use graphql::errors::*;
use graphql::perro::MapToError;
use graphql::reqwest::blocking::Client;
use graphql::schema::*;
use graphql::{build_client, post_blocking};
use honey_badger::Auth;
use std::sync::Arc;

pub struct ChannelStatePersistenceClient {
    backend_url: String,
    backend_health_url: String,
    auth: Arc<Auth>,
}

impl ChannelStatePersistenceClient {
    pub fn new(backend_url: String, backend_health_url: String, auth: Arc<Auth>) -> Self {
        Self {
            backend_url,
            backend_health_url,
            auth,
        }
    }

    pub fn check_health(&self) -> bool {
        // In the future, we might want to check the feasibility of the actual use case:
        //  - Can we authenticate or are we authenticated?
        //  - Can we actually write channel states to the backend?

        let client = Client::new();

        if let Ok(response) = client.get(&self.backend_health_url).send() {
            if response.status().is_success() {
                if let Ok(body) = response.text() {
                    if body == "OK" {
                        return true;
                    }
                }
            }
        }

        log::error!(
            "Backend health check failed for url: {}",
            self.backend_health_url
        );
        false
    }

    pub fn verify_channel_monitor_field_exists(&self) -> Result<bool> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = verify_channel_monitor_field_exists::Variables {};
        let response =
            post_blocking::<VerifyChannelMonitorFieldExists>(&client, &self.backend_url, variables);

        match response {
            Ok(_) => Ok(true),
            Err(err) => {
                if error_is_data_structure_related(&err) {
                    Ok(false)
                } else {
                    Err(err)
                }
            }
        }
    }

    pub fn verify_channel_manager_field_exists(&self) -> Result<bool> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = verify_channel_manager_field_exists::Variables {};
        let response =
            post_blocking::<VerifyChannelManagerFieldExists>(&client, &self.backend_url, variables);

        match response {
            Ok(_) => Ok(true),
            Err(err) => {
                if error_is_data_structure_related(&err) {
                    Ok(false)
                } else {
                    Err(err)
                }
            }
        }
    }

    pub fn write_channel_monitor(
        &self,
        channel_id: &str,
        encrypted_channel_monitor: &Vec<u8>,
        installation_id: &str,
        encrypted_device_info: &Vec<u8>,
    ) -> Result<()> {
        debug_assert!(
            channel_id.len() % 2 == 0,
            "GraphQL interface expects hex encoded strings to have an even number of characters"
        );

        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = insert_channel_monitor_one::Variables {
            channel_id: format!("\\x{channel_id}"),
            encrypted_channel_monitor: format!("\\x{}", hex::encode(encrypted_channel_monitor)),
            installation_id: installation_id.to_string(),
            encrypted_device_info: format!("\\x{}", hex::encode(encrypted_device_info)),
        };
        post_blocking::<InsertChannelMonitorOne>(&client, &self.backend_url, variables)?;

        Ok(())
    }

    pub fn get_channel_monitor_ids(&self) -> Result<Vec<String>> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = get_channel_monitor_channel_ids::Variables {};
        let data =
            post_blocking::<GetChannelMonitorChannelIds>(&client, &self.backend_url, variables)?;
        let list = data
            .channel_monitor
            .into_iter()
            .map(|cm| cm.channel_id.replace("\\x", ""))
            .collect();

        Ok(list)
    }

    pub fn read_channel_monitor(&self, channel_id: &str) -> Result<Vec<u8>> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = get_latest_channel_monitor::Variables {
            channel_id: format!("\\x{channel_id}"),
        };
        let data = post_blocking::<GetLatestChannelMonitor>(&client, &self.backend_url, variables)?;
        let binary = hex::decode(
            data.channel_monitor[0]
                .encrypted_channel_monitor
                .replace("\\x", ""),
        )
        .map_to_runtime_error(
            GraphQlRuntimeErrorCode::GenericError,
            "Could not decode hex encoded binary",
        )?;

        Ok(binary)
    }

    pub fn write_channel_manager(&self, encrypted_channel_manager: &Vec<u8>) -> Result<()> {
        let token = self.auth.query_token().unwrap();
        let client = build_client(Some(&token))?;
        let variables = insert_channel_manager_one::Variables {
            encrypted_channel_manager: format!("\\x{}", hex::encode(encrypted_channel_manager)),
        };
        post_blocking::<InsertChannelManagerOne>(&client, &self.backend_url, variables)?;

        Ok(())
    }

    pub fn read_channel_manager(&self) -> Result<Vec<u8>> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = get_latest_channel_manager::Variables {};
        let data = post_blocking::<GetLatestChannelManager>(&client, &self.backend_url, variables)?;
        hex::decode(
            data.channel_manager[0]
                .encrypted_channel_manager
                .replace("\\x", ""),
        )
        .map_to_runtime_error(
            GraphQlRuntimeErrorCode::GenericError,
            "Could not decode hex encoded binary",
        )
    }
}

fn error_is_data_structure_related(
    error: &graphql::perro::Error<graphql::GraphQlRuntimeErrorCode>,
) -> bool {
    matches!(
        error,
        Error::RuntimeError {
            code: GraphQlRuntimeErrorCode::GenericError,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphql::perro::runtime_error;

    #[test]
    fn test_nature_of_error() {
        let error = runtime_error(GraphQlRuntimeErrorCode::GenericError, "test");
        assert!(error_is_data_structure_related(&error));

        let error = runtime_error(GraphQlRuntimeErrorCode::NetworkError, "test");
        assert!(!error_is_data_structure_related(&error));

        let error = runtime_error(GraphQlRuntimeErrorCode::AuthServiceError, "test");
        assert!(!error_is_data_structure_related(&error));
    }
}
