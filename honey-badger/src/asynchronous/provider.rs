use crate::secrets::KeyPair;
use crate::signing::sign;

use crate::provider::{add_bitcoin_message_prefix, add_hex_prefix};
use crate::{AuthLevel, TermsAndConditions};
use graphql::perro::{ensure, invalid_input, permanent_failure, runtime_error, OptionToError};
use graphql::reqwest::Client;
use graphql::schema::*;
use graphql::{build_async_client, post};
use graphql::{errors::*, parse_from_rfc3339};
use log::info;
use std::time::SystemTime;

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
        let client = build_async_client(None)?;
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

    pub async fn query_token(&mut self) -> Result<String> {
        let (access_token, refresh_token) = match self.refresh_token.clone() {
            Some(refresh_token) => {
                match self.refresh_session(refresh_token).await {
                    // Tolerate authentication errors and retry auth flow.
                    Err(Error::RuntimeError {
                        code: GraphQlRuntimeErrorCode::AuthServiceError,
                        ..
                    }) => self.run_auth_flow().await,
                    result => result,
                }
            }
            None => self.run_auth_flow().await,
        }?;
        self.refresh_token = Some(refresh_token);
        Ok(access_token)
    }

    pub fn get_wallet_pubkey_id(&self) -> Option<String> {
        self.wallet_pubkey_id.clone()
    }

    pub async fn accept_terms_and_conditions(
        &self,
        access_token: String,
        terms: TermsAndConditions,
    ) -> Result<()> {
        info!("Accepting T&C ({:?})...", terms);
        ensure!(
            self.auth_level == AuthLevel::Pseudonymous,
            invalid_input("Accepting T&C not supported for auth levels other than Pseudonymous")
        );

        let variables = accept_terms_and_conditions::Variables {
            service_provider: terms.into(),
        };
        let client = build_async_client(Some(&access_token))?;
        let data = post::<AcceptTermsAndConditions>(&client, &self.backend_url, variables).await?;
        ensure!(
            matches!(
                data.accept_terms_conditions,
                Some(
                    accept_terms_and_conditions::AcceptTermsAndConditionsAcceptTermsConditions { .. }
                )
            ),
            permanent_failure("Backend rejected accepting Terms and Conditions")
        );
        Ok(())
    }

    async fn run_auth_flow(&mut self) -> Result<(String, String)> {
        let (access_token, refresh_token, wallet_pub_key_id) = self.start_basic_session().await?;

        self.wallet_pubkey_id = Some(wallet_pub_key_id.clone());

        match self.auth_level {
            AuthLevel::Pseudonymous => Ok((access_token, refresh_token)),
            AuthLevel::Owner => {
                self.start_priviledged_session(access_token, wallet_pub_key_id)
                    .await
            }
            AuthLevel::Employee => {
                let owner_pub_key_id = self
                    .get_business_owner(access_token.clone(), wallet_pub_key_id)
                    .await?;
                self.start_priviledged_session(access_token, owner_pub_key_id)
                    .await
            }
        }
    }

    async fn start_basic_session(&self) -> Result<(String, String, String)> {
        let challenge = self.request_challenge().await?;

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

        let data = post::<StartSession>(&self.client, &self.backend_url, variables).await?;

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
        #[cfg(debug_assertions)]
        info!("access_token: {}", access_token);
        #[cfg(debug_assertions)]
        info!("refresh_token: {}", refresh_token);
        info!("wallet_pub_key_id: {}", wallet_pub_key_id);
        Ok((access_token, refresh_token, wallet_pub_key_id))
    }

    async fn start_priviledged_session(
        &self,
        access_token: String,
        owner_pub_key_id: String,
    ) -> Result<(String, String)> {
        let challenge = self.request_challenge().await?;

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

        let client = build_async_client(Some(&access_token))?;
        let data = post::<PrepareWalletSession>(&client, &self.backend_url, variables).await?;

        let prepared_permission_token = data.prepare_wallet_session.ok_or_permanent_failure(
            "Response to prepare_wallet_session request doesn't have the expected structure",
        )?;

        info!("Starting wallet session ...");
        let variables = unlock_wallet::Variables {
            challenge,
            challenge_signature: add_hex_prefix(&challenge_signature),
            prepared_permission_token,
        };
        let data = post::<UnlockWallet>(&client, &self.backend_url, variables).await?;

        let session_permit = data.start_prepared_session.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure",
        )?;
        let access_token = session_permit.access_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing access token",
        )?;
        let refresh_token = session_permit.refresh_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing refresh token",
        )?;

        #[cfg(debug_assertions)]
        info!("access_token: {}", access_token);
        #[cfg(debug_assertions)]
        info!("refresh_token: {}", refresh_token);

        Ok((access_token, refresh_token))
    }

    async fn get_business_owner(
        &self,
        access_token: String,
        wallet_pub_key_id: String,
    ) -> Result<String> {
        info!("Getting business owner ...");
        let variables = get_business_owner::Variables {
            owner_wallet_pub_key_id: wallet_pub_key_id,
        };
        let client = build_async_client(Some(&access_token))?;
        let data = post::<GetBusinessOwner>(&client, &self.backend_url, variables).await?;

        let result = data
            .wallet_acl
            .first()
            .ok_or_invalid_input("Employee does not belong to any owner")?;

        if let Some(access_expires_at) = result.access_expires_at.as_ref() {
            let access_expires_at = parse_from_rfc3339(access_expires_at)?;
            ensure!(
                SystemTime::now() <= access_expires_at,
                runtime_error(GraphQlRuntimeErrorCode::AccessExpired, "Access expired")
            );
        }
        info!("Owner: {:?}", result.owner_wallet_pub_key_id);
        Ok(result.owner_wallet_pub_key_id.clone())
    }

    async fn refresh_session(&self, refresh_token: String) -> Result<(String, String)> {
        // Refresh session.
        info!("Refreshing session ...");
        let variables = refresh_session::Variables { refresh_token };
        let data = post::<RefreshSession>(&self.client, &self.backend_url, variables).await?;

        let session_permit = data.refresh_session.ok_or_permanent_failure(
            "Response to refresh_session request doesn't have the expected structure",
        )?;
        let access_token = session_permit.access_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing access token",
        )?;
        let refresh_token = session_permit.refresh_token.ok_or_permanent_failure(
            "Response to unlock_wallet request doesn't have the expected structure: missing refresh token",
        )?;

        #[cfg(debug_assertions)]
        info!("access_token: {}", access_token);
        #[cfg(debug_assertions)]
        info!("refresh_token: {}", refresh_token);

        Ok((access_token, refresh_token))
    }

    async fn request_challenge(&self) -> Result<String> {
        info!("Requesting challenge ...");
        let variables = request_challenge::Variables {};
        let data = post::<RequestChallenge>(&self.client, &self.backend_url, variables).await?;

        let challenge = data
            .auth_challenge
            .ok_or_permanent_failure(
                "Response to request_challenge request doesn't have the expected structure: missing auth challenge",
            )?;

        Ok(challenge)
    }
}
