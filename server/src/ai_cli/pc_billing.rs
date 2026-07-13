use anyhow::{anyhow, Context};

use crate::{
    billing_lifecycle::TrustedBillingCall,
    cli_usage::CliTokenUsage,
    node_router,
    store::token_usage::{
        BillingReservationConstraint, BillingReservationConstraintViolation,
        BILLING_SOURCE_OWN_CODEX, BILLING_SOURCE_PLATFORM, BILLING_SOURCE_SHARED_CODEX,
    },
    store::{
        NodeComputeReplayBinding, NodeTransaction, SettleParams, Store, TokenUsageAccountingResult,
    },
    token_usage_api,
    types::AppState,
};

use super::{pc_cli_price_per_1k_credits, pc_cli_usage_tokens};

#[derive(Debug, Clone)]
pub(super) struct PcCliBillingContext {
    pub(super) billing_source: String,
    pub(super) resource_owner_user_id: Option<String>,
    pub(super) lease_id: Option<String>,
    pub(super) replay_deadline: Option<String>,
    pub(super) charge_platform_balance: bool,
    pub(super) max_cost_rmb_fen: i64,
    pub(super) allowance_id: Option<String>,
    pub(super) frozen_reservation_required: bool,
}

impl PcCliBillingContext {
    pub(super) fn requires_cloud_control(&self) -> bool {
        self.billing_source != BILLING_SOURCE_OWN_CODEX || self.charge_platform_balance
    }

    pub(super) fn codex_credential_binding(
        &self,
        cli_name: &str,
    ) -> Option<homecli_proto::CliCodexCredentialBinding> {
        pc_cli_name_is_codex(cli_name).then(|| homecli_proto::CliCodexCredentialBinding {
            managed: self.lease_id.is_some(),
            lease_id: self.lease_id.clone(),
        })
    }
}

pub(crate) fn pc_cli_request_is_own_codex(
    state: &AppState,
    user_id: &str,
    node_id: &str,
    cli_name: Option<&str>,
) -> bool {
    let cli_name = cli_name.unwrap_or("codex");
    if !pc_cli_name_is_codex(cli_name) {
        return false;
    }
    match state.store.get_node_credential_owner(node_id) {
        Ok(Some(owner)) => owner == user_id,
        _ => false,
    }
}

pub(crate) fn requested_pc_cli_looks_like_codex(
    state: &AppState,
    requested_agent_name: Option<&str>,
) -> bool {
    let Some(name) = requested_agent_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return true;
    };
    if pc_cli_name_is_codex(name) {
        return true;
    }
    state
        .ai_cli
        .find_option(Some(name))
        .map(|option| {
            [
                option.id.as_str(),
                option.provider.as_str(),
                option.bin.as_str(),
            ]
            .iter()
            .any(|value| pc_cli_name_is_codex(value))
        })
        .unwrap_or(false)
}

pub(super) fn reserve_pc_cli_billing_call<'a>(
    state: &'a AppState,
    user_id: &str,
    node_id: &str,
    accounting_key: &str,
    feature: &str,
    model: Option<&str>,
    reserve_fen: i64,
    cli_name: &str,
) -> Result<(TrustedBillingCall<'a>, PcCliBillingContext), String> {
    let context = pc_cli_billing_context(state, user_id, node_id, cli_name)
        .map_err(|error| error.to_string())?;
    let call = if context.charge_platform_balance {
        TrustedBillingCall::reserve(
            &state.store,
            user_id,
            accounting_key,
            feature,
            "pc_agent_cli",
            model,
            reserve_fen,
        )?
    } else {
        TrustedBillingCall::no_platform_charge(&state.store, user_id, accounting_key)
    };
    Ok((call, context))
}

/// Freeze the dispatch-time resource owner and reservation. Replay must never
/// recalculate these facts from whichever Codex account happens to be active
/// after a reconnect.
pub(super) fn bind_pc_cli_replay_policy(
    state: &AppState,
    consumer_user_id: &str,
    accounting_key: &str,
    context: &PcCliBillingContext,
) -> anyhow::Result<Option<String>> {
    bind_pc_cli_replay_policy_in_store(&state.store, consumer_user_id, accounting_key, context)
}

fn bind_pc_cli_replay_policy_in_store(
    store: &Store,
    consumer_user_id: &str,
    accounting_key: &str,
    context: &PcCliBillingContext,
) -> anyhow::Result<Option<String>> {
    let reservation = if context.charge_platform_balance {
        store
            .get_active_billing_reservation(consumer_user_id, accounting_key)
            .with_context(|| format!("查询 PC CLI 预授权失败: {accounting_key}"))?
    } else {
        None
    };
    if context.charge_platform_balance && reservation.is_none() {
        return Err(BillingReservationConstraintViolation::MissingFrozenAllowance.into());
    }
    let replay_deadline = earliest_deadline(
        context.replay_deadline.as_deref(),
        reservation
            .as_ref()
            .and_then(|value| value.expires_at.as_deref()),
    );
    let offline_policy =
        if context.billing_source == BILLING_SOURCE_OWN_CODEX && !context.charge_platform_balance {
            "allow_offline"
        } else {
            "require_active_reservation"
        };
    let binding = NodeComputeReplayBinding {
        billing_source: &context.billing_source,
        resource_owner_user_id: context.resource_owner_user_id.as_deref(),
        lease_id: context.lease_id.as_deref(),
        offline_policy,
        replay_deadline: replay_deadline.as_deref(),
        max_cost_rmb_fen: reservation
            .as_ref()
            .map(|value| value.reserved_fen)
            .unwrap_or(0),
        allowance_id: reservation
            .as_ref()
            .map(|value| value.reservation_id.as_str()),
    };
    let bound = if context.lease_id.is_some() {
        store.bind_node_compute_run_to_active_emergency_lease(accounting_key, binding)?
    } else {
        store.bind_node_compute_run_replay_policy(accounting_key, binding)?
    }
    .context("PC CLI 计算运行不存在，不能冻结离线回放策略")?;
    if bound.billing_source != context.billing_source
        || bound.resource_owner_user_id != context.resource_owner_user_id
        || bound.lease_id != context.lease_id
        || bound.offline_policy != offline_policy
        || !same_deadline(bound.replay_deadline.as_deref(), replay_deadline.as_deref())
        || bound.max_cost_rmb_fen
            != reservation
                .as_ref()
                .map(|value| value.reserved_fen)
                .unwrap_or(0)
        || bound.allowance_id
            != reservation
                .as_ref()
                .map(|value| value.reservation_id.clone())
    {
        return Err(anyhow!("PC CLI 离线回放策略未按派发上下文完整冻结"));
    }
    Ok(bound.replay_deadline)
}

/// Reload only the server-frozen compute binding. This intentionally does not
/// inspect whichever `CODEX_HOME` happens to be active at completion time. The
/// shared-account lease endpoint is the sole authority allowed to upgrade an
/// own-Codex run after dispatch.
pub(super) fn refresh_pc_cli_billing_context_from_run(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    accounting_key: &str,
    current: &PcCliBillingContext,
) -> anyhow::Result<PcCliBillingContext> {
    refresh_pc_cli_billing_context_from_store(
        &state.store,
        consumer_user_id,
        node_id,
        accounting_key,
        current,
    )
}

fn refresh_pc_cli_billing_context_from_store(
    store: &Store,
    consumer_user_id: &str,
    node_id: &str,
    accounting_key: &str,
    _current: &PcCliBillingContext,
) -> anyhow::Result<PcCliBillingContext> {
    let run = store
        .get_node_compute_run_by_compute_call_id(accounting_key)?
        .with_context(|| format!("PC CLI 冻结计费上下文不存在: {accounting_key}"))?;
    if run.consumer_user_id != consumer_user_id || run.node_id != node_id {
        return Err(anyhow!("PC CLI 冻结计费上下文与用户或节点不匹配"));
    }
    Ok(match run.billing_source.as_str() {
        BILLING_SOURCE_OWN_CODEX => PcCliBillingContext {
            billing_source: BILLING_SOURCE_OWN_CODEX.to_string(),
            resource_owner_user_id: run.resource_owner_user_id,
            lease_id: None,
            replay_deadline: run.replay_deadline,
            charge_platform_balance: false,
            max_cost_rmb_fen: run.max_cost_rmb_fen,
            allowance_id: run.allowance_id,
            frozen_reservation_required: false,
        },
        BILLING_SOURCE_SHARED_CODEX => PcCliBillingContext {
            billing_source: BILLING_SOURCE_SHARED_CODEX.to_string(),
            resource_owner_user_id: run.resource_owner_user_id,
            lease_id: run.lease_id,
            replay_deadline: run.replay_deadline,
            charge_platform_balance: true,
            max_cost_rmb_fen: run.max_cost_rmb_fen,
            allowance_id: run.allowance_id,
            frozen_reservation_required: true,
        },
        BILLING_SOURCE_PLATFORM => PcCliBillingContext {
            billing_source: BILLING_SOURCE_PLATFORM.to_string(),
            resource_owner_user_id: run.resource_owner_user_id,
            lease_id: run.lease_id,
            replay_deadline: run.replay_deadline,
            charge_platform_balance: true,
            max_cost_rmb_fen: run.max_cost_rmb_fen,
            allowance_id: run.allowance_id,
            frozen_reservation_required: true,
        },
        source => {
            return Err(anyhow!("PC CLI 冻结计费来源无效: {source}"));
        }
    })
}

fn earliest_deadline(first: Option<&str>, second: Option<&str>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) => {
            let first_time = chrono::DateTime::parse_from_rfc3339(first).ok();
            let second_time = chrono::DateTime::parse_from_rfc3339(second).ok();
            match (first_time, second_time) {
                (Some(first_time), Some(second_time)) if first_time <= second_time => {
                    Some(first.to_string())
                }
                (Some(_), Some(_)) => Some(second.to_string()),
                _ => Some(first.min(second).to_string()),
            }
        }
        (Some(value), None) | (None, Some(value)) => Some(value.to_string()),
        (None, None) => None,
    }
}

fn same_deadline(first: Option<&str>, second: Option<&str>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(first), Some(second)) => {
            match (
                chrono::DateTime::parse_from_rfc3339(first),
                chrono::DateTime::parse_from_rfc3339(second),
            ) {
                (Ok(first), Ok(second)) => first == second,
                _ => first == second,
            }
        }
        _ => false,
    }
}

pub(super) fn record_pc_cli_trusted_usage_result(
    store: &Store,
    user_id: &str,
    feature: &str,
    frozen_model_id: &str,
    usage: &CliTokenUsage,
    accounting_key: &str,
    context: &PcCliBillingContext,
) -> anyhow::Result<Option<TokenUsageAccountingResult>> {
    let reservation_constraint = if context.charge_platform_balance
        && (context.frozen_reservation_required || context.allowance_id.is_some())
    {
        match context.allowance_id.as_deref() {
            Some(expected_reservation_id) => Some(BillingReservationConstraint {
                expected_reservation_id,
                max_cost_rmb_fen: context.max_cost_rmb_fen,
            }),
            None => {
                return Err(BillingReservationConstraintViolation::MissingFrozenAllowance.into())
            }
        }
    } else {
        None
    };
    token_usage_api::try_record_trusted_usage_with_key_and_resource(
        store,
        user_id,
        feature,
        "pc_agent_cli",
        Some(frozen_model_id),
        usage,
        Some(accounting_key),
        Some(context.billing_source.as_str()),
        context.resource_owner_user_id.as_deref(),
        context.charge_platform_balance,
        reservation_constraint,
    )
}

pub(super) fn settle_pc_cli_node_usage(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    feature: &str,
    frozen_model_id: &str,
    usage: &CliTokenUsage,
    accounting_result: Option<&TokenUsageAccountingResult>,
    context: &PcCliBillingContext,
) -> anyhow::Result<Option<NodeTransaction>> {
    let Some(accounting_result) = accounting_result else {
        return Ok(None);
    };
    let Some(provider_user_id) = context.resource_owner_user_id.as_deref() else {
        return Ok(None);
    };
    if provider_user_id == consumer_user_id {
        return Ok(None);
    }
    let (prompt_tokens_i64, completion_tokens_i64) = pc_cli_usage_tokens(usage);
    let prompt_tokens = clamp_i64_to_u32(prompt_tokens_i64);
    let completion_tokens = clamp_i64_to_u32(completion_tokens_i64);
    if prompt_tokens == 0 && completion_tokens == 0 {
        return Ok(None);
    }
    let params = SettleParams {
        consumer_user_id,
        provider_user_id,
        node_id,
        model_id: frozen_model_id,
        feature,
        usage_mode: "pc_agent_cli",
        compute_call_id: accounting_result.idempotency_key.as_deref(),
        token_usage_event_id: Some(accounting_result.token_usage_event_id.as_str()),
        billing_event_id: accounting_result.billing_event_id.as_deref(),
        prompt_tokens,
        completion_tokens,
        price_per_1k_credits: pc_cli_price_per_1k_credits(),
        billed_cost_rmb_fen: accounting_result.cost_rmb_fen,
        accounting_status: Some(accounting_result.accounting_status.as_str()),
        provider_revenue_share_x1000: node_router::provider_revenue_share_x1000(&state.store),
        platform_fee_rate: 0.2,
    };
    match state.store.settle_node_inference(params) {
        Ok(tx) => {
            if let Some(lease_id) = context.lease_id.as_deref() {
                let compute_call_id = accounting_result
                    .idempotency_key
                    .as_deref()
                    .ok_or_else(|| anyhow!("共享 Codex 用量缺少 exact compute_call_id"))?;
                state.store.attach_codex_vault_emergency_usage_strict(
                    lease_id,
                    compute_call_id,
                    accounting_result.token_usage_event_id.as_str(),
                    accounting_result.billing_event_id.as_deref(),
                    &tx.id,
                    prompt_tokens as i64,
                    completion_tokens as i64,
                    tx.billed_cost_rmb_fen,
                    tx.provider_earned_fen,
                    &tx.settlement_status,
                )?;
            }
            tracing::debug!(
                consumer_user_id,
                provider_user_id,
                node_id,
                tokens = prompt_tokens + completion_tokens,
                billed_cost_rmb_fen = tx.billed_cost_rmb_fen,
                provider_earned_fen = tx.provider_earned_fen,
                settlement_status = tx.settlement_status,
                "PC CLI 节点收益流水已记录"
            );
            Ok(Some(tx))
        }
        Err(e) => {
            tracing::error!(
                consumer_user_id,
                provider_user_id,
                node_id,
                "PC CLI 节点收益流水记录失败: {e}"
            );
            Err(e.into())
        }
    }
}

pub(super) fn pc_cli_billing_context(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    cli_name: &str,
) -> anyhow::Result<PcCliBillingContext> {
    pc_cli_billing_context_from_store(&state.store, consumer_user_id, node_id, cli_name)
}

fn pc_cli_billing_context_from_store(
    store: &Store,
    consumer_user_id: &str,
    node_id: &str,
    cli_name: &str,
) -> anyhow::Result<PcCliBillingContext> {
    let owner = store
        .get_node_credential_owner(node_id)
        .with_context(|| format!("查询 PC 节点 owner 失败: {node_id}"))?;
    if pc_cli_name_is_codex(cli_name) {
        if owner.as_deref() == Some(consumer_user_id) {
            match store
                .get_active_codex_vault_emergency_lease_for_node(consumer_user_id, node_id)?
            {
                Some(lease) if lease.provider_user_id != consumer_user_id => {
                    return Ok(PcCliBillingContext {
                        billing_source: BILLING_SOURCE_SHARED_CODEX.to_string(),
                        resource_owner_user_id: Some(lease.provider_user_id),
                        lease_id: Some(lease.id),
                        replay_deadline: Some(lease.expires_at),
                        charge_platform_balance: true,
                        max_cost_rmb_fen: 0,
                        allowance_id: None,
                        frozen_reservation_required: false,
                    });
                }
                _ => {}
            }
            return Ok(PcCliBillingContext {
                billing_source: BILLING_SOURCE_OWN_CODEX.to_string(),
                resource_owner_user_id: owner,
                lease_id: None,
                replay_deadline: None,
                charge_platform_balance: false,
                max_cost_rmb_fen: 0,
                allowance_id: None,
                frozen_reservation_required: false,
            });
        }
        if let Some(owner) = owner {
            return Ok(PcCliBillingContext {
                billing_source: BILLING_SOURCE_SHARED_CODEX.to_string(),
                resource_owner_user_id: Some(owner),
                lease_id: None,
                replay_deadline: None,
                charge_platform_balance: true,
                max_cost_rmb_fen: 0,
                allowance_id: None,
                frozen_reservation_required: false,
            });
        }
    }
    Ok(PcCliBillingContext {
        billing_source: BILLING_SOURCE_PLATFORM.to_string(),
        resource_owner_user_id: owner,
        lease_id: None,
        replay_deadline: None,
        charge_platform_balance: true,
        max_cost_rmb_fen: 0,
        allowance_id: None,
        frozen_reservation_required: false,
    })
}

fn pc_cli_name_is_codex(cli_name: &str) -> bool {
    cli_name.trim().to_ascii_lowercase().contains("codex")
}

fn clamp_i64_to_u32(value: i64) -> u32 {
    value.clamp(0, u32::MAX as i64) as u32
}

#[cfg(test)]
#[path = "pc_billing_deadline_tests.rs"]
mod deadline_tests;

#[cfg(test)]
mod tests {
    use super::{
        pc_cli_billing_context_from_store, record_pc_cli_trusted_usage_result,
        refresh_pc_cli_billing_context_from_store,
    };
    use crate::{
        cli_usage::CliTokenUsage,
        store::{
            codex_vault_emergency::CodexVaultEmergencyLeaseCreate,
            token_usage::{BILLING_SOURCE_OWN_CODEX, BILLING_SOURCE_SHARED_CODEX},
            NodeComputeReplayBinding, NodeComputeRunStart, Store,
        },
    };

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-pc-cli-billing-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn refreshed_context_bills_shared_codex_after_local_node_switches_to_provider_auth() {
        let (store, path) = temp_store();
        let consumer = store
            .create_user(
                "pc-billing-consumer@example.com",
                "secret1",
                Some("钱一龙"),
                None,
            )
            .unwrap();
        let provider = store
            .create_user(
                "pc-billing-provider@example.com",
                "secret1",
                Some("全嘉"),
                None,
            )
            .unwrap();
        store
            .create_node_credential(
                "node-consumer",
                "secret-hash",
                &consumer.id,
                Some("钱一龙节点"),
                Some("钱一龙 PC"),
                Some("install-consumer"),
            )
            .unwrap();

        let initial_context =
            pc_cli_billing_context_from_store(&store, &consumer.id, "node-consumer", "codex")
                .unwrap();
        assert_eq!(initial_context.billing_source, BILLING_SOURCE_OWN_CODEX);
        assert_eq!(
            initial_context.resource_owner_user_id.as_deref(),
            Some(consumer.id.as_str())
        );
        assert!(
            !initial_context.charge_platform_balance,
            "own Codex should only be metered, not billed"
        );

        let grant = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &consumer.id,
                Some("全嘉 shares Codex to 钱一龙"),
                Some("robot_codex_vault_shared_access"),
                Some(900),
                None,
                &provider.id,
            )
            .unwrap();
        let _lease = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: "node-consumer",
                provider_slot_id: "slot-quanjia",
                account_hint_hash: Some("hint-quanjia"),
                purpose: Some("unit_test_default_provider_auth"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();

        let refreshed_context =
            pc_cli_billing_context_from_store(&store, &consumer.id, "node-consumer", "codex")
                .unwrap();
        assert_eq!(
            refreshed_context.billing_source,
            BILLING_SOURCE_SHARED_CODEX
        );
        assert_eq!(
            refreshed_context.resource_owner_user_id.as_deref(),
            Some(provider.id.as_str())
        );
        assert!(
            refreshed_context.charge_platform_balance,
            "shared provider auth must be billable to the consumer"
        );

        let usage = CliTokenUsage {
            input_tokens: 3_000,
            cached_input_tokens: 0,
            output_tokens: 2_000,
            reasoning_tokens: 0,
            total_tokens: 5_000,
            model: Some("codex".to_string()),
        };
        let result = record_pc_cli_trusted_usage_result(
            &store,
            &consumer.id,
            "pc_agent_cli_chat",
            "pc-cli/codex",
            &usage,
            "pc_agent_cli:test-shared-context-refresh",
            &refreshed_context,
        )
        .unwrap()
        .expect("shared usage should be recorded");
        assert_ne!(result.accounting_status, "unbilled_own_codex");

        let stats = store.get_usage_stats(&consumer.id, 30).unwrap();
        assert!(stats
            .by_billing_source
            .iter()
            .any(|row| row.billing_source == BILLING_SOURCE_SHARED_CODEX
                && row.total_tokens == 5_000
                && row.call_count == 1));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn completion_uses_server_bound_midrun_shared_context_not_process_snapshot() {
        let (store, path) = temp_store();
        let consumer = store
            .create_user("bound-consumer@example.com", "secret1", None, None)
            .unwrap();
        let provider = store
            .create_user("bound-provider@example.com", "secret1", None, None)
            .unwrap();
        store
            .create_node_credential(
                "bound-node",
                "secret-hash",
                &consumer.id,
                None,
                None,
                Some("bound-install"),
            )
            .unwrap();
        let snapshot =
            pc_cli_billing_context_from_store(&store, &consumer.id, "bound-node", "codex").unwrap();
        assert_eq!(snapshot.billing_source, BILLING_SOURCE_OWN_CODEX);
        store
            .start_node_compute_run(NodeComputeRunStart {
                compute_call_id: "pc_agent_cli:bound-midrun",
                consumer_user_id: &consumer.id,
                provider_user_id: Some(&consumer.id),
                node_id: "bound-node",
                model_id: Some("codex"),
                feature: "pc_agent_cli_chat",
                usage_mode: "pc_agent_cli",
                route_reason: Some("test"),
            })
            .unwrap();
        store
            .bind_node_compute_run_replay_policy(
                "pc_agent_cli:bound-midrun",
                NodeComputeReplayBinding {
                    billing_source: BILLING_SOURCE_SHARED_CODEX,
                    resource_owner_user_id: Some(&provider.id),
                    lease_id: Some("lease-bound-midrun"),
                    offline_policy: "require_active_reservation",
                    replay_deadline: Some(
                        &(chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc3339(),
                    ),
                    max_cost_rmb_fen: 10,
                    allowance_id: Some("reservation-bound-midrun"),
                },
            )
            .unwrap();

        let refreshed = refresh_pc_cli_billing_context_from_store(
            &store,
            &consumer.id,
            "bound-node",
            "pc_agent_cli:bound-midrun",
            &snapshot,
        )
        .unwrap();
        assert_eq!(refreshed.billing_source, BILLING_SOURCE_SHARED_CODEX);
        assert_eq!(
            refreshed.resource_owner_user_id.as_deref(),
            Some(provider.id.as_str())
        );
        assert_eq!(refreshed.lease_id.as_deref(), Some("lease-bound-midrun"));
        assert!(refreshed.charge_platform_balance);

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn frozen_billing_context_requires_matching_run_identity() {
        let (store, path) = temp_store();
        let owner = store
            .create_user("frozen-owner@example.com", "secret1", None, None)
            .unwrap();
        store
            .create_node_credential(
                "frozen-node",
                "secret-hash",
                &owner.id,
                None,
                None,
                Some("frozen-install"),
            )
            .unwrap();
        let snapshot =
            pc_cli_billing_context_from_store(&store, &owner.id, "frozen-node", "codex").unwrap();

        assert!(refresh_pc_cli_billing_context_from_store(
            &store,
            &owner.id,
            "frozen-node",
            "pc_agent_cli:missing",
            &snapshot,
        )
        .is_err());

        store
            .start_node_compute_run(NodeComputeRunStart {
                compute_call_id: "pc_agent_cli:frozen",
                consumer_user_id: &owner.id,
                provider_user_id: Some(&owner.id),
                node_id: "frozen-node",
                model_id: Some("codex"),
                feature: "pc_agent_cli_chat",
                usage_mode: "pc_agent_cli",
                route_reason: Some("test"),
            })
            .unwrap();
        assert!(refresh_pc_cli_billing_context_from_store(
            &store,
            "different-user",
            "frozen-node",
            "pc_agent_cli:frozen",
            &snapshot,
        )
        .is_err());
        assert!(refresh_pc_cli_billing_context_from_store(
            &store,
            &owner.id,
            "different-node",
            "pc_agent_cli:frozen",
            &snapshot,
        )
        .is_err());

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
