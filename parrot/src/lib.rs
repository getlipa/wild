use graphql::schema::report_payment_telemetry::{
    PayFailedInput, PayInitiatedInput, PaySource, PaySucceededInput, PaymentTelemetryEventsInput,
    RequestInitiatedInput, RequestSucceededInput,
};
use graphql::schema::{report_payment_telemetry, ReportPaymentTelemetry};
use graphql::{build_client, post_blocking, ToRfc3339};
use honey_badger::Auth;
use std::sync::Arc;
use std::time::SystemTime;

pub enum PaymentSource {
    Camera,
    Clipboard,
    Nfc,
    Manual,
}
pub enum PayFailureReason {
    NoRoute,
    Unkown,
}

pub enum AnalyticsEvent {
    PayInitiated {
        payment_hash: String,

        paid_amount_msat: u64,
        requested_amount_msat: Option<u64>,
        sats_per_user_currency: Option<u32>,

        source: PaymentSource,
        user_currency: String,

        process_started_at: SystemTime,
        executed_at: SystemTime,
    },
    PaySucceeded {
        payment_hash: String,

        ln_fees_paid_msat: u64,
        confirmed_at: SystemTime,
    },
    PayFailed {
        payment_hash: String,

        reason: PayFailureReason,
        failed_at: SystemTime,
    },
    RequestInitiated {
        payment_hash: String,

        entered_amount_msat: Option<u64>,
        sats_per_user_currency: Option<u32>,

        user_currency: String,
        request_currency: String,

        created_at: SystemTime,
    },
    RequestSucceeded {
        payment_hash: String,

        paid_amount_sat: u64,
        channel_opening_fee_msat: u64,

        received_at: SystemTime,
    },
}

pub struct AnalyticsClient {
    backend_url: String,
    analytics_id: String,
    auth: Arc<Auth>,
}

impl AnalyticsClient {
    pub fn new(backend_url: String, analytics_id: String, auth: Arc<Auth>) -> Self {
        Self {
            backend_url,
            analytics_id,
            auth,
        }
    }

    pub fn report_event(&self, analytics_event: AnalyticsEvent) -> graphql::Result<()> {
        let variables = match analytics_event {
            AnalyticsEvent::PayInitiated {
                payment_hash,
                paid_amount_msat,
                requested_amount_msat,
                sats_per_user_currency,
                source,
                user_currency,
                process_started_at,
                executed_at,
            } => report_payment_telemetry::Variables {
                telemetry_id: self.analytics_id.clone(),
                events: Some(PaymentTelemetryEventsInput {
                    pay_failed: None,
                    pay_initiated: Some(PayInitiatedInput {
                        process_started_at: process_started_at.to_rfc3339(),
                        executed_at: executed_at.to_rfc3339(),
                        paid_amount_m_sat: paid_amount_msat,
                        payment_hash,
                        requested_amount_m_sat: requested_amount_msat.unwrap_or(0),
                        sats_per_user_currency: sats_per_user_currency.unwrap_or(0) as i64,
                        source: map_payment_source(source),
                        user_currency,
                    }),
                    pay_succeeded: None,
                    request_initiated: None,
                    request_succeeded: None,
                }),
            },
            AnalyticsEvent::PaySucceeded {
                payment_hash,
                ln_fees_paid_msat,
                confirmed_at,
            } => report_payment_telemetry::Variables {
                telemetry_id: self.analytics_id.clone(),
                events: Some(PaymentTelemetryEventsInput {
                    pay_failed: None,
                    pay_initiated: None,
                    pay_succeeded: Some(PaySucceededInput {
                        confirmed_at: confirmed_at.to_rfc3339(),
                        ln_fees_paid_m_sat: ln_fees_paid_msat,
                        payment_hash,
                    }),
                    request_initiated: None,
                    request_succeeded: None,
                }),
            },
            AnalyticsEvent::PayFailed {
                payment_hash,
                reason,
                failed_at,
            } => report_payment_telemetry::Variables {
                telemetry_id: self.analytics_id.clone(),
                events: Some(PaymentTelemetryEventsInput {
                    pay_failed: Some(PayFailedInput {
                        failed_at: failed_at.to_rfc3339(),
                        payment_hash,
                        reason: map_pay_failure_reason(reason),
                    }),
                    pay_initiated: None,
                    pay_succeeded: None,
                    request_initiated: None,
                    request_succeeded: None,
                }),
            },
            AnalyticsEvent::RequestInitiated {
                payment_hash,
                entered_amount_msat,
                sats_per_user_currency,
                user_currency,
                request_currency,
                created_at,
            } => report_payment_telemetry::Variables {
                telemetry_id: self.analytics_id.clone(),
                events: Some(PaymentTelemetryEventsInput {
                    pay_failed: None,
                    pay_initiated: None,
                    pay_succeeded: None,
                    request_initiated: Some(RequestInitiatedInput {
                        created_at: created_at.to_rfc3339(),
                        entered_amount_m_sat: entered_amount_msat.unwrap_or(0),
                        payment_hash,
                        request_currency,
                        sats_per_user_currency: sats_per_user_currency.unwrap_or(0) as i64,
                        user_currency,
                    }),
                    request_succeeded: None,
                }),
            },
            AnalyticsEvent::RequestSucceeded {
                payment_hash,
                paid_amount_sat,
                channel_opening_fee_msat,
                received_at,
            } => report_payment_telemetry::Variables {
                telemetry_id: self.analytics_id.clone(),
                events: Some(PaymentTelemetryEventsInput {
                    pay_failed: None,
                    pay_initiated: None,
                    pay_succeeded: None,
                    request_initiated: None,
                    request_succeeded: Some(RequestSucceededInput {
                        channel_opening_fee_m_sat: channel_opening_fee_msat,
                        paid_amount_m_sat: paid_amount_sat,
                        payment_hash,
                        payment_received_at: received_at.to_rfc3339(),
                    }),
                }),
            },
        };

        let token = self.auth.query_token()?;
        let client = build_client(Some(&token))?;
        post_blocking::<ReportPaymentTelemetry>(&client, &self.backend_url, variables)?;

        Ok(())
    }
}

fn map_payment_source(payment_source: PaymentSource) -> PaySource {
    match payment_source {
        PaymentSource::Camera => PaySource::CAMERA,
        PaymentSource::Clipboard => PaySource::CLIPBOARD,
        PaymentSource::Nfc => PaySource::NFC,
        PaymentSource::Manual => PaySource::MANUAL,
    }
}

fn map_pay_failure_reason(
    pay_failure_reason: PayFailureReason,
) -> report_payment_telemetry::PayFailureReason {
    match pay_failure_reason {
        PayFailureReason::NoRoute => report_payment_telemetry::PayFailureReason::NO_ROUTE,
        PayFailureReason::Unkown => report_payment_telemetry::PayFailureReason::UNKNOWN,
    }
}
