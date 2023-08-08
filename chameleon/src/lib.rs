use graphql::perro::OptionToError;
use graphql::schema::*;
use graphql::{build_client, post_blocking};
use graphql::{errors::*, parse_from_rfc3339};
use honey_badger::Auth;
use std::sync::Arc;
use std::time::SystemTime;

pub struct ExchangeRate {
    pub currency_code: String,
    pub sats_per_unit: u32,
    pub updated_at: SystemTime,
}

pub struct ExchangeRateProvider {
    backend_url: String,
    auth: Arc<Auth>,
}

impl ExchangeRateProvider {
    pub fn new(backend_url: String, auth: Arc<Auth>) -> Self {
        Self { backend_url, auth }
    }

    pub fn list_currency_codes(&self) -> Result<Vec<String>> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = list_currency_codes::Variables {};
        let data = post_blocking::<ListCurrencyCodes>(&client, &self.backend_url, variables)?;
        let list = data.currency.into_iter().map(|c| c.currency_code).collect();

        Ok(list)
    }

    pub fn query_exchange_rate(&self, code: String) -> Result<u32> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = get_exchange_rate::Variables { code };
        let data = post_blocking::<GetExchangeRate>(&client, &self.backend_url, variables)?;
        let rate = data
            .currency
            .first()
            .ok_or_invalid_input("Unknown currency")?
            .sats_per_unit;

        Ok(rate)
    }

    pub fn query_all_exchange_rates(&self) -> Result<Vec<ExchangeRate>> {
        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        let variables = get_all_exchange_rates::Variables {};
        let data = post_blocking::<GetAllExchangeRates>(&client, &self.backend_url, variables)?;
        let list: Vec<Result<ExchangeRate>> = data
            .currency
            .into_iter()
            .map(|c| {
                Ok(ExchangeRate {
                    currency_code: c.currency_code,
                    sats_per_unit: c.sats_per_unit,
                    updated_at: parse_from_rfc3339(&c.conversion_rate_updated_at)?,
                })
            })
            .collect();

        list.into_iter().collect()
    }
}
