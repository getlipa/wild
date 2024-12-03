use graphql_client::GraphQLQuery;

type DateTime = String;
#[allow(non_camel_case_types)]
type numeric = float8;
#[allow(non_camel_case_types)]
type timestamptz = String;
#[allow(non_camel_case_types)]
type uuid = String;
type Void = ();

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct RequestChallenge;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct StartSession;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct PrepareWalletSession;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct UnlockWallet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct RefreshSession;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct AcceptTermsAndConditionsV2;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct GetTermsAndConditionsStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct RegisterTopup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct HideTopup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct RegisterNotificationToken;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct GetBusinessOwner;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct GetExchangeRate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct GetAllExchangeRates;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct ListCurrencyCodes;

#[allow(non_camel_case_types)]
type bigint = u64;
type BigInteger = bigint;
#[allow(non_camel_case_types)]
type float8 = f64;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct ListUncompletedTopups;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct MigrationBalance;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct MigrateFunds;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct CreateBackup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct RecoverBackup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct ReportPaymentTelemetry;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct AssignLightningAddress;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct SubmitLnurlPayInvoice;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct RequestPhoneNumberVerification;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct VerifyPhoneNumber;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct VerifiedPhoneNumber;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct DisableLightningAddresses;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct EnableLightningAddresses;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct StartTopupSetup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/schema_wallet_read.graphql",
    query_path = "schemas/operations.graphql",
    response_derives = "Debug"
)]
pub struct CompleteTopupSetup;
