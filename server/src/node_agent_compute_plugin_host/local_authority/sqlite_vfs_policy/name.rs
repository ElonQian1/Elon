use std::{
    ffi::{CStr, CString},
    fmt,
};

use super::types::ManagedSqliteLogicalFileRole;

const LOGICAL_NAME_PREFIX: &str = "elon-hbsql-v1-";
const JOURNAL_SUFFIX: &str = "-journal";
const WAL_SUFFIX: &str = "-wal";
const TOKEN_NONCE_BYTES: usize = 16;
const HEX: &[u8; 16] = b"0123456789abcdef";

/// An opaque token that a future one-shot registry must mint from a unique nonce.
///
/// The nonce is encoded, never interpreted as a path, and consumed when the exact logical-name
/// set is created. This type intentionally implements neither `Clone` nor `Copy`.
#[must_use = "a one-shot SQLite token must be consumed into its sealed policy"]
pub(super) struct ManagedSqliteOneShotToken {
    encoded: String,
}

impl ManagedSqliteOneShotToken {
    pub(super) fn from_registry_nonce(
        nonce: [u8; TOKEN_NONCE_BYTES],
    ) -> Result<Self, ManagedSqliteLogicalNameRejection> {
        if nonce.iter().all(|byte| *byte == 0) {
            return Err(ManagedSqliteLogicalNameRejection::InvalidRegistryNonce);
        }
        let mut encoded = String::with_capacity(LOGICAL_NAME_PREFIX.len() + nonce.len() * 2);
        encoded.push_str(LOGICAL_NAME_PREFIX);
        for byte in nonce {
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
        Ok(Self { encoded })
    }
}

impl fmt::Debug for ManagedSqliteOneShotToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ManagedSqliteOneShotToken(<opaque>)")
    }
}

pub(super) struct ManagedSqliteLogicalNames {
    main: CString,
    journal: CString,
    wal: CString,
}

impl ManagedSqliteLogicalNames {
    pub(super) fn from_one_shot_token(
        token: ManagedSqliteOneShotToken,
    ) -> Result<Self, ManagedSqliteLogicalNameRejection> {
        let main = token.encoded;
        let journal = format!("{main}{JOURNAL_SUFFIX}");
        let wal = format!("{main}{WAL_SUFFIX}");
        Ok(Self {
            main: CString::new(main).map_err(|_| ManagedSqliteLogicalNameRejection::EmbeddedNul)?,
            journal: CString::new(journal)
                .map_err(|_| ManagedSqliteLogicalNameRejection::EmbeddedNul)?,
            wal: CString::new(wal).map_err(|_| ManagedSqliteLogicalNameRejection::EmbeddedNul)?,
        })
    }

    pub(super) fn get(&self, role: ManagedSqliteLogicalFileRole) -> &CStr {
        match role {
            ManagedSqliteLogicalFileRole::Main => &self.main,
            ManagedSqliteLogicalFileRole::Journal => &self.journal,
            ManagedSqliteLogicalFileRole::Wal => &self.wal,
        }
    }

    pub(super) fn classify(
        &self,
        candidate: Option<&[u8]>,
    ) -> Result<ManagedSqliteLogicalFileRole, ManagedSqliteLogicalNameRejection> {
        let candidate = candidate.ok_or(ManagedSqliteLogicalNameRejection::MissingOrTemporary)?;
        validate_candidate(candidate)?;
        if candidate == self.main.as_bytes() {
            Ok(ManagedSqliteLogicalFileRole::Main)
        } else if candidate == self.journal.as_bytes() {
            Ok(ManagedSqliteLogicalFileRole::Journal)
        } else if candidate == self.wal.as_bytes() {
            Ok(ManagedSqliteLogicalFileRole::Wal)
        } else {
            Err(ManagedSqliteLogicalNameRejection::NotExact)
        }
    }
}

impl fmt::Debug for ManagedSqliteLogicalNames {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ManagedSqliteLogicalNames(<opaque-exact-set>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum ManagedSqliteLogicalNameRejection
{
    InvalidRegistryNonce,
    MissingOrTemporary,
    Empty,
    EmbeddedNul,
    UriSyntax,
    PathSyntax,
    SpecialCharacter,
    NotExact,
}

fn validate_candidate(candidate: &[u8]) -> Result<(), ManagedSqliteLogicalNameRejection> {
    if candidate.is_empty() {
        return Err(ManagedSqliteLogicalNameRejection::Empty);
    }
    if candidate.contains(&0) {
        return Err(ManagedSqliteLogicalNameRejection::EmbeddedNul);
    }
    if candidate
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"file:"))
    {
        return Err(ManagedSqliteLogicalNameRejection::UriSyntax);
    }
    if candidate
        .iter()
        .any(|byte| matches!(byte, b'/' | b'\\' | b':'))
    {
        return Err(ManagedSqliteLogicalNameRejection::PathSyntax);
    }
    if candidate
        .iter()
        .any(|byte| !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'-')
    {
        return Err(ManagedSqliteLogicalNameRejection::SpecialCharacter);
    }
    Ok(())
}
