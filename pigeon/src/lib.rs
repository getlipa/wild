use graphql::{build_async_client, post};
use honey_badger::asynchronous::Auth;

pub async fn assign_lightning_address(_backend_url: &str, auth: &Auth) -> graphql::Result<String> {
    let token = auth.query_token().await?;
    let _client = build_async_client(Some(&token))?;
    //    post::<ReportPaymentTelemetry>(&client, backend_url, graphql_client::GraphQLQuery::Variables {}).await?;
    Ok("satoshi@lipa.swiss".to_string())
}
