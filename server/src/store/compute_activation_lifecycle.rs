use anyhow::{anyhow, bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::compute_federation_activation_model::{
    ComputeActivationEvidenceRequest, ACTIVATION_REQUEST_STATUS_APPROVED,
    ACTIVATION_REQUEST_STATUS_SUPERSEDED,
};

use super::{compute_activation_requests::request_on, now, Store};

#[derive(Debug, Clone)]
pub(crate) struct SupersedeComputeActivationEvidenceRequest {
    pub request_id: String,
    pub expected_request_digest: String,
    pub actor_user_id: String,
    pub reason: String,
}

impl Store {
    pub(crate) fn supersede_compute_activation_evidence_request(
        &self,
        input: SupersedeComputeActivationEvidenceRequest,
    ) -> Result<ComputeActivationEvidenceRequest> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = request_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("激活证据申请不存在"))?;
        if current.request_digest != input.expected_request_digest.trim() {
            bail!("激活证据申请内容已变化，请刷新后重试");
        }
        if current.status == ACTIVATION_REQUEST_STATUS_SUPERSEDED {
            if current.superseded_by_user_id.as_deref() != Some(input.actor_user_id.trim())
                || current.supersede_reason.as_deref() != Some(input.reason.trim())
            {
                bail!("该激活证据申请已由不同废止操作终结");
            }
            tx.commit()?;
            return Ok(current);
        }
        if current.status != ACTIVATION_REQUEST_STATUS_APPROVED {
            bail!("只有 approved 激活证据申请可以显式废止");
        }

        let superseded_at = now();
        let changed = tx.execute(
            "UPDATE compute_activation_evidence_requests
                SET status='superseded', superseded_at=?1,
                    superseded_by_user_id=?2, supersede_reason=?3, updated_at=?1
              WHERE request_id=?4 AND status='approved' AND request_digest=?5",
            params![
                superseded_at,
                input.actor_user_id.trim(),
                input.reason.trim(),
                input.request_id.trim(),
                input.expected_request_digest.trim(),
            ],
        )?;
        if changed != 1 {
            bail!("激活证据申请状态已并发变化");
        }
        let request = request_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("激活证据申请废止后无法读取"))?;
        tx.commit()?;
        Ok(request)
    }
}

fn validate_input(input: &SupersedeComputeActivationEvidenceRequest) -> Result<()> {
    validate_exact("激活证据申请 ID", &input.request_id, 160)?;
    validate_exact("废止执行人", &input.actor_user_id, 160)?;
    validate_exact("废止原因", &input.reason, 1000)?;
    let digest = input.expected_request_digest.as_str();
    if digest.len() != 64
        || digest != digest.to_ascii_lowercase()
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("激活证据申请摘要必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}
