use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::Serialize;

use super::model::{UploadFailure, UploadReceipt};

pub(super) fn find_cache_root(node_data_root: Option<&Path>) -> Option<PathBuf> {
    cache_root_candidates(node_data_root)
        .into_iter()
        .find(|root| root.join("reports").join("fleet").join("outbox").is_dir())
}

pub(super) fn pending_envelopes(cache_root: &Path, limit: usize) -> Result<Vec<PathBuf>> {
    let outbox = cache_root.join("reports").join("fleet").join("outbox");
    let mut files = std::fs::read_dir(&outbox)
        .with_context(|| format!("read Rust cache fleet outbox {}", outbox.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    files.truncate(limit);
    Ok(files)
}

pub(super) fn archive_accepted(
    cache_root: &Path,
    envelope_path: &Path,
    receipt: &UploadReceipt,
) -> Result<()> {
    let fleet_root = cache_root.join("reports").join("fleet");
    let receipts = fleet_root.join("receipts");
    let accepted = fleet_root.join("accepted");
    std::fs::create_dir_all(&receipts)?;
    std::fs::create_dir_all(&accepted)?;
    let receipt_path = receipts.join(format!("{}.ack.json", receipt.envelope_id));
    write_or_verify_receipt(&receipt_path, receipt)?;

    let file_name = envelope_path
        .file_name()
        .ok_or_else(|| anyhow!("fleet envelope has no file name"))?;
    let destination = accepted.join(file_name);
    if destination.exists() {
        if std::fs::read(&destination)? != std::fs::read(envelope_path)? {
            return Err(anyhow!("accepted fleet envelope name collision"));
        }
        // The immutable accepted copy and matching ACK already exist. Remove only
        // the duplicate outbox copy so this acknowledged item is not retried.
        std::fs::remove_file(envelope_path).context("remove duplicate accepted envelope")?;
        return Ok(());
    }
    std::fs::rename(envelope_path, destination).context("archive accepted fleet envelope")?;
    Ok(())
}

pub(super) fn record_attempt(
    cache_root: &Path,
    envelope_path: &Path,
    failure: &UploadFailure,
) -> Result<()> {
    let attempts = cache_root.join("reports").join("fleet").join("attempts");
    std::fs::create_dir_all(&attempts)?;
    let file_name = envelope_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-envelope");
    let safe_name = file_name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .take(160)
        .collect::<String>();
    let receipt = AttemptReceipt {
        schema: "elon.rust_cache.fleet_upload_attempt.v1",
        attempted_at_utc: Utc::now().to_rfc3339(),
        envelope_file: safe_name,
        failure,
        destructive_actions_authorized: false,
    };
    let path = attempts.join(format!("{}.state.json", receipt.envelope_file));
    std::fs::write(path, serde_json::to_vec_pretty(&receipt)?)?;
    Ok(())
}

fn cache_root_candidates(node_data_root: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    add_env_path(&mut candidates, "ELON_RUST_CACHE_ROOT", None);
    add_env_path(
        &mut candidates,
        "ELON_NODE_DATA_ROOT",
        Some("cache/rust-cache-v2"),
    );
    add_env_path(
        &mut candidates,
        "RUST_SHARED_BUILD_ROOT",
        Some("rust-cache-v2"),
    );
    if let Some(root) = node_data_root {
        candidates.push(root.join("cache").join("rust-cache-v2"));
    }
    let shared = PathBuf::from(r"D:\rust\shared\rust-cache-v2");
    if shared.parent().is_some_and(Path::is_dir) {
        candidates.push(shared);
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Elon").join("rust-cache-v2"));
    }
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|candidate| candidate.is_absolute())
        .filter(|candidate| seen.insert(candidate.clone()))
        .collect()
}

fn add_env_path(candidates: &mut Vec<PathBuf>, name: &str, suffix: Option<&str>) {
    let Some(value) = std::env::var_os(name) else {
        return;
    };
    let mut path = PathBuf::from(value);
    if let Some(suffix) = suffix {
        path.push(suffix);
    }
    candidates.push(path);
}

fn write_or_verify_receipt(path: &Path, receipt: &UploadReceipt) -> Result<()> {
    let payload = serde_json::to_vec_pretty(receipt)?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(&payload)?;
            file.sync_all()?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing: UploadReceipt = serde_json::from_slice(&std::fs::read(path)?)?;
            if existing.envelope_id == receipt.envelope_id
                && existing.node_id == receipt.node_id
                && existing.report_sha256 == receipt.report_sha256
                && existing.envelope_sha256 == receipt.envelope_sha256
                && existing.accepted
                && !existing.destructive_actions_authorized
            {
                Ok(())
            } else {
                Err(anyhow!("fleet upload receipt identity conflict"))
            }
        }
        Err(error) => Err(error.into()),
    }
}

#[derive(Serialize)]
struct AttemptReceipt<'a> {
    schema: &'static str,
    attempted_at_utc: String,
    envelope_file: String,
    failure: &'a UploadFailure,
    destructive_actions_authorized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_envelope_is_moved_only_after_receipt_is_written() {
        let root = std::env::temp_dir().join(format!(
            "elon-rust-cache-fleet-upload-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let outbox = root.join("reports/fleet/outbox");
        std::fs::create_dir_all(&outbox).unwrap();
        let envelope = outbox.join("envelope.json");
        std::fs::write(&envelope, b"immutable").unwrap();
        let receipt = UploadReceipt {
            schema: "elon.rust_cache.fleet_upload_receipt.v1".into(),
            accepted: true,
            deduplicated: false,
            envelope_id: "a".repeat(32),
            node_id: "node-a".into(),
            report_sha256: "1".repeat(64),
            received_at: "2026-08-16T00:00:00Z".into(),
            destructive_actions_authorized: false,
            envelope_sha256: "2".repeat(64),
        };
        archive_accepted(&root, &envelope, &receipt).unwrap();
        assert!(!envelope.exists());
        assert!(root.join("reports/fleet/accepted/envelope.json").exists());
        assert!(root
            .join(format!(
                "reports/fleet/receipts/{}.ack.json",
                receipt.envelope_id
            ))
            .exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn repeated_ack_removes_only_the_identical_outbox_copy() {
        let root = std::env::temp_dir().join(format!(
            "elon-rust-cache-fleet-upload-repeat-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let outbox = root.join("reports/fleet/outbox");
        let accepted = root.join("reports/fleet/accepted");
        std::fs::create_dir_all(&outbox).unwrap();
        std::fs::create_dir_all(&accepted).unwrap();
        let envelope = outbox.join("envelope.json");
        std::fs::write(&envelope, b"immutable").unwrap();
        std::fs::write(accepted.join("envelope.json"), b"immutable").unwrap();
        let receipt = UploadReceipt {
            schema: "elon.rust_cache.fleet_upload_receipt.v1".into(),
            accepted: true,
            deduplicated: true,
            envelope_id: "b".repeat(32),
            node_id: "node-a".into(),
            report_sha256: "3".repeat(64),
            received_at: "2026-08-16T00:00:00Z".into(),
            destructive_actions_authorized: false,
            envelope_sha256: "4".repeat(64),
        };

        archive_accepted(&root, &envelope, &receipt).unwrap();

        assert!(!envelope.exists());
        assert_eq!(
            std::fs::read(accepted.join("envelope.json")).unwrap(),
            b"immutable"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_upload_attempt_keeps_envelope_and_writes_only_safe_state() {
        let root = std::env::temp_dir().join(format!(
            "elon-rust-cache-fleet-upload-failure-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let outbox = root.join("reports/fleet/outbox");
        std::fs::create_dir_all(&outbox).unwrap();
        let envelope = outbox.join("fleet-envelope-safe.json");
        std::fs::write(&envelope, br#"{"immutable":true}"#).unwrap();

        record_attempt(
            &root,
            &envelope,
            &UploadFailure::local("secure-upload-origin-required"),
        )
        .unwrap();

        assert_eq!(std::fs::read(&envelope).unwrap(), br#"{"immutable":true}"#);
        let state = std::fs::read_to_string(
            root.join("reports/fleet/attempts/fleet-envelope-safe.state.json"),
        )
        .unwrap();
        assert!(state.contains("secure-upload-origin-required"));
        assert!(state.contains("\"destructive_actions_authorized\": false"));
        assert!(!state.contains(root.to_string_lossy().as_ref()));
        let _ = std::fs::remove_dir_all(root);
    }
}
