use anyhow::{anyhow, bail, Result};
use sha2::{Digest, Sha256};

use crate::{
    erp_blueprint::model::{ErpBlueprintVersion, ErpInstance},
    open_commerce_runtime_model::{
        normalize_credential_ref, normalize_manifest_sha256, normalize_timeout_ms,
    },
    open_commerce_runtime_security::validate_endpoint_base_url,
};

use super::model::{
    CreateManagedRolloutPlanRequest, ManagedEdgeRoute, ManagedMerchantInstanceContract,
    ManagedRolloutPayload, ManagedRolloutSource, ManagedRuntimeCandidate, EDGE_ROUTE_SCHEMA,
    MANAGED_INSTANCE_SCHEMA, ROLLOUT_PLAN_SCHEMA,
};

pub(super) fn compile_payload(
    instance: &ErpInstance,
    version: &ErpBlueprintVersion,
    merchant_id: &str,
    request: CreateManagedRolloutPlanRequest,
) -> Result<ManagedRolloutPayload> {
    let instance_key = normalize_instance_key(&instance.instance_key)?;
    let target_node_id = normalize_target_node_id(&request.target_node_id)?;
    let service_user = normalize_service_user(&request.service_user)?;
    let store_id = uuid::Uuid::parse_str(request.store_id.trim())?.to_string();
    let profile_source = normalize_linux_path(&request.profile_source, "profile_source")?;
    let secrets_source = normalize_linux_path(&request.secrets_source, "secrets_source")?;
    if !(1024..=65535).contains(&request.listen_port) {
        bail!("listen_port 必须在 1024 到 65535 之间");
    }
    let runtime_key_id = normalize_credential_ref(&request.runtime_key_id)?;
    let public_base_path = request.public_base_path.trim().to_string();
    let expected_path = format!("/merchants/{instance_key}");
    if public_base_path != expected_path {
        bail!("public_base_path 必须是当前实例的 {expected_path}");
    }
    let endpoint_base_url = validate_endpoint_base_url(&request.endpoint_base_url)?;
    let endpoint =
        reqwest::Url::parse(&endpoint_base_url).map_err(|_| anyhow!("候选 Runtime 地址无效"))?;
    if endpoint.path() != public_base_path {
        bail!("候选 Runtime 地址路径必须与 public_base_path 一致");
    }
    let manifest_sha256 = normalize_manifest_sha256(Some(&request.runtime_manifest_sha256))?
        .ok_or_else(|| anyhow!("runtime_manifest_sha256 不能为空"))?;
    let timeout_ms = normalize_timeout_ms(request.timeout_ms)?;
    let release_manifest_sha256 =
        normalize_sha256(&version.manifest_sha256, "已固定蓝图版本 manifest_sha256")?;

    Ok(ManagedRolloutPayload {
        schema: ROLLOUT_PLAN_SCHEMA.to_string(),
        source: ManagedRolloutSource {
            instance_id: instance.id.clone(),
            instance_key: instance_key.clone(),
            configuration_revision: instance.configuration_revision,
            blueprint_version_id: version.id.clone(),
            blueprint_version: version.manifest.version.clone(),
            release_manifest_sha256,
            merchant_id: merchant_id.to_string(),
        },
        target_node_id,
        deployment_contract: ManagedMerchantInstanceContract {
            schema: MANAGED_INSTANCE_SCHEMA.to_string(),
            instance_id: instance_key.clone(),
            service_user,
            merchant_id: merchant_id.to_string(),
            store_id,
            profile_source,
            secrets_source,
            listen_port: request.listen_port,
            runtime_key_id: runtime_key_id.clone(),
            public_base_path: public_base_path.clone(),
            enabled: true,
        },
        edge_route: ManagedEdgeRoute {
            schema: EDGE_ROUTE_SCHEMA.to_string(),
            instance_id: instance_key,
            public_base_path,
            upstream_addr: format!("127.0.0.1:{}", request.listen_port),
            enabled: true,
        },
        runtime_candidate: ManagedRuntimeCandidate {
            endpoint_base_url,
            credential_ref: runtime_key_id,
            manifest_sha256,
            timeout_ms,
        },
        boundaries: vec![
            "plan_only_no_remote_execution".to_string(),
            "target_node_identity_not_verified".to_string(),
            "secret_values_not_included".to_string(),
            "runtime_binding_not_activated".to_string(),
            "payment_and_external_platforms_not_covered".to_string(),
        ],
    })
}

pub(super) fn payload_hash(payload: &ManagedRolloutPayload) -> Result<String> {
    let bytes = serde_json::to_vec(payload)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn normalize_instance_key(value: &str) -> Result<String> {
    let value = value.trim();
    if !(3..=48).contains(&value.len())
        || !value.starts_with(|ch: char| ch.is_ascii_lowercase())
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        bail!("ERP instance_key 不符合托管实例合同");
    }
    Ok(value.to_string())
}

fn normalize_target_node_id(value: &str) -> Result<String> {
    let value = value.trim();
    if !(3..=120).contains(&value.len())
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-'))
    {
        bail!("target_node_id 只能包含字母、数字、点、下划线、冒号和连字符");
    }
    Ok(value.to_string())
}

fn normalize_service_user(value: &str) -> Result<String> {
    let value = value.trim();
    let suffix = value.strip_prefix("ym-").unwrap_or_default();
    if !(2..=28).contains(&suffix.len())
        || !suffix.starts_with(|ch: char| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        || !suffix
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        bail!("service_user 必须使用 ym- 前缀和安全的小写标识");
    }
    Ok(value.to_string())
}

fn normalize_linux_path(value: &str, field: &str) -> Result<String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty()
        || !value.starts_with('/')
        || value.len() > 512
        || value.contains('\0')
        || value.contains("//")
        || value
            .split('/')
            .any(|segment| matches!(segment, "." | ".."))
        || value.chars().any(char::is_control)
    {
        bail!("{field} 必须是规范化的 Linux 绝对路径");
    }
    Ok(value.to_string())
}

fn normalize_sha256(value: &str, field: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("{field} 必须是 64 位十六进制摘要");
    }
    Ok(value)
}
