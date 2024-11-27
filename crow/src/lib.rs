use graphql::perro::{ensure, permanent_failure};
use graphql::schema::list_uncompleted_topups::{topup_status_enum, ListUncompletedTopupsTopup};
use graphql::schema::{
    complete_topup_setup, hide_topup, list_uncompleted_topups, register_notification_token,
    start_topup_setup, CompleteTopupSetup, HideTopup, ListUncompletedTopups,
    RegisterNotificationToken, StartTopupSetup,
};
use graphql::{build_client, parse_from_rfc3339, post_blocking, ExchangeRate};
use honeybadger::Auth;
use std::sync::Arc;
use std::time::SystemTime;

use graphql::perro::runtime_error;
use graphql::schema::complete_topup_setup::CompleteTopupSetupCompleteTopupSetup;
use graphql::schema::start_topup_setup::StartTopupSetupRequest;
pub use isocountry::CountryCode;
pub use isolanguage_1::LanguageCode;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TopupStatus {
    READY,
    FAILED,
    REFUNDED,
    SETTLED,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporaryFailureCode {
    NoRoute,
    InvoiceExpired,
    Unexpected,
    Unknown { msg: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermanentFailureCode {
    ThresholdExceeded,
    OrderInactive,
    CompaniesUnsupported,
    CountryUnsupported,
    OtherRiskDetected,
    CustomerRequested,
    AccountNotMatching,
    PayoutExpired,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TopupError {
    TemporaryFailure { code: TemporaryFailureCode },
    PermanentFailure { code: PermanentFailureCode },
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TopupInfo {
    pub id: String,
    pub status: TopupStatus,

    pub amount_sat: u64,
    pub topup_value_minor_units: u64,
    pub exchange_fee_rate_permyriad: u16,
    pub exchange_fee_minor_units: u64,
    pub exchange_rate: ExchangeRate,

    pub expires_at: Option<SystemTime>,
    pub lnurlw: Option<String>,
    pub error: Option<TopupError>,
}

pub struct FiatTopupSetupChallenge {
    pub id: String,
    pub challenge: String,
}

/// Information about a fiat top-up registration
#[derive(Debug, Clone, PartialEq)]
pub struct FiatTopupSetupInfo {
    pub order_id: String,
    /// The user should transfer fiat from this IBAN
    pub debitor_iban: String,
    /// This reference should be included in the fiat transfer reference
    pub creditor_reference: String,
    /// The user should transfer fiat to this IBAN
    pub creditor_iban: String,
    pub creditor_bank_name: String,
    pub creditor_bank_street: String,
    pub creditor_bank_postal_code: String,
    pub creditor_bank_town: String,
    pub creditor_bank_country: String,
    pub creditor_bank_bic: String,
    pub creditor_name: String,
    pub creditor_street: String,
    pub creditor_postal_code: String,
    pub creditor_town: String,
    pub creditor_country: String,
    pub currency: String,
}

impl From<CompleteTopupSetupCompleteTopupSetup> for FiatTopupSetupInfo {
    fn from(value: CompleteTopupSetupCompleteTopupSetup) -> Self {
        Self {
            order_id: value.id,
            debitor_iban: value.debitor_iban,
            creditor_reference: value.creditor_reference,
            creditor_iban: value.creditor_iban,
            creditor_bank_name: value.creditor_bank_name,
            creditor_bank_street: value.creditor_bank_street,
            creditor_bank_postal_code: value.creditor_bank_postal_code,
            creditor_bank_town: value.creditor_bank_town,
            creditor_bank_country: value.creditor_bank_country,
            creditor_bank_bic: value.creditor_bank_bic,
            creditor_name: value.creditor_name,
            creditor_street: value.creditor_street,
            creditor_postal_code: value.creditor_postal_code,
            creditor_town: value.creditor_town,
            creditor_country: value.creditor_country,
            currency: value.currency,
        }
    }
}

pub struct OfferManager {
    backend_url: String,
    auth: Arc<Auth>,
}

impl OfferManager {
    pub fn new(backend_url: String, auth: Arc<Auth>) -> Self {
        Self { backend_url, auth }
    }

    pub fn start_topup_setup(
        &self,
        node_pubkey: String,
        provider: String,
        source_iban: String,
        user_currency: String,
        email: Option<String>,
        referral_code: Option<String>,
    ) -> graphql::Result<FiatTopupSetupChallenge> {
        let variables = start_topup_setup::Variables {
            request: StartTopupSetupRequest {
                node_pubkey,
                provider,
                source_iban,
                user_currency,
                email,
                referral_code,
            },
        };
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<StartTopupSetup>(&client, &self.backend_url, variables)?;

        Ok(FiatTopupSetupChallenge {
            id: data.start_topup_setup.id,
            challenge: data.start_topup_setup.challenge,
        })
    }

    pub fn complete_topup_setup(
        &self,
        id: String,
        signed_challenge: String,
        source_iban: String,
    ) -> graphql::Result<FiatTopupSetupInfo> {
        let variables = complete_topup_setup::Variables {
            id,
            signed_challenge,
            source_iban,
        };
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<CompleteTopupSetup>(&client, &self.backend_url, variables)?;

        Ok(data.complete_topup_setup.into())
    }

    pub fn register_notification_token(
        &self,
        notification_token: String,
        language: LanguageCode,
        country: CountryCode,
    ) -> graphql::Result<()> {
        let variables = register_notification_token::Variables {
            notification_token,
            language: format!("{}-{}", language.code(), country.alpha2()),
        };
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        let data =
            post_blocking::<RegisterNotificationToken>(&client, &self.backend_url, variables)?;
        ensure!(matches!(
            data.register_notification_token,
            Some(
                register_notification_token::RegisterNotificationTokenRegisterNotificationToken { .. }
            )
        ), permanent_failure("Backend rejected notification token registration"));
        Ok(())
    }

    pub fn hide_topup(&self, id: String) -> graphql::Result<()> {
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        post_blocking::<HideTopup>(&client, &self.backend_url, hide_topup::Variables { id })?;

        Ok(())
    }

    pub fn query_uncompleted_topups(&self) -> graphql::Result<Vec<TopupInfo>> {
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<ListUncompletedTopups>(
            &client,
            &self.backend_url,
            list_uncompleted_topups::Variables {},
        )?;
        data.topup.into_iter().map(to_topup_info).collect()
    }
}

fn to_topup_info(topup: ListUncompletedTopupsTopup) -> graphql::Result<TopupInfo> {
    let currency_code = topup.user_currency.to_string().to_uppercase();
    let sats_per_unit = (100_000_000_f64 / topup.exchange_rate).round() as u32;
    let created_at = parse_from_rfc3339(&topup.created_at)?;
    let exchange_rate = ExchangeRate {
        currency_code,
        sats_per_unit,
        updated_at: created_at,
    };

    let topup_value_minor_units = (topup.amount_user_currency * 100_f64).round() as u64;
    let exchange_fee_rate_permyriad = (topup.exchange_fee_rate * 10_000_f64).round() as u16;
    let exchange_fee_minor_units = (topup.exchange_fee_user_currency * 100_f64).round() as u64;
    let expires_at = match topup.expires_at {
        Some(e) => Some(parse_from_rfc3339(&e)?),
        None => None,
    };
    let lnurlw = topup.lnurl;

    let status = match topup.status {
        topup_status_enum::FAILED => TopupStatus::FAILED,
        topup_status_enum::READY => TopupStatus::READY,
        topup_status_enum::REFUNDED => TopupStatus::REFUNDED,
        topup_status_enum::SETTLED => TopupStatus::SETTLED,
        topup_status_enum::REFUND_HIDDEN => {
            runtime_error!(
                graphql::GraphQlRuntimeErrorCode::CorruptData,
                "The backend returned the unexpected status: REFUND_HIDDEN",
            );
        }
        topup_status_enum::Other(_) => {
            runtime_error!(
                graphql::GraphQlRuntimeErrorCode::CorruptData,
                "The backend returned an unknown topup status: {:?}",
                topup.status
            );
        }
    };

    let error = match topup.status {
        topup_status_enum::FAILED => to_topup_error(topup.additional_info),
        topup_status_enum::REFUNDED => to_topup_error(topup.additional_info),
        _ => None,
    };

    Ok(TopupInfo {
        id: topup.id,
        status,

        amount_sat: topup.amount_sat,
        topup_value_minor_units,
        exchange_fee_rate_permyriad,
        exchange_fee_minor_units,
        exchange_rate,

        expires_at,
        lnurlw,
        error,
    })
}

pub fn to_topup_error(code: Option<String>) -> Option<TopupError> {
    code.map(|c| match &*c {
        "no_route" => TopupError::TemporaryFailure {
            code: TemporaryFailureCode::NoRoute,
        },
        "invoice_expired" => TopupError::TemporaryFailure {
            code: TemporaryFailureCode::InvoiceExpired,
        },
        "error" => TopupError::TemporaryFailure {
            code: TemporaryFailureCode::Unexpected,
        },
        "threshold_exceeded" => TopupError::PermanentFailure {
            code: PermanentFailureCode::ThresholdExceeded,
        },
        "order_inactive" => TopupError::PermanentFailure {
            code: PermanentFailureCode::OrderInactive,
        },
        "companies_unsupported" => TopupError::PermanentFailure {
            code: PermanentFailureCode::CompaniesUnsupported,
        },
        "country_unsupported" => TopupError::PermanentFailure {
            code: PermanentFailureCode::CountryUnsupported,
        },
        "other_risk_detected" => TopupError::PermanentFailure {
            code: PermanentFailureCode::OtherRiskDetected,
        },
        "customer_requested" => TopupError::PermanentFailure {
            code: PermanentFailureCode::CustomerRequested,
        },
        "account_not_matching" => TopupError::PermanentFailure {
            code: PermanentFailureCode::AccountNotMatching,
        },
        "payout_expired" => TopupError::PermanentFailure {
            code: PermanentFailureCode::PayoutExpired,
        },
        e => TopupError::TemporaryFailure {
            code: TemporaryFailureCode::Unknown { msg: e.to_string() },
        },
    })
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::{
        to_topup_info, PermanentFailureCode, TemporaryFailureCode, TopupError, TopupStatus,
    };
    use graphql::schema::list_uncompleted_topups::{topup_status_enum, ListUncompletedTopupsTopup};

    const LNURL: &str = "LNURL1DP68GURN8GHJ7UR0VD4K2ARPWPCZ6EMFWSKHXARPVA5KUEEDWPHKX6M9W3SHQUPWWEJHYCM9DSHXZURS9ASHQ6F0D3H82UNV9AMKJARGV3EXZAE0XVUNQDNYVDJRGTF4XGEKXTF5X56NXTTZX3NRWTT9XDJRJEP4VE3XGD3KXVXTX4LS";

    #[test]
    fn test_topup_to_offer_info() {
        let amount_user_currency = 8.0;
        let topup = ListUncompletedTopupsTopup {
            additional_info: None,
            amount_sat: 42578,
            amount_user_currency,
            created_at: "2023-07-21T16:39:21.271+00:00".to_string(),
            exchange_fee_rate: 0.014999999664723873,
            exchange_fee_user_currency: 0.11999999731779099,
            exchange_rate: 18507.0,
            expires_at: Some("2023-09-21T16:39:21.919+00:00".to_string()),
            id: "1707e09e-ebe1-4004-abd7-7a64604501b3".to_string(),
            lightning_fee_user_currency: 0.0,
            lnurl: Some(LNURL.to_string()),
            node_pub_key: "0233786a3f5c79d25508ed973e7a37506ddab49d41a07fcb3d341ab638000d69cf"
                .to_string(),
            status: topup_status_enum::READY,
            user_currency: "eur".to_string(),
        };

        let topup_info = to_topup_info(topup).unwrap();
        assert_eq!(topup_info.id, "1707e09e-ebe1-4004-abd7-7a64604501b3");
        assert_eq!(topup_info.amount_sat, 42578);
        assert_eq!(topup_info.topup_value_minor_units, 800);
        assert_eq!(topup_info.exchange_fee_rate_permyriad, 150);
        assert_eq!(topup_info.exchange_fee_minor_units, 12);
        assert_eq!(topup_info.exchange_rate.currency_code, "EUR");
        assert_eq!(topup_info.exchange_rate.sats_per_unit, 5403);
        let updated_at = topup_info
            .exchange_rate
            .updated_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(updated_at, 1689957561);

        let expires_at = topup_info
            .expires_at
            .unwrap()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(expires_at, 1695314361);
        assert_eq!(topup_info.lnurlw.unwrap(), LNURL);

        assert_eq!(topup_info.status, TopupStatus::READY);

        let topup = ListUncompletedTopupsTopup {
            additional_info: Some("no_route".to_string()),
            amount_sat: 42578,
            amount_user_currency,
            created_at: "2023-07-21T16:39:21.271+00:00".to_string(),
            exchange_fee_rate: 0.014999999664723873,
            exchange_fee_user_currency: 0.11999999731779099,
            exchange_rate: 18507.0,
            expires_at: None,
            id: "1707e09e-ebe1-4004-abd7-7a64604501b3".to_string(),
            lightning_fee_user_currency: 0.0,
            lnurl: None,
            node_pub_key: "0233786a3f5c79d25508ed973e7a37506ddab49d41a07fcb3d341ab638000d69cf"
                .to_string(),
            status: topup_status_enum::FAILED,
            user_currency: "eur".to_string(),
        };

        let topup_info = to_topup_info(topup).unwrap();

        assert!(matches!(
            topup_info.error,
            Some(TopupError::TemporaryFailure {
                code: TemporaryFailureCode::NoRoute
            })
        ));
        assert!(topup_info.expires_at.is_none());
        assert!(topup_info.lnurlw.is_none());

        let topup = ListUncompletedTopupsTopup {
            additional_info: Some("customer_requested".to_string()),
            amount_sat: 42578,
            amount_user_currency,
            created_at: "2023-07-21T16:39:21.271+00:00".to_string(),
            exchange_fee_rate: 0.014999999664723873,
            exchange_fee_user_currency: 0.11999999731779099,
            exchange_rate: 18507.0,
            expires_at: Some("2023-09-21T16:39:21.919+00:00".to_string()),
            id: "1707e09e-ebe1-4004-abd7-7a64604501b3".to_string(),
            lightning_fee_user_currency: 0.0,
            lnurl: None,
            node_pub_key: "0233786a3f5c79d25508ed973e7a37506ddab49d41a07fcb3d341ab638000d69cf"
                .to_string(),
            status: topup_status_enum::REFUNDED,
            user_currency: "eur".to_string(),
        };

        let topup_info = to_topup_info(topup).unwrap();

        assert!(matches!(
            topup_info.error,
            Some(TopupError::PermanentFailure {
                code: PermanentFailureCode::CustomerRequested
            })
        ));
        assert!(topup_info.expires_at.is_some());
        assert!(topup_info.lnurlw.is_none());
    }
}
