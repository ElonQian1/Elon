use anyhow::{bail, Result};

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(super) fn validate(
    record: &homecli_proto::ComputePluginInstallPlanPlanningInstalledRecordV2,
) -> Result<()> {
    let Some(work_admission) = record.work_admission.as_ref() else {
        return Ok(());
    };
    let active_provenance_complete = [
        record.active_slot_ref.is_some(),
        record.active_release.is_some(),
        record.active_install_receipt_digest.is_some(),
        record.active_promotion_receipt_digest.is_some(),
        record.active_signed_manifest_envelope_digest.is_some(),
        record.permission_grant_digest.is_some(),
    ]
    .into_iter()
    .all(|present| present);
    if !active_provenance_complete
        || record.desired_presence != "present"
        || record.desired_activation != "enabled"
        || record.admission != "allowed"
        || record.runtime_phase != "stopped"
        || record.active_attempts != 0
        || record.candidate_slot_ref.is_some()
        || record.candidate.is_some()
        || work_admission.generation == 0
        || work_admission.generation > MAX_SAFE_INTEGER
        || !is_sha256(&work_admission.receipt_digest)
    {
        bail!("算力插件 Planning Snapshot V2 work-admission head 无效");
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
