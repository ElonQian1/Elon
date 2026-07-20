use super::{
    configured_auto_shared_provider, freeze_auth_switch_cloud_control,
    ordered_auto_shared_providers, restore_first_available_shared_provider,
    select_auto_shared_provider, AutoSharedProvider, CodexAuthAttemptState, SharingGrantSummary,
};
use std::sync::{Arc, Mutex};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn auth_switch_accepts_complete_owner_vault_cloud_window() {
    let issued_at = chrono::Utc::now();
    let deadline = issued_at + chrono::Duration::seconds(60);
    let frozen = freeze_auth_switch_cloud_control(
        Some(&deadline.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)),
        Some(&issued_at.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)),
        Some(60_000),
        None,
    );

    assert!(frozen.is_ok());
}

#[test]
fn auth_switch_rejects_owner_vault_window_with_missing_signed_fields() {
    let deadline = chrono::Utc::now() + chrono::Duration::seconds(60);
    let error = freeze_auth_switch_cloud_control(Some(&deadline.to_rfc3339()), None, None, None)
        .unwrap_err();

    assert!(error.to_string().contains("缺少服务器签发时间"));
}

#[test]
fn auto_shared_provider_selects_incoming_active_available_grant() {
    let grants = vec![
        grant("provider-a", "consumer-other", true, "active"),
        grant("provider-b", "consumer-1", true, "active")
            .with_account("15160532860")
            .with_nickname("全嘉"),
    ];

    let selected = select_auto_shared_provider(&grants, "consumer-1");

    assert_eq!(
        selected,
        Some(AutoSharedProvider {
            provider_user_id: Some("provider-b".to_string()),
            provider_account: Some("15160532860".to_string()),
            label: "全嘉".to_string(),
        })
    );
}

#[test]
fn auto_shared_provider_skips_unavailable_and_revoked_grants() {
    let grants = vec![
        grant("provider-a", "consumer-1", false, "active"),
        grant("provider-b", "consumer-1", true, "revoked"),
        grant("provider-c", "consumer-1", true, "active"),
    ];

    let selected = select_auto_shared_provider(&grants, "consumer-1");

    assert_eq!(
        selected,
        Some(AutoSharedProvider {
            provider_user_id: Some("provider-c".to_string()),
            provider_account: None,
            label: "provider-c".to_string(),
        })
    );
}

#[test]
fn shared_provider_candidates_keep_stable_order_deduplicate_and_include_every_active_grant() {
    let configured = AutoSharedProvider {
        provider_user_id: None,
        provider_account: Some("provider-a@example.com".to_string()),
        label: "configured-a".to_string(),
    };
    let mut grants = vec![
        grant("provider-a", "consumer-1", true, "active").with_account("PROVIDER-A@example.com"),
        grant("provider-b", "consumer-1", true, "active"),
    ];
    for index in 0..12 {
        grants.push(grant(
            &format!("provider-extra-{index}"),
            "consumer-1",
            true,
            "active",
        ));
    }

    let selected = ordered_auto_shared_providers(Some(configured), &grants, "consumer-1");

    assert_eq!(selected.len(), 14);
    assert!(
        selected.len() > 8,
        "all candidates must survive beyond the old cap"
    );
    assert_eq!(selected[0].label, "configured-a");
    assert_eq!(selected[1].label, "provider-b");
    assert_eq!(selected[2].label, "provider-extra-0");
    assert_eq!(selected[13].label, "provider-extra-11");
}

#[test]
fn shared_provider_snapshot_is_frozen_after_first_discovery() {
    let mut attempts = CodexAuthAttemptState::new(true);
    let first = attempts
        .freeze_shared_provider_snapshot(vec![provider("provider-a"), provider("provider-b")]);
    let second = attempts.freeze_shared_provider_snapshot(vec![
        provider("provider-a"),
        provider("provider-b"),
        provider("provider-added-later"),
    ]);

    assert_eq!(first, second);
    assert_eq!(second.len(), 2);
}

#[tokio::test]
async fn own_then_night_cloud_quota_continues_with_quanjia() {
    let providers = vec![provider("night-cloud"), provider("quanjia")];
    let mut attempts = CodexAuthAttemptState::new(true);
    assert!(attempts.reserve_owner_vault_attempt());

    let first = restore_first_available_shared_provider(
        "req-sequence",
        &mut attempts,
        providers.clone(),
        |provider| async move { Ok::<_, anyhow::Error>(provider.label) },
    )
    .await
    .unwrap();
    assert_eq!(first.0.label, "night-cloud");

    let second = restore_first_available_shared_provider(
        "req-sequence",
        &mut attempts,
        providers,
        |provider| async move { Ok::<_, anyhow::Error>(provider.label) },
    )
    .await
    .unwrap();
    assert_eq!(second.0.label, "quanjia");
    assert_eq!(attempts.attempt_count(), 3);
}

#[tokio::test]
async fn shared_provider_restore_failure_skips_to_next_candidate() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let calls_for_restore = Arc::clone(&calls);
    let mut attempts = CodexAuthAttemptState::new(true);

    let restored = restore_first_available_shared_provider(
        "req-restore-failure",
        &mut attempts,
        vec![provider("provider-a"), provider("provider-b")],
        move |provider| {
            let calls = Arc::clone(&calls_for_restore);
            async move {
                calls
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(provider.label.clone());
                if provider.label == "provider-a" {
                    anyhow::bail!("provider A restore failed");
                }
                Ok(provider.label)
            }
        },
    )
    .await
    .unwrap();

    assert_eq!(restored.0.label, "provider-b");
    assert_eq!(
        *calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        vec!["provider-a".to_string(), "provider-b".to_string()]
    );
}

#[tokio::test]
async fn exhausted_shared_providers_are_not_retried() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let providers = vec![provider("provider-a"), provider("provider-b")];
    let mut attempts = CodexAuthAttemptState::new(true);
    assert!(attempts.reserve_owner_vault_attempt());

    for run in 0..2 {
        let calls_for_restore = Arc::clone(&calls);
        let restored = restore_first_available_shared_provider(
            "req-exhausted",
            &mut attempts,
            providers.clone(),
            move |provider| {
                let calls = Arc::clone(&calls_for_restore);
                async move {
                    calls
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(provider.label);
                    Err::<String, _>(anyhow::anyhow!("restore failed"))
                }
            },
        )
        .await;
        assert!(
            restored.is_none(),
            "run {run} unexpectedly restored a provider"
        );
    }

    assert_eq!(attempts.attempt_count(), 3);
    assert_eq!(
        calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        2
    );
}

#[tokio::test]
async fn snapshot_with_more_than_eight_providers_attempts_every_candidate_once() {
    let providers = (0..12)
        .map(|index| provider(&format!("provider-{index}")))
        .collect::<Vec<_>>();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let calls_for_restore = Arc::clone(&calls);
    let mut attempts = CodexAuthAttemptState::new(true);
    let frozen = attempts.freeze_shared_provider_snapshot(providers);

    let restored = restore_first_available_shared_provider(
        "req-more-than-eight",
        &mut attempts,
        frozen,
        move |provider| {
            let calls = Arc::clone(&calls_for_restore);
            async move {
                calls
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(provider.label);
                Err::<String, _>(anyhow::anyhow!("quota exhausted"))
            }
        },
    )
    .await;

    assert!(restored.is_none());
    let calls = calls
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(calls.len(), 12);
    for index in 0..12 {
        assert_eq!(
            calls
                .iter()
                .filter(|label| label.as_str() == format!("provider-{index}"))
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn offline_state_never_attempts_owner_or_shared_auth() {
    let mut attempts = CodexAuthAttemptState::new(false);
    assert!(!attempts.reserve_owner_vault_attempt());
    let restored = restore_first_available_shared_provider(
        "req-offline",
        &mut attempts,
        vec![provider("provider-a")],
        |_| async move { Ok::<_, anyhow::Error>("unexpected") },
    )
    .await;

    assert!(restored.is_none());
    assert_eq!(attempts.attempt_count(), 0);
}

#[test]
fn configured_auto_shared_provider_takes_precedence() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let old_user_id = std::env::var("ELON_CODEX_AUTO_SHARED_PROVIDER_USER_ID").ok();
    let old_account = std::env::var("ELON_CODEX_AUTO_SHARED_PROVIDER_ACCOUNT").ok();
    let old_generic = std::env::var("ELON_CODEX_AUTO_SHARED_PROVIDER").ok();
    let old_node_generic = std::env::var("NODE_CODEX_AUTO_SHARED_PROVIDER").ok();
    std::env::set_var("ELON_CODEX_AUTO_SHARED_PROVIDER_USER_ID", "usr_quanjia");
    std::env::set_var("ELON_CODEX_AUTO_SHARED_PROVIDER_ACCOUNT", "15160532860");
    std::env::remove_var("ELON_CODEX_AUTO_SHARED_PROVIDER");
    std::env::remove_var("NODE_CODEX_AUTO_SHARED_PROVIDER");

    let selected = configured_auto_shared_provider();

    restore_env("ELON_CODEX_AUTO_SHARED_PROVIDER_USER_ID", old_user_id);
    restore_env("ELON_CODEX_AUTO_SHARED_PROVIDER_ACCOUNT", old_account);
    restore_env("ELON_CODEX_AUTO_SHARED_PROVIDER", old_generic);
    restore_env("NODE_CODEX_AUTO_SHARED_PROVIDER", old_node_generic);

    assert_eq!(
        selected,
        Some(AutoSharedProvider {
            provider_user_id: Some("usr_quanjia".to_string()),
            provider_account: Some("15160532860".to_string()),
            label: "15160532860".to_string(),
        })
    );
}

fn grant(
    provider_user_id: &str,
    consumer_user_id: &str,
    provider_vault_available: bool,
    status: &str,
) -> SharingGrantSummary {
    SharingGrantSummary {
        provider_user_id: Some(provider_user_id.to_string()),
        consumer_user_id: Some(consumer_user_id.to_string()),
        provider_vault_available: Some(provider_vault_available),
        status: Some(status.to_string()),
        ..SharingGrantSummary::default()
    }
}

fn provider(provider_user_id: &str) -> AutoSharedProvider {
    AutoSharedProvider {
        provider_user_id: Some(provider_user_id.to_string()),
        provider_account: None,
        label: provider_user_id.to_string(),
    }
}

trait GrantTestExt {
    fn with_account(self, account: &str) -> Self;
    fn with_nickname(self, nickname: &str) -> Self;
}

impl GrantTestExt for SharingGrantSummary {
    fn with_account(mut self, account: &str) -> Self {
        self.provider_account = Some(account.to_string());
        self
    }

    fn with_nickname(mut self, nickname: &str) -> Self {
        self.provider_nickname = Some(nickname.to_string());
        self
    }
}

fn restore_env(name: &str, value: Option<String>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}
