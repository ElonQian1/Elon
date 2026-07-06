use crate::{
    billing_lifecycle::TrustedBillingCall,
    cli_usage::CliTokenUsage,
    node_router,
    store::token_usage::{
        BILLING_SOURCE_OWN_CODEX, BILLING_SOURCE_PLATFORM, BILLING_SOURCE_SHARED_CODEX,
    },
    store::{NodeTransaction, SettleParams, Store, TokenUsageAccountingResult},
    token_usage_api,
    types::AppState,
};

use super::{pc_cli_model_id, pc_cli_price_per_1k_credits, pc_cli_usage_tokens};

#[derive(Debug, Clone)]
pub(super) struct PcCliBillingContext {
    pub(super) billing_source: &'static str,
    pub(super) resource_owner_user_id: Option<String>,
    pub(super) charge_platform_balance: bool,
}

impl PcCliBillingContext {
    pub(super) fn refresh(
        &mut self,
        state: &AppState,
        consumer_user_id: &str,
        node_id: &str,
        cli_name: &str,
    ) {
        *self = pc_cli_billing_context(state, consumer_user_id, node_id, cli_name);
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
    let context = pc_cli_billing_context(state, user_id, node_id, cli_name);
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

pub(super) fn record_pc_cli_trusted_usage(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: Option<&str>,
    usage: &CliTokenUsage,
    accounting_key: &str,
    context: &PcCliBillingContext,
) -> Option<TokenUsageAccountingResult> {
    token_usage_api::record_trusted_usage_with_key_and_resource(
        store,
        user_id,
        feature,
        "pc_agent_cli",
        model,
        usage,
        Some(accounting_key),
        Some(context.billing_source),
        context.resource_owner_user_id.as_deref(),
        context.charge_platform_balance,
    )
}

pub(super) fn settle_pc_cli_node_usage(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    feature: &str,
    model: Option<&str>,
    usage: &CliTokenUsage,
    accounting_result: Option<&TokenUsageAccountingResult>,
) -> Option<NodeTransaction> {
    if accounting_result
        .map(|result| result.deduplicated)
        .unwrap_or(true)
    {
        return None;
    }
    let node_owner = match state.store.get_node_credential_owner(node_id) {
        Ok(Some(owner)) if !owner.trim().is_empty() => owner,
        Ok(_) => {
            tracing::warn!(
                node_id,
                "PC CLI 用量已记录，但节点缺少 owner，跳过节点收益流水"
            );
            return None;
        }
        Err(e) => {
            tracing::warn!(node_id, error = %e, "查询 PC 节点 owner 失败，跳过节点收益流水");
            return None;
        }
    };
    let emergency_lease = if node_owner == consumer_user_id {
        match state
            .store
            .get_active_codex_vault_emergency_lease_for_node(consumer_user_id, node_id)
        {
            Ok(Some(lease)) if lease.provider_user_id != consumer_user_id => Some(lease),
            Ok(_) => None,
            Err(e) => {
                tracing::warn!(node_id, error = %e, "查询 Codex 保险箱共享租约失败，按节点 owner 结算");
                None
            }
        }
    } else {
        None
    };
    let provider_user_id = emergency_lease
        .as_ref()
        .map(|lease| lease.provider_user_id.clone())
        .unwrap_or(node_owner);
    if provider_user_id == consumer_user_id {
        return None;
    }
    let (prompt_tokens_i64, completion_tokens_i64) = pc_cli_usage_tokens(usage);
    let prompt_tokens = clamp_i64_to_u32(prompt_tokens_i64);
    let completion_tokens = clamp_i64_to_u32(completion_tokens_i64);
    if prompt_tokens == 0 && completion_tokens == 0 {
        return None;
    }
    let model_id = pc_cli_model_id(model);
    let params = SettleParams {
        consumer_user_id,
        provider_user_id: &provider_user_id,
        node_id,
        model_id: &model_id,
        feature,
        usage_mode: "pc_agent_cli",
        compute_call_id: accounting_result.and_then(|result| result.idempotency_key.as_deref()),
        token_usage_event_id: accounting_result.map(|result| result.token_usage_event_id.as_str()),
        billing_event_id: accounting_result.and_then(|result| result.billing_event_id.as_deref()),
        prompt_tokens,
        completion_tokens,
        price_per_1k_credits: pc_cli_price_per_1k_credits(),
        billed_cost_rmb_fen: accounting_result
            .map(|result| result.cost_rmb_fen)
            .unwrap_or(0),
        accounting_status: accounting_result.map(|result| result.accounting_status.as_str()),
        provider_revenue_share_x1000: node_router::provider_revenue_share_x1000(&state.store),
        platform_fee_rate: 0.2,
    };
    match state.store.settle_node_inference(params) {
        Ok(tx) => {
            if let Some(lease_id) = emergency_lease.as_ref().map(|lease| lease.id.as_str()) {
                let _ = state.store.attach_codex_vault_emergency_usage(
                    lease_id,
                    accounting_result.map(|result| result.token_usage_event_id.as_str()),
                    accounting_result.and_then(|result| result.billing_event_id.as_deref()),
                    Some(&tx.id),
                    prompt_tokens as i64,
                    completion_tokens as i64,
                    tx.billed_cost_rmb_fen,
                    tx.provider_earned_fen,
                    Some(&tx.settlement_status),
                );
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
            Some(tx)
        }
        Err(e) => {
            tracing::error!(
                consumer_user_id,
                provider_user_id,
                node_id,
                "PC CLI 节点收益流水记录失败: {e}"
            );
            None
        }
    }
}

pub(super) fn pc_cli_billing_context(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    cli_name: &str,
) -> PcCliBillingContext {
    pc_cli_billing_context_from_store(&state.store, consumer_user_id, node_id, cli_name)
}

fn pc_cli_billing_context_from_store(
    store: &Store,
    consumer_user_id: &str,
    node_id: &str,
    cli_name: &str,
) -> PcCliBillingContext {
    let owner = match store.get_node_credential_owner(node_id) {
        Ok(owner) => owner,
        Err(e) => {
            tracing::warn!(node_id, error = %e, "查询 PC 节点 owner 失败，按平台来源记账");
            None
        }
    };
    if pc_cli_name_is_codex(cli_name) {
        if owner.as_deref() == Some(consumer_user_id) {
            match store.get_active_codex_vault_emergency_lease_for_node(consumer_user_id, node_id) {
                Ok(Some(lease)) if lease.provider_user_id != consumer_user_id => {
                    return PcCliBillingContext {
                        billing_source: BILLING_SOURCE_SHARED_CODEX,
                        resource_owner_user_id: Some(lease.provider_user_id),
                        charge_platform_balance: true,
                    };
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        node_id,
                        error = %e,
                        "查询 Codex 保险箱共享租约失败，按自有 Codex 记账"
                    );
                }
            }
            return PcCliBillingContext {
                billing_source: BILLING_SOURCE_OWN_CODEX,
                resource_owner_user_id: owner,
                charge_platform_balance: false,
            };
        }
        if let Some(owner) = owner {
            return PcCliBillingContext {
                billing_source: BILLING_SOURCE_SHARED_CODEX,
                resource_owner_user_id: Some(owner),
                charge_platform_balance: true,
            };
        }
    }
    PcCliBillingContext {
        billing_source: BILLING_SOURCE_PLATFORM,
        resource_owner_user_id: owner,
        charge_platform_balance: true,
    }
}

fn pc_cli_name_is_codex(cli_name: &str) -> bool {
    cli_name.trim().to_ascii_lowercase().contains("codex")
}

fn clamp_i64_to_u32(value: i64) -> u32 {
    value.clamp(0, u32::MAX as i64) as u32
}

#[cfg(test)]
mod tests {
    use super::{pc_cli_billing_context_from_store, record_pc_cli_trusted_usage};
    use crate::{
        cli_usage::CliTokenUsage,
        store::{
            codex_vault_emergency::CodexVaultEmergencyLeaseCreate,
            token_usage::{BILLING_SOURCE_OWN_CODEX, BILLING_SOURCE_SHARED_CODEX},
            Store,
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
            pc_cli_billing_context_from_store(&store, &consumer.id, "node-consumer", "codex");
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
            pc_cli_billing_context_from_store(&store, &consumer.id, "node-consumer", "codex");
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
        let result = record_pc_cli_trusted_usage(
            &store,
            &consumer.id,
            "pc_agent_cli_chat",
            Some("codex"),
            &usage,
            "pc_agent_cli:test-shared-context-refresh",
            &refreshed_context,
        )
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
}
