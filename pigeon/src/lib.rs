use graphql::perro::OptionToError;
use graphql::schema::{
    assign_lightning_address, submit_lnurl_pay_invoice, AssignLightningAddress,
    SubmitLnurlPayInvoice,
};
use graphql::{build_async_client, post};
use honeybadger::asynchronous::Auth;

pub async fn assign_lightning_address(backend_url: &str, auth: &Auth) -> graphql::Result<String> {
    let token = auth.query_token().await?;
    let client = build_async_client(Some(&token))?;
    let data = post::<AssignLightningAddress>(
        &client,
        backend_url,
        assign_lightning_address::Variables {},
    )
    .await?;
    let address = data
        .assign_lightning_address
        .ok_or_permanent_failure("Unexpected backend response: empty")?
        .address;
    Ok(address)
}

pub async fn submit_lnurl_pay_invoice(
    backend_url: &str,
    auth: &Auth,
    id: String,
    invoice: String,
) -> graphql::Result<()> {
    let token = auth.query_token().await?;
    let client = build_async_client(Some(&token))?;
    let _data = post::<SubmitLnurlPayInvoice>(
        &client,
        backend_url,
        submit_lnurl_pay_invoice::Variables { id, invoice },
    )
    .await?;
    Ok(())
}
