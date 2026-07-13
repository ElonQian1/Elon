//! Fail-closed policy shared by live PC CLI completion and durable replay.

use anyhow::{anyhow, Context};

use super::pc_billing::PcCliBillingContext;
use crate::{
    billing_lifecycle::TrustedBillingCall,
    cli_usage::CliTokenUsage,
    store::{NodeComputeRunFinish, Store},
};

/// Type-state proof that billing preparation committed before the first
/// `CliPrompt` send attempt. Its private field prevents dispatch callers from
/// fabricating readiness without going through the durable-hold transition.
pub(crate) struct PcCliBillingDispatchReady {
    compute_call_id: String,
}

impl PcCliBillingDispatchReady {
    pub(crate) fn matches_pc_req_id(&self, req_id: &str) -> bool {
        self.compute_call_id == format!("pc_agent_cli:{req_id}")
    }
}

/// Persist the bounded balance hold before any prompt can be sent to the node.
/// This transaction must commit before dispatch so a server crash can never
/// leave already-started work protected only by the expiry janitor's TTL.
pub(super) fn prepare_pc_cli_billing_for_dispatch(
    call: &mut TrustedBillingCall<'_>,
    context: &PcCliBillingContext,
) -> anyhow::Result<PcCliBillingDispatchReady> {
    let held = call.hold_for_dispatch().map_err(|error| anyhow!(error))?;
    if context.charge_platform_balance && !held {
        return Err(anyhow!("PC CLI 派发前没有可持久化的计费预授权"));
    }
    Ok(PcCliBillingDispatchReady {
        compute_call_id: call.key().to_string(),
    })
}

/// Once dispatch may have reached a node, durable replay owns the hold until a
/// trusted completion settles it or explicit verification releases it.
pub(super) fn defer_pc_cli_billing_after_acceptance(
    call: &mut TrustedBillingCall<'_>,
    _context: &PcCliBillingContext,
) {
    // Own-Codex runs may be upgraded to a managed/shared lease after
    // acceptance. Deferring unconditionally also protects that later hold.
    call.handoff_to_durable_replay();
}

pub(super) fn pc_cli_unknown_usage_requires_verification(
    context: Option<&PcCliBillingContext>,
    usage: Option<&CliTokenUsage>,
) -> bool {
    usage.is_none() && context.is_none_or(PcCliBillingContext::requires_cloud_control)
}

pub(super) fn bind_pc_cli_usage_to_frozen_model(
    usage: Option<CliTokenUsage>,
    frozen_model_id: &str,
) -> Option<CliTokenUsage> {
    usage.map(|mut usage| {
        usage.model = Some(frozen_model_id.to_string());
        usage
    })
}

pub(super) fn hold_pc_cli_usage_for_verification(
    store: &Store,
    user_id: &str,
    compute_call_id: &str,
) -> anyhow::Result<()> {
    store
        .hold_billing_reservation_for_verification(user_id, compute_call_id)?
        .with_context(|| format!("PC CLI 待核验预授权不存在: {compute_call_id}"))?;
    let run = store
        .finish_node_compute_run(
            compute_call_id,
            NodeComputeRunFinish {
                provider_user_id: None,
                status: "verification_pending",
                prompt_tokens: 0,
                completion_tokens: 0,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some("usage_verification_pending"),
                error_message: Some("completion 缺少可信 token 用量，预授权等待人工核验"),
            },
        )?
        .ok_or_else(|| anyhow!("PC CLI 待核验计算运行不存在: {compute_call_id}"))?;
    if run.status != "verification_pending" {
        return Err(anyhow!("PC CLI 计算运行未进入待核验状态"));
    }
    Ok(())
}

pub(super) fn mark_pc_cli_dispatch_outcome_unknown(
    store: &Store,
    user_id: &str,
    compute_call_id: &str,
    error_message: &str,
) -> anyhow::Result<()> {
    store
        .get_active_billing_reservation(user_id, compute_call_id)?
        .with_context(|| format!("PC CLI 未确认派发的 durable hold 不存在: {compute_call_id}"))?;
    let run = store
        .finish_node_compute_run(
            compute_call_id,
            NodeComputeRunFinish {
                provider_user_id: None,
                status: "verification_pending",
                prompt_tokens: 0,
                completion_tokens: 0,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some("dispatch_outcome_unknown"),
                error_message: Some(error_message),
            },
        )?
        .ok_or_else(|| anyhow!("PC CLI 未确认派发的计算运行不存在: {compute_call_id}"))?;
    if run.status != "verification_pending" {
        return Err(anyhow!("PC CLI 未确认派发未进入待核验状态"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        bind_pc_cli_usage_to_frozen_model, defer_pc_cli_billing_after_acceptance,
        hold_pc_cli_usage_for_verification, pc_cli_unknown_usage_requires_verification,
        prepare_pc_cli_billing_for_dispatch,
    };
    use crate::{
        billing_lifecycle::TrustedBillingCall,
        cli_usage::CliTokenUsage,
        store::{
            token_usage::{
                BILLING_SOURCE_OWN_CODEX, BILLING_SOURCE_PLATFORM, BILLING_SOURCE_SHARED_CODEX,
            },
            NodeComputeRunStart, Store,
        },
    };

    use crate::ai_cli::pc_billing::PcCliBillingContext;

    fn context(source: &str, charge_platform_balance: bool) -> PcCliBillingContext {
        PcCliBillingContext {
            billing_source: source.to_string(),
            resource_owner_user_id: None,
            lease_id: None,
            replay_deadline: None,
            charge_platform_balance,
            max_cost_rmb_fen: 0,
            allowance_id: None,
            frozen_reservation_required: false,
        }
    }

    #[test]
    fn shared_and_platform_unknown_usage_require_verification() {
        for context in [
            context(BILLING_SOURCE_SHARED_CODEX, true),
            context(BILLING_SOURCE_PLATFORM, true),
        ] {
            assert!(pc_cli_unknown_usage_requires_verification(
                Some(&context),
                None
            ));
        }
        assert!(!pc_cli_unknown_usage_requires_verification(
            Some(&context(BILLING_SOURCE_OWN_CODEX, false)),
            None
        ));
        assert!(pc_cli_unknown_usage_requires_verification(None, None));
    }

    #[test]
    fn pre_send_dispatch_hold_prevents_later_no_usage_release() {
        let path = std::env::temp_dir().join(format!(
            "elon-pc-live-fail-closed-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).unwrap();
        let user = store
            .create_user(
                &format!("live-hold-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        store
            .billing_recharge(&user.id, 1_000, "test", "test", None)
            .unwrap();
        let key = "pc_agent_cli:live-unknown-usage";
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
        store
            .start_node_compute_run(NodeComputeRunStart {
                compute_call_id: key,
                consumer_user_id: &user.id,
                provider_user_id: None,
                node_id: "node-live-hold",
                model_id: Some("pc-cli/codex"),
                feature: "pc_agent_cli_chat",
                usage_mode: "pc_agent_cli",
                route_reason: Some("test"),
            })
            .unwrap();

        let platform = context(BILLING_SOURCE_PLATFORM, true);
        let ready = prepare_pc_cli_billing_for_dispatch(&mut call, &platform).unwrap();
        assert!(ready.matches_pc_req_id("live-unknown-usage"));
        assert!(!ready.matches_pc_req_id("different-run"));
        assert_eq!(
            store
                .admin_billing_reservations(Some("dispatch_hold"), 10)
                .unwrap()[0]
                .status,
            "dispatch_hold"
        );
        defer_pc_cli_billing_after_acceptance(&mut call, &platform);
        hold_pc_cli_usage_for_verification(&store, &user.id, key).unwrap();
        call.release_no_usage();
        drop(call);

        assert!(store
            .billing_reservation_is_still_reserved(&user.id, key)
            .unwrap());
        assert_eq!(
            store
                .get_node_compute_run_by_compute_call_id(key)
                .unwrap()
                .unwrap()
                .status,
            "verification_pending"
        );
        assert_eq!(store.release_expired_billing_reservations().unwrap(), 0);
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn own_codex_dispatch_does_not_create_a_balance_hold() {
        let path = std::env::temp_dir().join(format!(
            "elon-pc-own-dispatch-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).unwrap();
        let user = store
            .create_user(
                &format!("own-dispatch-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let mut call =
            TrustedBillingCall::no_platform_charge(&store, &user.id, "pc_agent_cli:own-dispatch");

        prepare_pc_cli_billing_for_dispatch(&mut call, &context(BILLING_SOURCE_OWN_CODEX, false))
            .unwrap();
        defer_pc_cli_billing_after_acceptance(&mut call, &context(BILLING_SOURCE_OWN_CODEX, false));
        drop(call);

        assert!(store
            .admin_billing_reservations(None, 10)
            .unwrap()
            .is_empty());
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn client_reported_model_is_replaced_by_server_frozen_model() {
        let usage = bind_pc_cli_usage_to_frozen_model(
            Some(CliTokenUsage {
                input_tokens: 10,
                total_tokens: 10,
                model: Some("attacker-controlled-free-model".to_string()),
                ..CliTokenUsage::default()
            }),
            "pc-cli/server-authorized-model",
        )
        .unwrap();

        assert_eq!(
            usage.model.as_deref(),
            Some("pc-cli/server-authorized-model")
        );
    }
}
