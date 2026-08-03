use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_adapter_claim_model::{
        validate_claim_token_shape, validate_release_reason_code,
        CompleteAdapterHandoffClaimRequest, OpenCommerceAdapterHandoffClaimIssue,
        OpenCommerceAdapterHandoffClaimList, OpenCommerceAdapterHandoffClaimPoll,
        OpenCommerceAdapterHandoffClaimRelease, OpenCommerceAdapterHandoffClaimRenew,
        OpenCommerceAdapterHandoffClaimResume, OpenCommerceAdapterHandoffTask,
        ReleaseAdapterHandoffClaimRequest, RenewAdapterHandoffClaimRequest,
        ResumeAdapterHandoffClaimRequest, ADAPTER_HANDOFF_CLAIM_LIST_SCHEMA,
        ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA, ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA,
        ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA, ADAPTER_HANDOFF_CLAIM_RESUME_SCHEMA,
    },
    open_commerce_adapter_model::{
        OpenCommerceAdapterCredential, ADAPTER_HANDOFF_CLAIM_SCOPE, ADAPTER_HANDOFF_SCOPE,
    },
    open_commerce_business_handoff_service,
    open_commerce_connector_contract::{MAX_HANDOFF_LEASE_SECONDS, MIN_HANDOFF_LEASE_SECONDS},
    open_commerce_merchant_evidence_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

const BOUNDARY: [&str; 6] = [
    "只有显式获得 business_handoff.claim 的机器凭据可以领取任务",
    "每次最多返回一条属于凭据商户的终态结果，不提供任意经营数据库查询",
    "租约密钥只返回一次并仅绑定当前任务、接入器、凭据版本和到期时间",
    "同一业务证据同时只能有一个活动租约；超时、主动释放或有界退避结束后才可重新领取",
    "完成回执与租约状态原子绑定，网络重放不会生成第二条回执",
    "领取和回执均不是平台对外部 ERP/CRM、订单、支付或履约的独立核验",
];

pub(crate) fn list_claims(
    store: &Store,
    project_id: &str,
    limit: usize,
) -> Result<OpenCommerceAdapterHandoffClaimList> {
    Ok(OpenCommerceAdapterHandoffClaimList {
        schema: ADAPTER_HANDOFF_CLAIM_LIST_SCHEMA,
        project_id: project_id.trim().to_string(),
        claims: store.list_project_open_commerce_adapter_handoff_claims(project_id, limit)?,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(crate) fn claim_next(
    store: &Store,
    credential: &OpenCommerceAdapterCredential,
    lease_seconds: i64,
) -> Result<OpenCommerceAdapterHandoffClaimPoll> {
    require_claim_scope(credential)?;
    if !(MIN_HANDOFF_LEASE_SECONDS..=MAX_HANDOFF_LEASE_SECONDS).contains(&lease_seconds) {
        bail!("衔接任务租约必须在 60 至 900 秒之间");
    }
    let candidate_ids = store.list_open_commerce_adapter_handoff_candidate_ids(credential, 30)?;
    for invocation_id in candidate_ids {
        let detail = open_commerce_merchant_evidence_service::get_evidence(
            store,
            &credential.project_id,
            &credential.merchant_id,
            &invocation_id,
        )?;
        if detail.evidence.status != "succeeded" || detail.evidence.receipt_state != "valid" {
            continue;
        }
        let Some(result) = detail.result else {
            continue;
        };
        let Some((claim, lease_token)) = store.try_claim_open_commerce_adapter_handoff(
            credential,
            &invocation_id,
            lease_seconds,
        )?
        else {
            continue;
        };
        store.record_open_commerce_audit(
            &credential.project_id,
            &credential.created_by_user_id,
            Some(&format!("adapter-{}", credential.id)),
            "business_handoff.claimed",
            "business_handoff_claim",
            &claim.id,
            &json!({
                "merchant_id":claim.merchant_id,
                "invocation_id":claim.invocation_id,
                "integration_id":claim.integration_id,
                "adapter_credential_id":claim.adapter_credential_id,
                "adapter_credential_version":claim.adapter_credential_version,
                "attempt_no":claim.attempt_no,
                "lease_expires_at":claim.lease_expires_at,
                "lease_token_visible_once":true,
                "funds_moved":false
            }),
        )?;
        store.touch_open_commerce_adapter_credential(&credential.id)?;
        return Ok(OpenCommerceAdapterHandoffClaimPoll {
            schema: ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA,
            claimed: true,
            issue: Some(OpenCommerceAdapterHandoffClaimIssue {
                claim,
                lease_token,
                lease_token_visible_once: true,
                task: OpenCommerceAdapterHandoffTask {
                    evidence: detail.evidence,
                    result,
                },
            }),
            retry_after_seconds: 0,
            boundary: BOUNDARY.to_vec(),
        });
    }
    Ok(OpenCommerceAdapterHandoffClaimPoll {
        schema: ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA,
        claimed: false,
        issue: None,
        retry_after_seconds: 15,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(crate) fn complete_claim(
    store: &Store,
    credential: &OpenCommerceAdapterCredential,
    claim_id: &str,
    request: CompleteAdapterHandoffClaimRequest,
) -> Result<crate::open_commerce_business_handoff_model::OpenCommerceBusinessHandoffReceipt> {
    require_claim_scope(credential)?;
    validate_claim_token_shape(&request.lease_token)?;
    let claim = store.verify_open_commerce_adapter_handoff_claim(
        credential,
        claim_id,
        &request.lease_token,
    )?;
    let lease_token = request.lease_token.clone();
    let receipt = open_commerce_business_handoff_service::record_claimed_adapter_receipt(
        store,
        credential,
        &claim,
        &lease_token,
        request,
    )?;
    store.touch_open_commerce_adapter_credential(&credential.id)?;
    Ok(receipt)
}

pub(crate) fn release_claim(
    store: &Store,
    credential: &OpenCommerceAdapterCredential,
    claim_id: &str,
    request: ReleaseAdapterHandoffClaimRequest,
) -> Result<OpenCommerceAdapterHandoffClaimRelease> {
    require_claim_scope(credential)?;
    validate_claim_token_shape(&request.lease_token)?;
    let reason_code = validate_release_reason_code(&request.reason_code)?;
    let claim = store.release_open_commerce_adapter_handoff_claim(
        credential,
        claim_id,
        &request.lease_token,
        reason_code,
    )?;
    store.record_open_commerce_audit(
        &credential.project_id,
        &credential.created_by_user_id,
        Some(&format!("adapter-{}", credential.id)),
        "business_handoff.claim_released",
        "business_handoff_claim",
        &claim.id,
        &json!({
            "merchant_id":claim.merchant_id,
            "invocation_id":claim.invocation_id,
            "integration_id":claim.integration_id,
            "adapter_credential_id":claim.adapter_credential_id,
            "adapter_credential_version":claim.adapter_credential_version,
            "attempt_no":claim.attempt_no,
            "reason_code":reason_code,
            "retryable":true,
            "funds_moved":false
        }),
    )?;
    store.touch_open_commerce_adapter_credential(&credential.id)?;
    Ok(OpenCommerceAdapterHandoffClaimRelease {
        schema: ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA,
        claim,
        retryable: true,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(crate) fn renew_claim(
    store: &Store,
    credential: &OpenCommerceAdapterCredential,
    claim_id: &str,
    request: RenewAdapterHandoffClaimRequest,
) -> Result<OpenCommerceAdapterHandoffClaimRenew> {
    require_claim_scope(credential)?;
    validate_claim_token_shape(&request.lease_token)?;
    if !(MIN_HANDOFF_LEASE_SECONDS..=MAX_HANDOFF_LEASE_SECONDS).contains(&request.extend_seconds) {
        bail!("续租时长必须在 60 至 900 秒之间");
    }
    let claim = store.renew_open_commerce_adapter_handoff_claim(
        credential,
        claim_id,
        &request.lease_token,
        request.extend_seconds,
    )?;
    store.record_open_commerce_audit(
        &credential.project_id,
        &credential.created_by_user_id,
        Some(&format!("adapter-{}", credential.id)),
        "business_handoff.claim_renewed",
        "business_handoff_claim",
        &claim.id,
        &json!({
            "merchant_id":claim.merchant_id,
            "invocation_id":claim.invocation_id,
            "integration_id":claim.integration_id,
            "adapter_credential_id":claim.adapter_credential_id,
            "adapter_credential_version":claim.adapter_credential_version,
            "attempt_no":claim.attempt_no,
            "lease_expires_at":claim.lease_expires_at,
            "lease_deadline_at":claim.lease_deadline_at,
            "funds_moved":false
        }),
    )?;
    store.touch_open_commerce_adapter_credential(&credential.id)?;
    Ok(OpenCommerceAdapterHandoffClaimRenew {
        schema: ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA,
        claim,
        renewed: true,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(crate) fn resume_retry(
    store: &Store,
    project_id: &str,
    claim_id: &str,
    request: ResumeAdapterHandoffClaimRequest,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceAdapterHandoffClaimResume> {
    if !actor.project_role.is_some_and(can_edit) {
        bail!("只有项目编辑者可以重新排队暂停重试的接入器任务");
    }
    if !request.confirmed_by_user {
        bail!("重新排队接入器任务前必须取得用户明确确认");
    }
    let claim =
        store.resume_open_commerce_adapter_handoff_retry(project_id, claim_id, actor.user_id)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "business_handoff.claim_retry_resumed",
        "business_handoff_claim",
        &claim.id,
        &json!({
            "merchant_id":claim.merchant_id,
            "invocation_id":claim.invocation_id,
            "integration_id":claim.integration_id,
            "attempt_no":claim.attempt_no,
            "previous_suspension_reason":claim.retry_suspension_reason,
            "resumed":true,
            "funds_moved":false
        }),
    )?;
    Ok(OpenCommerceAdapterHandoffClaimResume {
        schema: ADAPTER_HANDOFF_CLAIM_RESUME_SCHEMA,
        claim,
        resumed: true,
        funds_moved: false,
        boundary: BOUNDARY.to_vec(),
    })
}

fn require_claim_scope(credential: &OpenCommerceAdapterCredential) -> Result<()> {
    if credential.status != "active"
        || !credential
            .scopes
            .iter()
            .any(|scope| scope == ADAPTER_HANDOFF_SCOPE)
        || !credential
            .scopes
            .iter()
            .any(|scope| scope == ADAPTER_HANDOFF_CLAIM_SCOPE)
    {
        bail!("适配器凭据未获得 business_handoff.claim 权限");
    }
    Ok(())
}
