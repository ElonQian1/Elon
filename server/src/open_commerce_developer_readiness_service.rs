//! Derives developer-App production readiness from existing authoritative records.

use anyhow::Result;
use chrono::Utc;

use crate::{
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_developer_readiness_model::{
        DeveloperProductionReadinessStep, DeveloperProductionReadinessSummary,
    },
    open_commerce_webhook_health_service,
    store::Store,
};

pub(crate) fn readiness_summary(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
) -> Result<DeveloperProductionReadinessSummary> {
    let admission = store.open_commerce_developer_app_admission(&app.id)?;
    let webhook_health = open_commerce_webhook_health_service::health_summary(store, app)?;
    let credential_gateway_ready = webhook_health.production_credentials_enabled;
    let current_credential_present = webhook_health.production_credential_eligible;
    let webhook_gateway_ready = webhook_health.production_webhooks_enabled;
    let active_production_webhook_count = webhook_health
        .environments
        .iter()
        .find(|item| item.environment == "production")
        .map(|item| item.active_subscription_count)
        .unwrap_or_default();

    let app_ready = app.status == "active";
    let manifest_ready = app_ready && app.manifest_status == "approved";
    let domain_ready = app_ready
        && app.domain_verification_status == "verified"
        && app.domain_verification_revision == Some(app.manifest_revision);
    let admission_ready = admission.as_ref().is_some_and(|value| {
        value.status == "approved" && value.manifest_revision == app.manifest_revision
    });
    let steps = vec![
        step("app", app_ready, "app_inactive"),
        step("manifest", manifest_ready, "manifest_not_approved"),
        step(
            "domain",
            domain_ready,
            "domain_not_verified_for_current_revision",
        ),
        step(
            "admission",
            admission_ready,
            "admission_not_approved_for_current_revision",
        ),
        step(
            "credential_gateway",
            credential_gateway_ready,
            "production_credentials_disabled",
        ),
        step(
            "credential",
            current_credential_present,
            "current_production_credential_missing",
        ),
        step(
            "webhook_gateway",
            webhook_gateway_ready,
            "production_webhooks_disabled",
        ),
        step(
            "webhook",
            active_production_webhook_count > 0,
            "active_production_webhook_missing",
        ),
    ];
    let blocker_codes = steps
        .iter()
        .filter_map(|item| item.blocker_code)
        .collect::<Vec<_>>();
    let production_invocation_ready = app_ready
        && manifest_ready
        && domain_ready
        && admission_ready
        && credential_gateway_ready
        && current_credential_present;
    let production_webhook_ready =
        production_invocation_ready && webhook_gateway_ready && active_production_webhook_count > 0;

    Ok(DeveloperProductionReadinessSummary {
        schema: "open_commerce.developer_production_readiness.v1",
        app_record_id: app.id.clone(),
        app_id: app.app_id.clone(),
        manifest_revision: app.manifest_revision,
        admission_status: admission.as_ref().map(|value| value.status.clone()),
        admission_revision: admission.as_ref().map(|value| value.manifest_revision),
        production_credentials_enabled: credential_gateway_ready,
        current_production_credential_present: current_credential_present,
        production_webhooks_enabled: webhook_gateway_ready,
        active_production_webhook_count,
        production_invocation_ready,
        production_webhook_ready,
        next_action_code: blocker_codes.first().copied(),
        blocker_codes,
        steps,
        generated_at: Utc::now().to_rfc3339(),
    })
}

fn step(
    code: &'static str,
    ready: bool,
    blocker_code: &'static str,
) -> DeveloperProductionReadinessStep {
    DeveloperProductionReadinessStep {
        code,
        ready,
        blocker_code: (!ready).then_some(blocker_code),
    }
}
