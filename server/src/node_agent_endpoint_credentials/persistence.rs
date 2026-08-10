use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use super::secure_store;
use super::types::{
    CurrentEndpointCredential, EndpointCredentialState, PendingEndpointMutation,
    PendingMutationAction, PersistedCurrentEndpointCredential, PersistedEndpointCredentialState,
    STORE_FILE, STORE_SCHEMA,
};

const MAX_STORE_BYTES: u64 = 64 * 1024;

pub(super) fn default_path() -> Result<PathBuf> {
    let state_path = crate::node_agent_config::state_path();
    let root = state_path
        .parent()
        .ok_or_else(|| anyhow!("节点状态目录不存在"))?;
    Ok(root.join(STORE_FILE))
}

pub(super) fn load_default() -> Result<EndpointCredentialState> {
    load(&default_path()?)
}

fn load(path: &Path) -> Result<EndpointCredentialState> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(EndpointCredentialState::absent())
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查 endpoint credential 状态 {}", path.display()))
        }
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID: 状态文件类型无效");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID: 拒绝重解析点状态文件");
        }
    }
    if metadata.len() == 0 || metadata.len() > MAX_STORE_BYTES {
        bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID: 状态文件大小无效");
    }
    let persisted: PersistedEndpointCredentialState = serde_json::from_slice(
        &fs::read(path)
            .with_context(|| format!("无法读取 endpoint credential 状态 {}", path.display()))?,
    )
    .with_context(|| {
        format!(
            "NODE_ENDPOINT_CREDENTIAL_STORE_INVALID: 状态文件损坏 {}",
            path.display()
        )
    })?;
    if persisted.schema != STORE_SCHEMA || !persisted.endpoint_required {
        bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID: schema 或永久门禁无效");
    }
    let endpoint_https_origin =
        crate::node_agent_endpoint_credentials::normalize_endpoint_https_origin(
            &persisted.endpoint_https_origin,
        )?;
    let current = persisted
        .current
        .map(|current| {
            current.binding.validate()?;
            if current.protection != secure_store::protection_name() {
                bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID: 凭据保护方式不匹配");
            }
            let secret = current
                .protected_secret_base64
                .as_deref()
                .map(secure_store::unprotect)
                .transpose()?;
            Ok(CurrentEndpointCredential {
                binding: current.binding,
                secret,
            })
        })
        .transpose()?;
    if let Some(pending) = persisted.pending_mutation.as_ref() {
        pending.validate()?;
    }
    validate_consistency(
        current.as_ref().map(|current| &current.binding),
        persisted.pending_mutation.as_ref(),
    )?;
    Ok(EndpointCredentialState {
        endpoint_required: true,
        endpoint_https_origin: Some(endpoint_https_origin),
        current,
        pending_mutation: persisted.pending_mutation,
    })
}

pub(super) fn save_parts(
    endpoint_required: bool,
    endpoint_https_origin: Option<&str>,
    current: Option<(
        &super::types::EndpointAuthorityBinding,
        Option<&super::types::EndpointSecret>,
    )>,
    pending_mutation: Option<&super::types::PendingEndpointMutation>,
) -> Result<()> {
    let path = default_path()?;
    if !endpoint_required {
        bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID: 不得持久化可降级状态");
    }
    let endpoint_https_origin =
        endpoint_https_origin.ok_or_else(|| anyhow!("NODE_ENDPOINT_HTTPS_ORIGIN_REQUIRED"))?;
    let endpoint_https_origin =
        crate::node_agent_endpoint_credentials::normalize_endpoint_https_origin(
            endpoint_https_origin,
        )?;
    if let Some((binding, _)) = current {
        binding.validate()?;
    }
    if let Some(pending) = pending_mutation {
        pending.validate()?;
    }
    validate_consistency(current.map(|(binding, _)| binding), pending_mutation)?;
    let current = current
        .map(|(binding, secret)| -> Result<_> {
            Ok(PersistedCurrentEndpointCredential {
                binding: binding.clone(),
                protection: secure_store::protection_name().to_string(),
                protected_secret_base64: secret.map(secure_store::protect).transpose()?,
            })
        })
        .transpose()?;
    let persisted = PersistedEndpointCredentialState {
        schema: STORE_SCHEMA.to_string(),
        endpoint_required: true,
        endpoint_https_origin,
        current,
        pending_mutation: pending_mutation.cloned(),
    };
    let json =
        serde_json::to_vec_pretty(&persisted).context("无法序列化 endpoint credential 状态")?;
    crate::node_agent_atomic_file::write(&path, &json)
        .with_context(|| format!("无法持久化 endpoint credential 状态 {}", path.display()))
}

fn validate_consistency(
    current: Option<&super::types::EndpointAuthorityBinding>,
    pending: Option<&PendingEndpointMutation>,
) -> Result<()> {
    let Some(pending) = pending else {
        return Ok(());
    };
    match pending.action {
        PendingMutationAction::Issue if current.is_none() => Ok(()),
        PendingMutationAction::Recover => {
            let current =
                current.ok_or_else(|| anyhow!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID"))?;
            let expected = pending
                .expected_credential
                .as_ref()
                .ok_or_else(|| anyhow!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID"))?;
            if current.agent_id == pending.agent_id
                && current.owner_user_id == pending.owner_user_id
                && current.install_id == pending.install_id
                && current.credential_id == expected.credential_id
                && current.credential_revision == expected.credential_revision
                && current.credential_digest == expected.credential_digest
            {
                Ok(())
            } else {
                bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID")
            }
        }
        _ => bail!("NODE_ENDPOINT_CREDENTIAL_STORE_INVALID"),
    }
}
