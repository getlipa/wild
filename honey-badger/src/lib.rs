pub mod asynchronous;
mod jwt;
mod provider;
pub mod secrets;
mod signing;

pub use graphql;

pub use crate::provider::{AuthLevel, TermsAndConditions};

use crate::jwt::parse_token;
use crate::provider::AuthProvider;
use crate::secrets::KeyPair;

pub use graphql::errors::{GraphQlRuntimeErrorCode, Result};
use graphql::perro::{MapToError, OptionToError};
use std::cmp::{max, min};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

#[derive(Clone)]
struct AdjustedToken {
    raw: String,
    expires_at: SystemTime,
}

pub struct Auth {
    provider: Mutex<AuthProvider>,
    token: Mutex<AdjustedToken>,
}

#[derive(Debug, PartialEq)]
pub struct TermsAndConditionsStatus {
    pub accepted_at: Option<SystemTime>,
    pub terms_and_conditions: TermsAndConditions,
    pub version: i64,
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

    pub fn query_token(&self) -> Result<String> {
        if let Some(token) = self.get_token_if_valid() {
            return Ok(token);
        }

        let mut provider = self.provider.lock().unwrap();
        // Anyone else refreshed the token by chance?...
        if let Some(token) = self.get_token_if_valid() {
            return Ok(token);
        }

        let token = adjust_token(provider.query_token()?)?;
        *self.token.lock().unwrap() = token;
        self.get_token_if_valid()
            .ok_or_permanent_failure("Newly refreshed token is not valid long enough")
    }

    pub fn get_wallet_pubkey_id(&self) -> Option<String> {
        self.provider.lock().unwrap().get_wallet_pubkey_id()
    }

    // Not exposed in UDL, used in tests.
    pub fn refresh_token(&self) -> Result<String> {
        let mut provider = self.provider.lock().unwrap();
        let token = adjust_token(provider.query_token()?)?;
        *self.token.lock().unwrap() = token;
        self.get_token_if_valid()
            .ok_or_permanent_failure("Newly refreshed token is not valid long enough")
    }

    pub fn accept_terms_and_conditions(
        &self,
        terms: TermsAndConditions,
        version: i64,
    ) -> Result<()> {
        let token = self.query_token()?;
        let provider = self.provider.lock().unwrap();
        provider.accept_terms_and_conditions(token, terms, version)
    }

    pub fn get_terms_and_conditions_status(
        &self,
        terms: TermsAndConditions,
    ) -> Result<TermsAndConditionsStatus> {
        let token = self.query_token()?;
        let provider = self.provider.lock().unwrap();
        provider.get_terms_and_conditions_status(token, terms)
    }

    fn get_token_if_valid(&self) -> Option<String> {
        let now = SystemTime::now();
        let token = self.token.lock().unwrap();
        if now < token.expires_at {
            Some(token.raw.clone())
        } else {
            None
        }
    }
}

pub(crate) fn adjust_token(raw_token: String) -> Result<AdjustedToken> {
    let token = parse_token(raw_token).map_to_runtime_error(
        GraphQlRuntimeErrorCode::AuthServiceError,
        "Auth service returned invalid JWT",
    )?;

    let token_validity_period = token
        .expires_at
        .duration_since(token.received_at)
        .map_to_runtime_error(
            GraphQlRuntimeErrorCode::AuthServiceError,
            "Expiration date of JWT is in the past",
        )?;

    let leeway = compute_leeway(token_validity_period)?;
    let expires_at = token
        .expires_at
        .checked_sub(leeway)
        .ok_or_permanent_failure(format!(
            "Failed to substract leeway: {leeway:?} from tokent expire at: {:?}",
            token.expires_at
        ))?;
    debug_assert!(token.received_at < expires_at);

    Ok(AdjustedToken {
        raw: token.raw,
        expires_at,
    })
}

fn compute_leeway(period: Duration) -> Result<Duration> {
    let leeway_10_percents = period
        .checked_div(100 / 10)
        .ok_or_permanent_failure("Failed to divide duration")?;

    let leeway_50_percents = period
        .checked_div(100 / 50)
        .ok_or_permanent_failure("Failed to divide duration")?;

    // At least 10 seconds.
    let lower_bound = max(Duration::from_secs(10), leeway_10_percents);
    // At most 30 seconds.
    let upper_bound = min(Duration::from_secs(30), leeway_50_percents);
    // If 50% < 10 seconds, use 50% of the period.
    let leeway = min(lower_bound, upper_bound);

    Ok(leeway)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_compute_leeway() -> Result<()> {
        assert_eq!(compute_leeway(secs(    10))?, secs( 5));
        assert_eq!(compute_leeway(secs(    20))?, secs(10));
        assert_eq!(compute_leeway(secs(    30))?, secs(10));
        assert_eq!(compute_leeway(secs(    60))?, secs(10));
        assert_eq!(compute_leeway(secs(2 * 60))?, secs(12));
        assert_eq!(compute_leeway(secs(3 * 60))?, secs(18));
        assert_eq!(compute_leeway(secs(4 * 60))?, secs(24));
        assert_eq!(compute_leeway(secs(5 * 60))?, secs(30));
        assert_eq!(compute_leeway(secs(6 * 60))?, secs(30));
        Ok(())
    }

    fn secs(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }
}
