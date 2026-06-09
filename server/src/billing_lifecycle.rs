//! Guarded lifecycle for billable compute calls.
//!
//! A guard reserves balance before compute starts and releases the hold on any
//! early return unless the call is explicitly marked as settled.

use std::sync::Arc;

use uuid::Uuid;

use crate::{store::Store, types::AppState};

pub(crate) struct TrustedBillingCall<'a> {
    store: &'a Store,
    user_id: String,
    compute_call_id: String,
    active: bool,
}

impl<'a> TrustedBillingCall<'a> {
    pub(crate) fn reserve(
        store: &'a Store,
        user_id: &str,
        compute_call_id: impl Into<String>,
        feature: &str,
        usage_mode: &str,
        model: Option<&str>,
        estimated_cost_fen: i64,
    ) -> Result<Self, String> {
        let compute_call_id = compute_call_id.into();
        crate::billing::reserve_trusted_call(
            store,
            user_id,
            &compute_call_id,
            feature,
            usage_mode,
            model,
            estimated_cost_fen,
        )?;
        Ok(Self {
            store,
            user_id: user_id.to_string(),
            compute_call_id,
            active: true,
        })
    }

    pub(crate) fn key(&self) -> &str {
        &self.compute_call_id
    }

    pub(crate) fn mark_settled(&mut self) {
        self.active = false;
    }

    pub(crate) fn release_no_usage(&mut self) {
        self.release("released_no_usage");
    }

    pub(crate) fn release_error(&mut self) {
        self.release("released_error");
    }

    fn release(&mut self, status: &str) {
        if !self.active {
            return;
        }
        crate::billing::release_trusted_call(
            self.store,
            &self.user_id,
            &self.compute_call_id,
            status,
        );
        self.active = false;
    }
}

impl Drop for TrustedBillingCall<'_> {
    fn drop(&mut self) {
        self.release_error();
    }
}

pub(crate) fn new_compute_call_id(prefix: &str) -> String {
    let cleaned: String = prefix
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-' || *ch == ':')
        .take(64)
        .collect();
    let prefix = if cleaned.is_empty() {
        "compute"
    } else {
        cleaned.as_str()
    };
    format!("{prefix}:{}", Uuid::new_v4())
}

pub(crate) fn spawn_reservation_janitor(state: Arc<AppState>) {
    let interval_secs = std::env::var("BILLING_RESERVATION_JANITOR_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value >= 10)
        .unwrap_or(60);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            match state.store.release_expired_billing_reservations() {
                Ok(0) => {}
                Ok(n) => tracing::info!(
                    released = n,
                    "billing reservation janitor released expired holds"
                ),
                Err(error) => tracing::warn!(%error, "billing reservation janitor failed"),
            }
        }
    });
}
