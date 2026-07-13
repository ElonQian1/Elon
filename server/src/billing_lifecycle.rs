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
    dispatch_held: bool,
    deferred_to_replay: bool,
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
            dispatch_held: false,
            deferred_to_replay: false,
        })
    }

    pub(crate) fn no_platform_charge(
        store: &'a Store,
        user_id: &str,
        compute_call_id: impl Into<String>,
    ) -> Self {
        Self {
            store,
            user_id: user_id.to_string(),
            compute_call_id: compute_call_id.into(),
            active: false,
            dispatch_held: false,
            deferred_to_replay: false,
        }
    }

    pub(crate) fn key(&self) -> &str {
        &self.compute_call_id
    }

    pub(crate) fn mark_settled(&mut self) {
        self.active = false;
        self.dispatch_held = false;
        self.deferred_to_replay = false;
    }

    /// Persist the crash-safety boundary before a prompt can reach a node.
    /// Returns false for non-billable calls that have no balance reservation.
    pub(crate) fn hold_for_dispatch(&mut self) -> Result<bool, String> {
        let held = self
            .store
            .hold_billing_reservation_for_dispatch(&self.user_id, &self.compute_call_id)
            .map_err(|error| format!("持久化派发计费预留失败: {error}"))?
            .is_some();
        self.dispatch_held = held;
        Ok(held)
    }

    /// Dedicated refund path for a dispatch failure that proves the prompt was
    /// never enqueued to the node writer.
    pub(crate) fn release_dispatch_not_sent(&mut self) {
        if !self.dispatch_held {
            self.release_error();
            return;
        }
        if let Err(error) = self
            .store
            .release_dispatch_billing_hold_before_send(&self.user_id, &self.compute_call_id)
        {
            tracing::warn!(
                user_id = %self.user_id,
                compute_call_id = %self.compute_call_id,
                %error,
                "verified pre-send billing hold release failed"
            );
        }
        self.active = false;
        self.dispatch_held = false;
        self.deferred_to_replay = false;
    }

    /// Transfer responsibility for an already-dispatched PC completion to its
    /// durable replay outbox without releasing its dispatch hold.
    pub(crate) fn handoff_to_durable_replay(&mut self) {
        self.active = false;
        self.deferred_to_replay = true;
    }

    pub(crate) fn release_no_usage(&mut self) {
        self.release("released_no_usage");
    }

    pub(crate) fn release_error(&mut self) {
        self.release("released_error");
    }

    fn release(&mut self, status: &str) {
        if self.dispatch_held || self.deferred_to_replay {
            return;
        }
        // The PC node may have started as own-Codex (no hold) and then asked
        // the cloud to atomically upgrade this same compute_call_id to a
        // shared-Codex lease. Always attempt the idempotent release so an
        // externally-created mid-run hold cannot remain stranded on timeout.
        if !self.active
            && !matches!(
                self.store
                    .get_active_billing_reservation(&self.user_id, &self.compute_call_id),
                Ok(Some(_))
            )
        {
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

#[cfg(test)]
mod tests {
    use super::TrustedBillingCall;
    use crate::store::Store;

    #[test]
    fn durable_replay_handoff_keeps_dispatch_hold_open() {
        let path = std::env::temp_dir().join(format!(
            "elon-billing-handoff-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).unwrap();
        let user = store
            .create_user(
                &format!("handoff-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        store
            .billing_recharge(&user.id, 1_000, "test", "test", None)
            .unwrap();
        let key = "pc_agent_cli:test-unknown-usage-handoff";

        let mut call = TrustedBillingCall::reserve(
            &store,
            &user.id,
            key,
            "pc_agent_cli_chat",
            "pc_agent_cli",
            Some("codex"),
            100,
        )
        .unwrap();
        assert!(call.hold_for_dispatch().unwrap());
        call.handoff_to_durable_replay();
        drop(call);

        assert!(store
            .billing_reservation_is_still_reserved(&user.id, key)
            .unwrap());
        assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(900));
        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
