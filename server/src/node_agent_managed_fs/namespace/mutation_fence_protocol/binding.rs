use std::{ffi::OsStr, io};

use sha2::{Digest, Sha256};

use super::{invalid_data, MinifilterDigest, MINIFILTER_FENCE_MAX_CHILD_NAME_UTF16_UNITS};

const AUTHORITY_BINDING_DOMAIN: &[u8] = b"ELON_NODE_NAMESPACE_FENCE_AUTHORITY_V1";
const CLEANUP_ID_DOMAIN: &[u8] = b"ELON_NODE_NAMESPACE_FENCE_CLEANUP_ID_V1";
const REQUESTED_NAME_DOMAIN: &[u8] = b"ELON_NODE_NAMESPACE_FENCE_REQUESTED_NAME_V1";

/// Opaque authority facts stored with a kernel-derived parent/name scope. None of these digests
/// substitutes for validating the actual parent handle, volume, FileId or filesystem name rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MinifilterFenceAuthorityBinding {
    pub(super) installation_id_digest: MinifilterDigest,
    pub(super) cleanup_id_digest: MinifilterDigest,
    pub(super) execution_plan_digest: MinifilterDigest,
    pub(super) authorization_receipt_digest: MinifilterDigest,
    pub(super) expected_object_digest: MinifilterDigest,
    pub(super) expected_object_identity_digest: MinifilterDigest,
    pub(super) expected_parent_identity_digest: MinifilterDigest,
    pub(super) requested_name_digest: MinifilterDigest,
    pub(super) authority_epoch: u64,
    pub(super) process_owner_epoch: u64,
    pub(super) step_ordinal: u64,
}

impl MinifilterFenceAuthorityBinding {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        installation_id_digest: &str,
        cleanup_id: &str,
        execution_plan_digest: &str,
        authorization_receipt_digest: &str,
        expected_object_digest: &str,
        expected_object_identity_digest: &str,
        expected_parent_identity_digest: &str,
        relative_name: &OsStr,
        authority_epoch: i64,
        process_owner_epoch: i64,
        step_ordinal: u64,
    ) -> io::Result<Self> {
        if cleanup_id.is_empty()
            || cleanup_id.len() > 256
            || cleanup_id.trim() != cleanup_id
            || cleanup_id.chars().any(char::is_control)
        {
            return Err(invalid_data("NODE_MINIFILTER_FENCE_CLEANUP_ID_INVALID"));
        }
        let authority_epoch = positive_epoch(
            authority_epoch,
            "NODE_MINIFILTER_FENCE_AUTHORITY_EPOCH_INVALID",
        )?;
        let process_owner_epoch = positive_epoch(
            process_owner_epoch,
            "NODE_MINIFILTER_FENCE_PROCESS_EPOCH_INVALID",
        )?;
        let requested_name_digest = requested_name_digest_utf16(&child_name_utf16(relative_name)?);
        Ok(Self {
            installation_id_digest: decode_sha256(installation_id_digest)?,
            cleanup_id_digest: domain_sha256(CLEANUP_ID_DOMAIN, cleanup_id.as_bytes()),
            execution_plan_digest: decode_sha256(execution_plan_digest)?,
            authorization_receipt_digest: decode_sha256(authorization_receipt_digest)?,
            expected_object_digest: decode_sha256(expected_object_digest)?,
            expected_object_identity_digest: decode_sha256(expected_object_identity_digest)?,
            expected_parent_identity_digest: decode_sha256(expected_parent_identity_digest)?,
            requested_name_digest,
            authority_epoch,
            process_owner_epoch,
            step_ordinal,
        })
    }

    pub(super) fn digest(&self) -> MinifilterDigest {
        let mut hasher = Sha256::new();
        hasher.update(AUTHORITY_BINDING_DOMAIN);
        hasher.update([0]);
        hasher.update(self.installation_id_digest);
        hasher.update(self.cleanup_id_digest);
        hasher.update(self.execution_plan_digest);
        hasher.update(self.authorization_receipt_digest);
        hasher.update(self.expected_object_digest);
        hasher.update(self.expected_object_identity_digest);
        hasher.update(self.expected_parent_identity_digest);
        hasher.update(self.requested_name_digest);
        hasher.update(self.authority_epoch.to_le_bytes());
        hasher.update(self.process_owner_epoch.to_le_bytes());
        hasher.update(self.step_ordinal.to_le_bytes());
        hasher.finalize().into()
    }
}

pub(super) fn validate_child_name_utf16(value: &[u16]) -> io::Result<()> {
    if value.is_empty()
        || value.len() > MINIFILTER_FENCE_MAX_CHILD_NAME_UTF16_UNITS
        || value
            .iter()
            .any(|unit| matches!(*unit, 0 | 0x2f | 0x3a | 0x5c))
        || (value.len() == 1 && value[0] == b'.' as u16)
        || (value.len() == 2 && value[0] == b'.' as u16 && value[1] == b'.' as u16)
    {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_CHILD_NAME_INVALID"));
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn child_name_utf16(value: &OsStr) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    let encoded = value.encode_wide().collect::<Vec<_>>();
    validate_child_name_utf16(&encoded)?;
    Ok(encoded)
}

#[cfg(not(windows))]
pub(super) fn child_name_utf16(_value: &OsStr) -> io::Result<Vec<u16>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "NODE_MINIFILTER_FENCE_WINDOWS_ONLY",
    ))
}

pub(super) fn requested_name_digest_utf16(value: &[u16]) -> MinifilterDigest {
    let mut bytes = Vec::with_capacity(value.len() * 2);
    for unit in value {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    domain_sha256(REQUESTED_NAME_DOMAIN, &bytes)
}

pub(super) fn decode_sha256(value: &str) -> io::Result<MinifilterDigest> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_SHA256_INVALID"));
    }
    let bytes =
        hex::decode(value).map_err(|_| invalid_data("NODE_MINIFILTER_FENCE_SHA256_INVALID"))?;
    bytes
        .try_into()
        .map_err(|_| invalid_data("NODE_MINIFILTER_FENCE_SHA256_INVALID"))
}

fn positive_epoch(value: i64, code: &'static str) -> io::Result<u64> {
    u64::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| invalid_data(code))
}

fn domain_sha256(domain: &[u8], payload: &[u8]) -> MinifilterDigest {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update([0]);
    hasher.update(payload);
    hasher.finalize().into()
}
