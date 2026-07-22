//! Read-only preflight and idempotent binding for PowerShell finalization receipts.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use homecli_proto::CliCompletionEnvelope;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::node_agent_supervision_terminal_lease_safety::{
    TerminalLeaseExpectation, VerifiedTerminalLeaseIdentity,
};

const RECEIPT_FILE: &str = "elon-terminal-finalization-v1.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TerminalFinalizationReceipt {
    schema: String,
    state: String,
    finalization_id: String,
    task_id: Option<String>,
    completion_event_id: Option<String>,
    terminal_status: Option<String>,
    task_contract_id: String,
    supervision_root_task_id: String,
    worktree: String,
    base_workspace: String,
    git_dir: String,
    git_common_dir: String,
    branch: String,
    origin: String,
    final_head: String,
    lease_marker_fingerprint: String,
    fingerprint: String,
    prepared_at_utc: String,
    completed_at_utc: Option<String>,
    bound_at_utc: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskContract {
    schema: String,
    worktree: String,
    branch: String,
    base_commit: String,
    origin: String,
    issued_at_utc: String,
    nonce: String,
    platform_provenance: String,
    supervision_root_task_id: String,
    lease_reason: String,
    git_common_dir: String,
}

pub(crate) struct TerminalFinalizationPreflight<'a> {
    identity: Option<&'a VerifiedTerminalLeaseIdentity>,
    receipt_path: Option<PathBuf>,
    original_bytes: Option<Vec<u8>>,
    receipt: Option<TerminalFinalizationReceipt>,
    bind_done: bool,
    lease: Option<TerminalLeaseExpectation>,
    contract_root: Option<PathBuf>,
}

impl<'a> TerminalFinalizationPreflight<'a> {
    pub(crate) fn not_applicable() -> Self {
        Self {
            identity: None,
            receipt_path: None,
            original_bytes: None,
            receipt: None,
            bind_done: false,
            lease: None,
            contract_root: None,
        }
    }

    pub(crate) fn commit(mut self, completion: &CliCompletionEnvelope) -> Result<()> {
        let Some(identity) = self.identity else {
            return Ok(());
        };
        let path = self
            .receipt_path
            .as_ref()
            .context("terminal receipt path missing")?;
        match self.lease.context("terminal lease expectation missing")? {
            TerminalLeaseExpectation::Exact => {
                anyhow::ensure!(
                    read_receipt(path)?.is_none(),
                    "failed or canceled completion refuses a finalization receipt"
                );
                identity.revalidate_lease(TerminalLeaseExpectation::Exact)
            }
            TerminalLeaseExpectation::Missing => {
                let current = fs::read(path)
                    .with_context(|| format!("reread terminal receipt {}", path.display()))?;
                anyhow::ensure!(
                    self.original_bytes.as_deref() == Some(current.as_slice()),
                    "terminal finalization receipt changed after preflight"
                );
                validate_completed_identity(
                    self.receipt.as_ref().context("terminal receipt missing")?,
                    identity,
                    self.contract_root.as_deref(),
                )?;
                identity.revalidate_lease(TerminalLeaseExpectation::Missing)?;
                if self.bind_done {
                    let receipt = self.receipt.as_mut().context("terminal receipt missing")?;
                    receipt.task_id = Some(completion.req_id.clone());
                    receipt.completion_event_id = Some(completion.event_id.clone());
                    receipt.terminal_status = Some("done".to_string());
                    receipt.bound_at_utc =
                        Some(Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true));
                    crate::node_agent_atomic_file::write(path, &serde_json::to_vec(receipt)?)?;
                }
                let durable_check = (|| {
                    let (durable, _) = read_receipt(path)?
                        .context("terminal finalization receipt disappeared after binding")?;
                    assert_binding(&durable, completion)?;
                    identity.revalidate_lease(TerminalLeaseExpectation::Missing)
                })();
                if let Err(error) = durable_check {
                    if self.bind_done {
                        crate::node_agent_atomic_file::write(
                            path,
                            self.original_bytes
                                .as_deref()
                                .context("terminal receipt rollback bytes missing")?,
                        )
                        .context("restore terminal receipt after lease drift")?;
                    }
                    return Err(error);
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn preflight<'a>(
    identity: &'a VerifiedTerminalLeaseIdentity,
    completion: &CliCompletionEnvelope,
) -> Result<TerminalFinalizationPreflight<'a>> {
    preflight_with_contract_root(identity, completion, None)
}

fn preflight_with_contract_root<'a>(
    identity: &'a VerifiedTerminalLeaseIdentity,
    completion: &CliCompletionEnvelope,
    contract_root: Option<&Path>,
) -> Result<TerminalFinalizationPreflight<'a>> {
    let status = crate::node_agent_task_journal_events::completion_terminal_status(
        completion.exit_ok,
        completion.error.as_deref(),
    );
    let path = identity.git_dir.join(RECEIPT_FILE);
    if status != "done" {
        anyhow::ensure!(
            read_receipt(&path)?.is_none(),
            "failed or canceled completion refuses a finalization receipt"
        );
        return Ok(TerminalFinalizationPreflight {
            identity: Some(identity),
            receipt_path: Some(path),
            original_bytes: None,
            receipt: None,
            bind_done: false,
            lease: Some(TerminalLeaseExpectation::Exact),
            contract_root: contract_root.map(Path::to_path_buf),
        });
    }

    let (receipt, bytes) = read_receipt(&path)?
        .context("successful supervised completion requires a completed finalization receipt")?;
    validate_completed_identity(&receipt, identity, contract_root)?;
    let fields = [
        receipt.task_id.is_some(),
        receipt.completion_event_id.is_some(),
        receipt.terminal_status.is_some(),
        receipt.bound_at_utc.is_some(),
    ];
    let bind_done = fields.iter().all(|present| !present);
    anyhow::ensure!(
        bind_done || fields.iter().all(|present| *present),
        "terminal finalization receipt has a partial terminal binding"
    );
    if !bind_done {
        assert_binding(&receipt, completion)?;
    }
    Ok(TerminalFinalizationPreflight {
        identity: Some(identity),
        receipt_path: Some(path),
        original_bytes: Some(bytes),
        receipt: Some(receipt),
        bind_done,
        lease: Some(TerminalLeaseExpectation::Missing),
        contract_root: contract_root.map(Path::to_path_buf),
    })
}

#[cfg(test)]
pub(crate) fn preflight_with_contract_root_for_test<'a>(
    identity: &'a VerifiedTerminalLeaseIdentity,
    completion: &CliCompletionEnvelope,
    contract_root: &Path,
) -> Result<TerminalFinalizationPreflight<'a>> {
    preflight_with_contract_root(identity, completion, Some(contract_root))
}

fn validate_completed_identity(
    receipt: &TerminalFinalizationReceipt,
    identity: &VerifiedTerminalLeaseIdentity,
    contract_root: Option<&Path>,
) -> Result<()> {
    anyhow::ensure!(
        receipt.schema == "elon.terminal_finalization.v1",
        "unsupported terminal receipt schema"
    );
    anyhow::ensure!(
        receipt.state == "completed",
        "successful completion requires a completed receipt"
    );
    anyhow::ensure!(
        lower_hex(&receipt.finalization_id, 32),
        "invalid terminal finalization id"
    );
    anyhow::ensure!(
        lower_hex(&receipt.task_contract_id, 64),
        "invalid TaskContract id"
    );
    anyhow::ensure!(
        lower_hex(&receipt.lease_marker_fingerprint, 64),
        "invalid lease fingerprint"
    );
    anyhow::ensure!(
        lower_hex(&receipt.fingerprint, 64),
        "invalid terminal fingerprint"
    );
    anyhow::ensure!(
        receipt.supervision_root_task_id == identity.root_task_id,
        "terminal root identity drifted"
    );
    path_eq(&receipt.worktree, &identity.active, "worktree")?;
    path_eq(&receipt.base_workspace, &identity.base, "base workspace")?;
    path_eq(&receipt.git_dir, &identity.git_dir, "git-dir")?;
    path_eq(
        &receipt.git_common_dir,
        &identity.git_common_dir,
        "git common-dir",
    )?;
    parse_timestamp(&receipt.prepared_at_utc, "preparedAtUtc")?;
    parse_timestamp(
        receipt
            .completed_at_utc
            .as_deref()
            .context("completedAtUtc missing")?,
        "completedAtUtc",
    )?;
    anyhow::ensure!(
        receipt.fingerprint == receipt_fingerprint(receipt),
        "terminal immutable fingerprint is invalid"
    );

    let contract = load_contract(&receipt.task_contract_id, contract_root)?;
    anyhow::ensure!(
        contract.schema == "elon.ai_finish_contract.v1",
        "unsupported TaskContract schema"
    );
    anyhow::ensure!(
        contract.platform_provenance == "elon.conversation_worktree.v1",
        "TaskContract provenance is not trusted"
    );
    anyhow::ensure!(
        contract.supervision_root_task_id == identity.root_task_id,
        "TaskContract root drifted"
    );
    anyhow::ensure!(
        contract.lease_reason == format!("elon-supervision:{}", identity.root_task_id),
        "TaskContract lease drifted"
    );
    path_eq(
        &contract.worktree,
        &identity.active,
        "TaskContract worktree",
    )?;
    path_eq(
        &contract.git_common_dir,
        &identity.git_common_dir,
        "TaskContract common-dir",
    )?;
    anyhow::ensure!(
        contract.branch == receipt.branch && contract.origin == receipt.origin,
        "TaskContract branch or origin drifted"
    );
    anyhow::ensure!(
        lower_hex(&contract.base_commit, 40),
        "TaskContract base commit is invalid"
    );
    anyhow::ensure!(
        lower_hex(&contract.nonce, 32),
        "TaskContract nonce is invalid"
    );
    parse_timestamp(&contract.issued_at_utc, "TaskContract issuedAtUtc")?;
    identity.verify_successful_git_state(
        &receipt.final_head,
        &contract.base_commit,
        &receipt.branch,
        &receipt.origin,
        Path::new(&receipt.git_common_dir),
    )
}

fn assert_binding(
    receipt: &TerminalFinalizationReceipt,
    completion: &CliCompletionEnvelope,
) -> Result<()> {
    anyhow::ensure!(
        receipt.task_id.as_deref() == Some(completion.req_id.as_str())
            && receipt.completion_event_id.as_deref() == Some(completion.event_id.as_str()),
        "terminal receipt binds a different task or completion event"
    );
    anyhow::ensure!(
        receipt.terminal_status.as_deref() == Some("done"),
        "completed receipt can bind only done"
    );
    parse_timestamp(
        receipt
            .bound_at_utc
            .as_deref()
            .context("boundAtUtc missing")?,
        "boundAtUtc",
    )
}

fn read_receipt(path: &Path) -> Result<Option<(TerminalFinalizationReceipt, Vec<u8>)>> {
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.file_type().is_file(),
        "terminal receipt is not a regular file"
    );
    let bytes = fs::read(path)?;
    let receipt = serde_json::from_slice(&bytes).context("parse terminal finalization receipt")?;
    Ok(Some((receipt, bytes)))
}

fn load_contract(id: &str, override_root: Option<&Path>) -> Result<TaskContract> {
    let root = match override_root {
        Some(root) => root.to_path_buf(),
        None => {
            PathBuf::from(std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA unavailable")?)
                .join("ElonNode")
                .join("ai-finish-contracts-v1")
        }
    };
    let path = root.join(format!("{id}.json"));
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("inspect immutable TaskContract {}", path.display()))?;
    anyhow::ensure!(
        metadata.file_type().is_file(),
        "TaskContract is not a regular file"
    );
    let bytes = fs::read(&path)?;
    anyhow::ensure!(
        format!("{:x}", Sha256::digest(&bytes)) == id,
        "TaskContract SHA-256 mismatch"
    );
    serde_json::from_slice(&bytes).context("parse immutable TaskContract")
}

fn receipt_fingerprint(receipt: &TerminalFinalizationReceipt) -> String {
    let text = format!(
        "taskContractId={}\nsupervisionRootTaskId={}\nworktree={}\nbaseWorkspace={}\ngitDir={}\ngitCommonDir={}\nbranch={}\norigin={}\nfinalHead={}\nleaseMarkerFingerprint={}",
        receipt.task_contract_id, receipt.supervision_root_task_id, receipt.worktree,
        receipt.base_workspace, receipt.git_dir, receipt.git_common_dir, receipt.branch,
        receipt.origin, receipt.final_head, receipt.lease_marker_fingerprint,
    );
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn path_eq(recorded: &str, actual: &Path, label: &str) -> Result<()> {
    anyhow::ensure!(
        crate::node_agent_update_checkpoint::same_path(Path::new(recorded), actual),
        "terminal {label} identity drifted"
    );
    Ok(())
}

fn parse_timestamp(value: &str, label: &str) -> Result<()> {
    DateTime::parse_from_rfc3339(value).with_context(|| format!("terminal {label} is invalid"))?;
    Ok(())
}

fn lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
