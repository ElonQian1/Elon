use std::collections::BTreeSet;

use sha2::{Digest as _, Sha256};

use super::super::{
    source_scope_support::{is_lower_hex, lower_hex, symbol_span},
    LeafRecordV1, SourceWitnessV1,
};
use super::{ProductionSourceSnapshotV1, PRODUCTION_SOURCE_SCOPE};

pub(crate) fn validate_record_source_witnesses(records: &[LeafRecordV1]) -> Result<(), String> {
    for record in records {
        if record.source_branch.is_empty() {
            return Err(format!(
                "source-first record has no source witnesses: {}",
                record.key.identity.leaf_id
            ));
        }
        for witness in &record.source_branch {
            validate_source_witness(witness)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_source_witness(witness: &SourceWitnessV1) -> Result<(), String> {
    if witness.symbol.is_empty() || witness.needle.is_empty() || witness.occurrence == 0 {
        return Err("source witness contains an empty identity or zero occurrence".to_owned());
    }
    let snapshot = PRODUCTION_SOURCE_SCOPE
        .iter()
        .find(|snapshot| snapshot.owner_id == witness.owner_id)
        .ok_or_else(|| {
            format!(
                "source witness owner is outside frozen scope: {}",
                witness.owner_id
            )
        })?;
    let span = symbol_span(snapshot.source, &witness.symbol).ok_or_else(|| {
        format!(
            "source witness symbol is absent or ambiguous in {}: {}",
            snapshot.repo_relative_path, witness.symbol
        )
    })?;
    if span
        .match_indices(&witness.needle)
        .nth(usize::from(witness.occurrence - 1))
        .is_none()
    {
        return Err(format!(
            "source witness occurrence is absent from symbol span {}: {}",
            witness.symbol, witness.needle
        ));
    }
    Ok(())
}

pub(super) fn validate_snapshot_shape(snapshot: &ProductionSourceSnapshotV1) -> Result<(), String> {
    if snapshot.owner_id.is_empty()
        || snapshot.repo_relative_path.is_empty()
        || !snapshot.repo_relative_path.starts_with("src/")
        || snapshot.repo_relative_path.contains('\\')
        || snapshot
            .repo_relative_path
            .split('/')
            .any(|part| part == "..")
    {
        return Err(format!(
            "invalid production source identity: {} ({})",
            snapshot.owner_id, snapshot.repo_relative_path
        ));
    }
    if !is_lower_hex(snapshot.git_blob_oid_sha1, 40)
        || !is_lower_hex(snapshot.normalized_lf_sha256, 64)
    {
        return Err(format!(
            "invalid production source digest metadata: {}",
            snapshot.owner_id
        ));
    }
    if snapshot.symbol_sentinels.is_empty() {
        return Err(format!(
            "production source owner has no symbol sentinels: {}",
            snapshot.owner_id
        ));
    }
    let mut symbols = BTreeSet::new();
    for symbol in snapshot.symbol_sentinels {
        if symbol.is_empty() || !symbols.insert(*symbol) {
            return Err(format!(
                "empty or duplicate symbol sentinel for production owner: {}",
                snapshot.owner_id
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_snapshot_bytes(snapshot: &ProductionSourceSnapshotV1) -> Result<(), String> {
    for symbol in snapshot.symbol_sentinels {
        if !snapshot.source.contains(symbol) {
            return Err(format!(
                "production source sentinel is absent from {}: {symbol}",
                snapshot.repo_relative_path
            ));
        }
    }

    let normalized = snapshot.source.replace("\r\n", "\n");
    let normalized_sha256 = lower_hex(Sha256::digest(normalized.as_bytes()).as_ref());
    if normalized_sha256 != snapshot.normalized_lf_sha256 {
        return Err(format!(
            "production source normalized SHA-256 drifted: {}",
            snapshot.repo_relative_path
        ));
    }

    let mut git_blob = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);
    git_blob.update(format!("blob {}\0", normalized.len()).as_bytes());
    git_blob.update(normalized.as_bytes());
    let git_blob_oid = lower_hex(git_blob.finish().as_ref());
    if git_blob_oid != snapshot.git_blob_oid_sha1 {
        return Err(format!(
            "production source git blob OID drifted: {}",
            snapshot.repo_relative_path
        ));
    }
    Ok(())
}
