pub mod errors;
pub mod schema;

pub use crate::errors::*;

pub use perro;
pub use reqwest;

use chrono::{DateTime, Utc};
use graphql_client::reqwest::{post_graphql, post_graphql_blocking};
use graphql_client::Response;
use perro::{permanent_failure, runtime_error, MapToError, OptionToError};
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::StatusCode;
use std::time::{Duration, SystemTime};

pub struct ExchangeRate {
    pub currency_code: String,
    pub sats_per_unit: u32,
    pub updated_at: SystemTime,
}

pub fn build_client(access_token: Option<&str>) -> Result<Client> {
    let user_agent = "graphql-rust/0.12.0";
    let timeout = Some(Duration::from_secs(20));

    let mut builder = Client::builder().user_agent(user_agent).timeout(timeout);
    if let Some(access_token) = access_token {
        let value = HeaderValue::from_str(&format!("Bearer {access_token}"))
            .map_to_permanent_failure("Failed to build header value from str")?;
        builder = builder.default_headers(std::iter::once((AUTHORIZATION, value)).collect());
    }

    let client = builder
        .build()
        .map_to_permanent_failure("Failed to build a reqwest client")?;
    Ok(client)
}

pub fn build_async_client(access_token: Option<&str>) -> Result<reqwest::Client> {
    let user_agent = "graphql-rust/0.12.0";
    let timeout = Duration::from_secs(20);

    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(timeout);
    if let Some(access_token) = access_token {
        let value = HeaderValue::from_str(&format!("Bearer {access_token}"))
            .map_to_permanent_failure("Failed to build header value from str")?;
        builder = builder.default_headers(std::iter::once((AUTHORIZATION, value)).collect());
    }

    let client = builder
        .build()
        .map_to_permanent_failure("Failed to build a async reqwest client")?;
    Ok(client)
}

pub fn post_blocking<Query: graphql_client::GraphQLQuery>(
    client: &Client,
    backend_url: &String,
    variables: Query::Variables,
) -> Result<Query::ResponseData> {
    let response = match post_graphql_blocking::<Query, _>(client, backend_url, variables) {
        Ok(r) => r,
        Err(e) => {
            if is_502_status(e.status()) || e.to_string().contains("502") {
                // checking for the error containing 502 because reqwest is unexpectedly returning a decode error instead of status error
                return Err(runtime_error(
                    GraphQlRuntimeErrorCode::RemoteServiceUnavailable,
                    "The remote server returned status 502",
                ));
            }
            return Err(runtime_error(
                GraphQlRuntimeErrorCode::NetworkError,
                "Failed to execute the query",
            ));
        }
    };
    get_response_data(response, backend_url)
}

pub async fn post<Query: graphql_client::GraphQLQuery>(
    client: &reqwest::Client,
    backend_url: &String,
    variables: Query::Variables,
) -> Result<Query::ResponseData> {
    let response = match post_graphql::<Query, _>(client, backend_url, variables).await {
        Ok(r) => r,
        Err(e) => {
            if is_502_status(e.status()) || e.to_string().contains("502") {
                // checking for the error containing 502 because reqwest is unexpectedly returning a decode error instead of status error
                return Err(runtime_error(
                    GraphQlRuntimeErrorCode::RemoteServiceUnavailable,
                    "The remote server returned status 502",
                ));
            }
            return Err(runtime_error(
                GraphQlRuntimeErrorCode::NetworkError,
                "Failed to execute the query",
            ));
        }
    };
    get_response_data(response, backend_url)
}

pub fn parse_from_rfc3339(rfc3339: &str) -> Result<SystemTime> {
    let datetime = chrono::DateTime::parse_from_rfc3339(rfc3339).map_to_runtime_error(
        GraphQlRuntimeErrorCode::CorruptData,
        "Failed to parse rfc3339 timestamp",
    )?;
    Ok(SystemTime::from(datetime))
}

fn is_502_status(status: Option<StatusCode>) -> bool {
    match status {
        None => false,
        Some(status) => status == StatusCode::BAD_GATEWAY,
    }
}

fn get_response_data<Data>(response: Response<Data>, backend_url: &str) -> Result<Data> {
    if let Some(errors) = response.errors {
        let error = errors
            .get(0)
            .ok_or_permanent_failure("Unexpected backend response: errors empty")?;
        let code = error
            .extensions
            .as_ref()
            .ok_or_permanent_failure("Unexpected backend response: error without extensions")?
            .get("code")
            .ok_or_permanent_failure("Unexpected backend response: error without code")?
            .as_str()
            .ok_or_permanent_failure("Unexpected backend response: error code isn't string")?;

        Err(map_error_code(code))
    } else {
        response.data.ok_or_permanent_failure(format!(
            "Response has no data. Verify URL is a GraphQL endpoint: {backend_url}"
        ))
    }
}

fn map_error_code(code: &str) -> Error {
    const AUTH_EXCEPTION_CODE: &str = "authentication-exception";
    const INVALID_JWT_ERROR_CODE: &str = "invalid-jwt";
    const MISSING_HTTP_HEADER_EXCEPTION_CODE: &str = "http-header-missing-exception";
    const INVALID_INVITATION_EXCEPTION_CODE: &str = "invalid-invitation-exception";
    const REMOTE_SCHEMA_ERROR_CODE: &str = "remote-schema-error";

    match code {
        AUTH_EXCEPTION_CODE => runtime_error(
            GraphQlRuntimeErrorCode::AuthServiceError,
            "The backend threw an Authentication Exception",
        ),
        INVALID_JWT_ERROR_CODE => runtime_error(
            GraphQlRuntimeErrorCode::AuthServiceError,
            "A request we made included an invalid JWT",
        ),
        MISSING_HTTP_HEADER_EXCEPTION_CODE => {
            permanent_failure("A request we made didn't include the necessary HTTP header")
        }
        INVALID_INVITATION_EXCEPTION_CODE => permanent_failure(
            "Unexpected backend response: invalid invitation when no invitations have been made",
        ),
        REMOTE_SCHEMA_ERROR_CODE => {
            permanent_failure("A remote schema call has failed on the backend")
        }
        _ => permanent_failure(format!(
            "Unexpected backend response: unknown error code: {code}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::parse_from_rfc3339;

    #[test]
    fn test_parse_from_rfc3339() {
        let date = parse_from_rfc3339("2023-09-21T16:39:21.919+00:00").unwrap();
        let timestamp = date
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(timestamp, 1695314361);

        let date = parse_from_rfc3339("2023-09-21T16:39:21.919+01:00").unwrap();
        let timestamp = date
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(timestamp, 1695314361 - 3600);

        let date = parse_from_rfc3339("2023-09-21T16:39:21.919-02:00").unwrap();
        let timestamp = date
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(timestamp, 1695314361 + 2 * 3600);
    }
}

pub trait ToRfc3339 {
    fn to_rfc3339(&self) -> String;
}

impl ToRfc3339 for SystemTime {
    fn to_rfc3339(&self) -> String {
        let datetime: DateTime<Utc> = DateTime::from(*self);
        datetime.to_rfc3339()
    }
}
