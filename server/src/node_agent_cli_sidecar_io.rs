// server/src/node_agent_cli_sidecar_io.rs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::Path,
};

use crate::node_agent_cli_sidecar::{now_ms, CliSidecarCommandRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CliSidecarOutputRecord {
    #[serde(rename = "type")]
    pub(crate) record_type: String,
    #[serde(default)]
    pub(crate) stream: Option<String>,
    #[serde(default)]
    pub(crate) text: Option<String>,
    #[serde(default)]
    pub(crate) child_pid: Option<u32>,
    #[serde(default)]
    pub(crate) exit_code: Option<i32>,
    #[serde(default)]
    pub(crate) success: Option<bool>,
    #[serde(default)]
    pub(crate) canceled: Option<bool>,
    #[serde(default)]
    pub(crate) error: Option<String>,
    #[serde(default)]
    pub(crate) runtime: Option<serde_json::Value>,
    pub(crate) at_ms: u128,
}

impl CliSidecarOutputRecord {
    pub(crate) fn chunk(stream: &str, text: &str) -> Self {
        Self {
            record_type: "chunk".to_string(),
            stream: Some(stream.to_string()),
            text: Some(crate::node_agent_cli_redaction::redact_text(text)),
            child_pid: None,
            exit_code: None,
            success: None,
            canceled: None,
            error: None,
            runtime: None,
            at_ms: now_ms(),
        }
    }

    pub(crate) fn child_started(pid: u32) -> Self {
        Self {
            record_type: "child_started".to_string(),
            stream: None,
            text: None,
            child_pid: Some(pid),
            exit_code: None,
            success: None,
            canceled: None,
            error: None,
            runtime: None,
            at_ms: now_ms(),
        }
    }

    pub(crate) fn exit(success: bool, canceled: bool) -> Self {
        Self {
            record_type: "exit".to_string(),
            stream: None,
            text: None,
            child_pid: None,
            exit_code: None,
            success: Some(success),
            canceled: Some(canceled),
            error: None,
            runtime: None,
            at_ms: now_ms(),
        }
    }

    pub(crate) fn error(error: String) -> Self {
        Self {
            record_type: "exit".to_string(),
            stream: None,
            text: None,
            child_pid: None,
            exit_code: None,
            success: Some(false),
            canceled: Some(false),
            error: Some(crate::node_agent_cli_redaction::redact_text(&error)),
            runtime: None,
            at_ms: now_ms(),
        }
    }

    pub(crate) fn heartbeat() -> Self {
        Self {
            record_type: "runtime".to_string(),
            stream: None,
            text: None,
            child_pid: None,
            exit_code: None,
            success: None,
            canceled: None,
            error: None,
            runtime: Some(serde_json::json!({ "heartbeat": true })),
            at_ms: now_ms(),
        }
    }
}

pub(crate) fn read_new_commands(
    path: &Path,
    offset: &mut u64,
) -> Result<Vec<CliSidecarCommandRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .with_context(|| format!("打开 sidecar command mailbox {:?}", path))?;
    file.seek(SeekFrom::Start(*offset))?;
    let mut reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if let Ok(record) = serde_json::from_str::<CliSidecarCommandRecord>(line.trim()) {
            records.push(record);
        }
    }
    *offset = reader.stream_position()?;
    Ok(records)
}

pub(crate) fn read_new_output_records(
    path: &Path,
    offset: &mut u64,
) -> Result<Vec<CliSidecarOutputRecord>> {
    read_output_records_from(path, offset, usize::MAX)
}

pub(crate) fn read_output_records_from(
    path: &Path,
    offset: &mut u64,
    limit: usize,
) -> Result<Vec<CliSidecarOutputRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .with_context(|| format!("打开 sidecar output {:?}", path))?;
    file.seek(SeekFrom::Start(*offset))?;
    let mut reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if let Ok(record) = serde_json::from_str::<CliSidecarOutputRecord>(line.trim()) {
            records.push(record);
            if records.len() >= limit {
                break;
            }
        }
    }
    *offset = reader.stream_position()?;
    Ok(records)
}

pub(crate) fn append_output(path: &Path, record: CliSidecarOutputRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建 sidecar output 目录 {:?}", parent))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("打开 sidecar output {:?}", path))?;
    writeln!(file, "{}", serde_json::to_string(&record)?)
        .with_context(|| format!("写入 sidecar output {:?}", path))
}
