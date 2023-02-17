use graphql::errors::*;
use graphql::perro::OptionToError;
use graphql::schema::*;
use graphql::{build_client, post_blocking};
use honey_badger::Auth;
use std::sync::Arc;

pub struct ExchangeRatesProvider {
    backend_url: String,
    auth: Arc<Auth>,
}

impl ExchangeRatesProvider {
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
        let variables = get_exchange_rates::Variables { code };
        let data = post_blocking::<GetExchangeRates>(&client, &self.backend_url, variables)?;
        let rate = data
            .currency
            .first()
            .ok_or_invalid_input("Unknown curency")?
            .sats_per_unit;

        Ok(rate)
    }
}
