use graphql::perro::OptionToError;
use graphql::schema::{
    assign_lightning_address, disable_lightning_addresses, enable_lightning_addresses,
    request_phone_number_verification, submit_lnurl_pay_invoice, verified_phone_number,
    verify_phone_number, AssignLightningAddress, DisableLightningAddresses,
    EnableLightningAddresses, RequestPhoneNumberVerification, SubmitLnurlPayInvoice,
    VerifiedPhoneNumber, VerifyPhoneNumber,
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
    invoice: Option<String>,
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

pub async fn request_phone_number_verification(
    backend_url: &str,
    auth: &Auth,
    number: String,
    encrypted_number: String,
) -> graphql::Result<()> {
    let token = auth.query_token().await?;
    let client = build_async_client(Some(&token))?;
    let _data = post::<RequestPhoneNumberVerification>(
        &client,
        backend_url,
        request_phone_number_verification::Variables {
            encrypted_number,
            number,
        },
    )
    .await?;
    Ok(())
}

pub async fn verify_phone_number(
    backend_url: &str,
    auth: &Auth,
    number: String,
    otp: String,
) -> graphql::Result<()> {
    let token = auth.query_token().await?;
    let client = build_async_client(Some(&token))?;
    let _data = post::<VerifyPhoneNumber>(
        &client,
        backend_url,
        verify_phone_number::Variables { number, otp },
    )
    .await?;
    Ok(())
}

pub async fn query_verified_phone_number(
    backend_url: &str,
    auth: &Auth,
) -> graphql::Result<Option<String>> {
    let token = auth.query_token().await?;
    let client = build_async_client(Some(&token))?;
    let data =
        post::<VerifiedPhoneNumber>(&client, backend_url, verified_phone_number::Variables {})
            .await?;
    Ok(data.verified_phone_number.map(|n| n.encrypted_phone_number))
}

pub async fn disable_lightning_addresses(
    backend_url: &str,
    auth: &Auth,
    addresses: Vec<String>,
) -> graphql::Result<()> {
    let token = auth.query_token().await?;
    let client = build_async_client(Some(&token))?;
    post::<DisableLightningAddresses>(
        &client,
        backend_url,
        disable_lightning_addresses::Variables { addresses },
    )
    .await?;
    Ok(())
}

pub async fn enable_lightning_addresses(
    backend_url: &str,
    auth: &Auth,
    addresses: Vec<String>,
) -> graphql::Result<()> {
    let token = auth.query_token().await?;
    let client = build_async_client(Some(&token))?;
    post::<EnableLightningAddresses>(
        &client,
        backend_url,
        enable_lightning_addresses::Variables { addresses },
    )
    .await?;
    Ok(())
}
