use crate::secrets::KeyPair;
use crate::signing::sign;

use graphql::errors::*;
use graphql::perro::OptionToError;
use graphql::reqwest::blocking::Client;
use graphql::schema::*;
use graphql::{build_client, post_blocking};
use log::info;

pub enum AuthLevel {
    Pseudonymous,
    Owner,
    Employee,
}

pub(crate) struct AuthProvider {
    backend_url: String,
    auth_level: AuthLevel,
    wallet_keypair: KeyPair,
    auth_keypair: KeyPair,
    client: Client,
    refresh_token: Option<String>,
    wallet_pubkey_id: Option<String>,
}

impl AuthProvider {
    pub fn new(
        backend_url: String,
        auth_level: AuthLevel,
        wallet_keypair: KeyPair,
        auth_keypair: KeyPair,
    ) -> Result<Self> {
        let client = build_client(None)?;
        Ok(AuthProvider {
            backend_url,
            auth_level,
            wallet_keypair,
            auth_keypair,
            client,
            refresh_token: None,
            wallet_pubkey_id: None,
        })
    }

    pub fn query_token(&mut self) -> Result<String> {
        let (access_token, refresh_token) = match self.refresh_token.clone() {
            Some(refresh_token) => {
                match self.refresh_session(refresh_token) {
                    // Tolerate authentication errors and retry auth flow.
                    Err(Error::RuntimeError {
                        code: GraphQlRuntimeErrorCode::AuthServiceError,
                        ..
                    }) => self.run_auth_flow(),
                    result => result,
                }
            }
            None => self.run_auth_flow(),
        }?;
        self.refresh_token = Some(refresh_token);
        Ok(access_token)
    }

    pub fn get_wallet_pubkey_id(&self) -> Option<String> {
        self.wallet_pubkey_id.clone()
    }

    fn run_auth_flow(&mut self) -> Result<(String, String)> {
        let (access_token, refresh_token, wallet_pub_key_id) = self.start_basic_session()?;

        self.wallet_pubkey_id = Some(wallet_pub_key_id.clone());

        match self.auth_level {
            AuthLevel::Pseudonymous => Ok((access_token, refresh_token)),
            AuthLevel::Owner => self.start_priviledged_session(access_token, wallet_pub_key_id),
            AuthLevel::Employee => {
                let owner_pub_key_id = self
                    .get_business_owner(access_token.clone(), wallet_pub_key_id)?
                    .ok_or_invalid_input("Employee does not belong to any owner")?;
                self.start_priviledged_session(access_token, owner_pub_key_id)
            }
        }
    }

    fn start_basic_session(&self) -> Result<(String, String, String)> {
        let challenge = self.request_challenge()?;

        let challenge_with_prefix = add_bitcoin_message_prefix(&challenge);
        let challenge_signature = sign(challenge_with_prefix, self.auth_keypair.secret_key.clone());

        let auth_pub_key_with_prefix = add_hex_prefix(&self.auth_keypair.public_key);
        let signed_auth_pub_key = sign(
            auth_pub_key_with_prefix,
            self.wallet_keypair.secret_key.clone(),
        );

        info!("Starting session ...");
        let variables = start_session::Variables {
            auth_pub_key: add_hex_prefix(&self.auth_keypair.public_key),
            challenge,
            challenge_signature: add_hex_prefix(&challenge_signature),
            wallet_pub_key: add_hex_prefix(&self.wallet_keypair.public_key),
            signed_auth_pub_key: add_hex_prefix(&signed_auth_pub_key),
        };

        let data = post_blocking::<StartSession>(&self.client, &self.backend_url, variables)?;

        let session_permit = data.start_session_v2.ok_or_permanent_failure(
            "Response to start_session request doesn't have the expected structure",
        )?;
        let access_token = session_permit.access_token.ok_or_permanent_failure(
            "Response to start_session request doesn't have the expected structure: missing access token",
        )?;
        let refresh_token = session_permit.refresh_token.ok_or_permanent_failure(
            "Response to start_session request doesn't have the expected structure: missing refresh token",
        )?;
        let wallet_pub_key_id = session_permit.wallet_pub_key_id.ok_or_permanent_failure(
            "Response to start_session request doesn't have the expected structure: missing wallet public key id",
        )?;
        info!("access_token: {}", access_token);
        info!("refresh_token: {}", refresh_token);
        info!("wallet_pub_key_id: {}", wallet_pub_key_id);
        Ok((access_token, refresh_token, wallet_pub_key_id))
    }

    fn start_priviledged_session(
        &self,
        access_token: String,
        owner_pub_key_id: String,
    ) -> Result<(String, String)> {
        let challenge = self.request_challenge()?;

        let challenge_with_prefix = add_bitcoin_message_prefix(&challenge);
        let challenge_signature = sign(
            challenge_with_prefix,
            self.wallet_keypair.secret_key.clone(),
        );

        info!("Preparing wallet session ...");
        let variables = prepare_wallet_session::Variables {
            wallet_pub_key_id: owner_pub_key_id,
            challenge: challenge.clone(),
            signed_challenge: add_hex_prefix(&challenge_signature),
        };

        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<PrepareWalletSession>(&client, &self.backend_url, variables)?;

        let prepared_permission_token = data.prepare_wallet_session.ok_or_permanent_failure(
            "Response to prepare_wallet_session request doesn't have the expected structure",
        )?;

        info!("Starting wallet session ...");
        let variables = unlock_wallet::Variables {
            challenge,
            challenge_signature: add_hex_prefix(&challenge_signature),
            prepared_permission_token,
        };
        let data = post_blocking::<UnlockWallet>(&client, &self.backend_url, variables)?;

        let session_permit = data.start_prepared_session.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure",
        )?;
        let access_token = session_permit.access_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing access token",
        )?;
        let refresh_token = session_permit.refresh_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing refresh token",
        )?;

        info!("access_token: {}", access_token);
        info!("refresh_token: {}", refresh_token);

        Ok((access_token, refresh_token))
    }

    fn get_business_owner(
        &self,
        access_token: String,
        wallet_pub_key_id: String,
    ) -> Result<Option<String>> {
        info!("Getting business owner ...");
        let variables = get_business_owner::Variables {
            owner_wallet_pub_key_id: wallet_pub_key_id,
        };
        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<GetBusinessOwner>(&client, &self.backend_url, variables)?;

        let result = data
            .wallet_acl
            .first()
            .map(|w| w.owner_wallet_pub_key_id.clone());
        info!("Owner: {:?}", result);
        Ok(result)
    }

    fn refresh_session(&self, refresh_token: String) -> Result<(String, String)> {
        // Refresh session.
        info!("Refreshing session ...");
        let variables = refresh_session::Variables { refresh_token };
        let data = post_blocking::<RefreshSession>(&self.client, &self.backend_url, variables)?;

        let session_permit = data.refresh_session.ok_or_permanent_failure(
            "Response to refresh_session request doesn't have the expected structure",
        )?;
        let access_token = session_permit.access_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing access token",
        )?;
        let refresh_token = session_permit.refresh_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing refresh token",
        )?;

        info!("access_token: {}", access_token);
        info!("refresh_token: {}", refresh_token);

        Ok((access_token, refresh_token))
    }

    fn request_challenge(&self) -> Result<String> {
        info!("Requesting challenge ...");
        let variables = request_challenge::Variables {};
        let data = post_blocking::<RequestChallenge>(&self.client, &self.backend_url, variables)?;

        let challenge = data
            .auth_challenge
            .ok_or_permanent_failure(
                "Response to request_challenge request doesn't have the expected structure: missing auth challenge",
            )?;

        Ok(challenge)
    }
}

fn add_hex_prefix(string: &str) -> String {
    ["\\x", string].concat()
}

fn add_bitcoin_message_prefix(string: &str) -> String {
    ["\\x18Bitcoin Signed Message:", string].concat()
}
