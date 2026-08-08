use anyhow::{bail, Result};

use super::NodeComputePluginInstallPlanPreparationDispatchIntent;

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_inert_observation(
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
    accepted: bool,
    replayed: bool,
    context_ready: bool,
    context: Option<&serde_json::Value>,
    bootstrap_instance_id: &str,
    value: &serde_json::Value,
) -> Result<()> {
    let observed: homecli_proto::ComputePluginInstallPlanPreparationObservedV1 =
        serde_json::from_value(value.clone())?;
    if serde_json::to_value(&observed)? != *value
        || observed.schema
            != homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA
        || observed.preparation_id != intent.preparation_id
        || observed.node_id != intent.node_id
        || observed.owner_user_id != intent.owner_user_id
        || observed.accepted != accepted
        || observed.replayed != replayed
        || observed.context_ready != context_ready
        || observed.bootstrap_instance_id != bootstrap_instance_id
        || !bounded_text(bootstrap_instance_id)
        || observed.phase != "blocked"
        || observed.blocked_reasons.is_empty()
        || observed.blocked_reasons.len() > 64
        || observed
            .blocked_reasons
            .iter()
            .any(|reason| !bounded_text(reason))
        || observed
            .error_code
            .as_deref()
            .is_some_and(|code| !bounded_text(code))
    {
        bail!("算力插件 InstallPlan 准备观察正文或绑定无效");
    }
    if context_ready || context.is_some() || observed.context_ready || observed.context.is_some() {
        bail!("算力插件 InstallPlan 准备上下文尚未接通生产事实源");
    }
    if observed.compute_plugin_root_lock_acquired
        || observed.trusted_time_authority_configured
        || observed.rollback_anchor_witness_configured
        || observed.root_pinned
        || observed.authority_opened
        || observed.process_fence_acquired
        || observed.new_work_admission_enabled
        || observed.downloads_allowed
        || observed.side_effects_started
    {
        bail!("算力插件 InstallPlan 准备观察不得声称任何副作用已开始");
    }
    if observed
        .installation_identity_digest
        .as_deref()
        .is_some_and(|digest| digest != intent.installation_identity_digest.as_str())
    {
        bail!("算力插件 InstallPlan 准备观察安装身份不一致");
    }
    if accepted {
        let policy_revision = u64::try_from(intent.policy_revision)?;
        let authorization_revision = u64::try_from(intent.authorization.revision)?;
        let expected_authorization = homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
            authorization_ref: intent.authorization.authorization_ref.clone(),
            revision: authorization_revision,
            digest: intent.authorization.digest.clone(),
        };
        if observed.installation_identity_digest.as_deref()
            != Some(intent.installation_identity_digest.as_str())
            || observed.observed_policy_revision != Some(policy_revision)
            || observed.observed_policy_digest.as_deref() != Some(intent.policy_digest.as_str())
            || observed.observed_policy_snapshot_digest.as_deref()
                != Some(intent.policy_snapshot_digest.as_str())
            || observed.observed_authorization.as_ref() != Some(&expected_authorization)
            || observed.error_code.is_some()
        {
            bail!("算力插件 InstallPlan 准备接受观察未精确绑定请求");
        }
    } else if observed.error_code.is_none() {
        bail!("算力插件 InstallPlan 准备拒绝观察缺少稳定错误码");
    }
    Ok(())
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
