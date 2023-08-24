use graphql::perro::{permanent_failure, OptionToError};
use graphql::schema::list_available_topups::{topup_status_enum, ListAvailableTopupsTopup};
use graphql::schema::{
    list_available_topups, register_email, register_node, register_notification_token,
    ListAvailableTopups, RegisterEmail, RegisterNode, RegisterNotificationToken,
};
use graphql::{build_client, parse_from_rfc3339, post_blocking, ExchangeRate};
use honey_badger::Auth;
use std::sync::Arc;
use std::time::SystemTime;

use graphql::perro::Error::RuntimeError;
pub use isocountry::CountryCode;
pub use isolanguage_1::LanguageCode;

#[derive(Debug, PartialEq)]
pub enum TopupStatus {
    READY,
    FAILED,
    SETTLED,
}

pub struct TopupInfo {
    pub id: String,
    pub status: TopupStatus,

    pub amount_sat: u64,
    pub topup_value_minor_units: u64,
    pub exchange_fee_rate_permyriad: u16,
    pub exchange_fee_minor_units: u64,
    pub exchange_rate: ExchangeRate,

    pub expires_at: SystemTime,
    pub lnurlw: String,
}

pub struct OfferManager {
    backend_url: String,
    auth: Arc<Auth>,
}

impl OfferManager {
    pub fn new(backend_url: String, auth: Arc<Auth>) -> Self {
        Self { backend_url, auth }
    }

    pub fn register_email(&self, email: String) -> graphql::Result<()> {
        let variables = register_email::Variables { email };
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<RegisterEmail>(&client, &self.backend_url, variables)?;
        if !matches!(
            data.register_email,
            Some(register_email::RegisterEmailRegisterEmail { .. })
        ) {
            return Err(permanent_failure("Backend rejected email registration"));
        }
        Ok(())
    }

    pub fn register_node(&self, node_pubkey: String) -> graphql::Result<()> {
        let variables = register_node::Variables {
            node_pub_key: node_pubkey,
        };
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<RegisterNode>(&client, &self.backend_url, variables)?;
        if !matches!(
            data.register_node,
            Some(register_node::RegisterNodeRegisterNode { .. })
        ) {
            return Err(permanent_failure("Backend rejected node registration"));
        }
        Ok(())
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
        if !matches!(
            data.register_notification_token,
            Some(
                register_notification_token::RegisterNotificationTokenRegisterNotificationToken { .. }
            )
        ) {
            return Err(permanent_failure(
                "Backend rejected notification token registration",
            ));
        }

        Ok(())
    }

    pub fn query_available_topups(&self) -> graphql::Result<Vec<TopupInfo>> {
        let access_token = self.auth.query_token()?;
        let client = build_client(Some(&access_token))?;
        let data = post_blocking::<ListAvailableTopups>(
            &client,
            &self.backend_url,
            list_available_topups::Variables {},
        )?;
        data.topup.into_iter().map(to_topup_info).collect()
    }
}

fn to_topup_info(topup: ListAvailableTopupsTopup) -> graphql::Result<TopupInfo> {
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
    let expires_at = topup.expires_at.ok_or_runtime_error(
        graphql::GraphQlRuntimeErrorCode::CorruptData,
        "The backend returned an incomplete topup - missing expires_at",
    )?;
    let expires_at = parse_from_rfc3339(&expires_at)?;
    let lnurlw = topup.lnurl.ok_or_runtime_error(
        graphql::GraphQlRuntimeErrorCode::CorruptData,
        "The backend returned an incomplete topup - missing lnurlw",
    )?;

    let status = match topup.status {
        topup_status_enum::FAILED => TopupStatus::FAILED,
        topup_status_enum::READY => TopupStatus::READY,
        topup_status_enum::SETTLED => TopupStatus::SETTLED,
        topup_status_enum::Other(_) => {
            return Err(RuntimeError {
                code: graphql::GraphQlRuntimeErrorCode::CorruptData,
                msg: format!(
                    "The backend returned an unknown topup status: {:?}",
                    topup.status
                ),
            })
        }
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
    })
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::{to_topup_info, TopupStatus};
    use graphql::schema::list_available_topups::{topup_status_enum, ListAvailableTopupsTopup};

    const LNURL: &str = "LNURL1DP68GURN8GHJ7UR0VD4K2ARPWPCZ6EMFWSKHXARPVA5KUEEDWPHKX6M9W3SHQUPWWEJHYCM9DSHXZURS9ASHQ6F0D3H82UNV9AMKJARGV3EXZAE0XVUNQDNYVDJRGTF4XGEKXTF5X56NXTTZX3NRWTT9XDJRJEP4VE3XGD3KXVXTX4LS";

    #[test]
    fn test_topup_to_offer_info() {
        let amount_user_currency = 8.0;
        let topup = ListAvailableTopupsTopup {
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
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(expires_at, 1695314361);
        assert_eq!(topup_info.lnurlw, LNURL);

        assert_eq!(topup_info.status, TopupStatus::READY)
    }
}
