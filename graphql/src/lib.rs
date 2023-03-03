pub mod errors;
pub mod schema;

pub use crate::errors::*;

pub use perro;
pub use reqwest;

use graphql_client::reqwest::post_graphql_blocking;
use graphql_client::Response;
use perro::{permanent_failure, runtime_error, MapToError, OptionToError};
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use std::time::Duration;

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

pub fn post_blocking<Query: graphql_client::GraphQLQuery>(
    client: &Client,
    backend_url: &String,
    variables: Query::Variables,
) -> Result<Query::ResponseData> {
    let response = post_graphql_blocking::<Query, _>(client, backend_url, variables)
        .map_to_runtime_error(
            GraphQlRuntimeErrorCode::NetworkError,
            "Failed to excute the query",
        )?;
    get_response_data(response, backend_url)
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
