use crate::{model::*, prepare, verify};
use anyhow::{ensure, Result};
use serde::de::DeserializeOwned;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub const MAX_CONFIG_BYTES: usize = 8192;
pub const MAX_REQUEST_BYTES: usize = 32768;
pub const MAX_EVIDENCE_BYTES: usize = 1024 * 1024;

pub fn read_json<T: DeserializeOwned>(path: &Path, max: usize) -> Result<T> {
    let bytes = bounded(path, max)?;
    Ok(serde_json::from_slice(&bytes)?)
}
fn bounded(path: &Path, max: usize) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    ensure!(file.metadata()?.is_file(), "regular file required");
    let mut bytes = Vec::new();
    file.take(max as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= max, "input exceeds limit");
    Ok(bytes)
}
fn evidence(directory: &Path, refs: &[EvidenceRef]) -> Result<()> {
    for reference in refs {
        // Filename is derived only from an already validated fixed lowercase digest.
        ensure!(
            verify::fixed_hex(&reference.sha256, 32),
            "invalid evidence digest"
        );
        let bytes = bounded(
            &directory.join(format!("{}.json", reference.sha256)),
            MAX_EVIDENCE_BYTES,
        )?;
        ensure!(
            verify::hash(bytes) == reference.sha256,
            "evidence content differs"
        );
    }
    Ok(())
}

/// The caller must retain the archive and have the independent reconciler inspect
/// its contents. Digest matching proves byte identity, not external financial truth.
pub fn run(
    config_path: &Path,
    request_path: &Path,
    archive: &Path,
    output: &Path,
    now_ms: i64,
) -> Result<()> {
    ensure!(!output.exists(), "output already exists");
    let config: Config = read_json(config_path, MAX_CONFIG_BYTES)?;
    let request: Request = read_json(request_path, MAX_REQUEST_BYTES)?;
    let candidate = prepare(&config, &request, now_ms)?;
    evidence(archive, &request.statement.payload.evidence)?;
    if let Some(previous) = &request.previous {
        evidence(archive, &previous.statement.payload.evidence)?;
    }
    let bytes = serde_json::to_vec_pretty(&candidate)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
