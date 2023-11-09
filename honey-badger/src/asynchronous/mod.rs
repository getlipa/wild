mod provider;

pub use graphql;

use crate::asynchronous::provider::AuthProvider;
use crate::secrets::KeyPair;
use crate::{adjust_token, AdjustedToken, AuthLevel, CustomTermsAndConditions};
pub use graphql::errors::{GraphQlRuntimeErrorCode, Result};
use graphql::perro::OptionToError;
use std::time::SystemTime;
use tokio::sync::Mutex;

pub struct Auth {
    provider: Mutex<AuthProvider>,
    token: Mutex<AdjustedToken>,
}

impl Auth {
    pub fn new(
        backend_url: String,
        auth_level: AuthLevel,
        wallet_keypair: KeyPair,
        auth_keypair: KeyPair,
    ) -> Result<Self> {
        let provider = AuthProvider::new(backend_url, auth_level, wallet_keypair, auth_keypair)?;
        let expired_token = AdjustedToken {
            raw: String::new(),
            expires_at: SystemTime::UNIX_EPOCH,
        };
        Ok(Auth {
            provider: Mutex::new(provider),
            token: Mutex::new(expired_token),
        })
    }

    pub async fn query_token(&self) -> Result<String> {
        if let Some(token) = self.get_token_if_valid().await {
            return Ok(token);
        }

        let mut provider = self.provider.lock().await;
        // Anyone else refreshed the token by chance?...
        if let Some(token) = self.get_token_if_valid().await {
            return Ok(token);
        }

        let token = adjust_token(provider.query_token().await?)?;
        *self.token.lock().await = token;
        self.get_token_if_valid()
            .await
            .ok_or_permanent_failure("Newly refreshed token is not valid long enough")
    }

    pub async fn get_wallet_pubkey_id(&self) -> Option<String> {
        self.provider.lock().await.get_wallet_pubkey_id()
    }

    // Not exposed in UDL, used in tests.
    pub async fn refresh_token(&self) -> Result<String> {
        let mut provider = self.provider.lock().await;
        let token = adjust_token(provider.query_token().await?)?;
        *self.token.lock().await = token;
        self.get_token_if_valid()
            .await
            .ok_or_permanent_failure("Newly refreshed token is not valid long enough")
    }

    pub async fn accept_terms_and_conditions(&self) -> Result<()> {
        let token = self.query_token().await?;
        let provider = self.provider.lock().await;
        provider.accept_terms_and_conditions(token).await
    }

    pub async fn accept_custom_terms_and_conditions(
        &self,
        custom_terms: CustomTermsAndConditions,
    ) -> Result<()> {
        let token = self.query_token().await?;
        let provider = self.provider.lock().await;
        provider
            .accept_custom_terms_and_conditions(custom_terms, token)
            .await
    }

    async fn get_token_if_valid(&self) -> Option<String> {
        let now = SystemTime::now();
        let token = self.token.lock().await;
        if now < token.expires_at {
            Some(token.raw.clone())
        } else {
            None
        }
    }
}
