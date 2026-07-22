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

const RECEIPT_DIRECTORY: &str = "terminal-finalization-receipts-v1";
const MAX_RECEIPT_BYTES: u64 = 64 * 1024;
const MAX_ROOT_RECEIPTS: usize = 256;

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
    receipt_root: Option<PathBuf>,
    task_id: Option<String>,
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
            receipt_root: None,
            task_id: None,
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
        let receipt_root = self
            .receipt_root
            .as_deref()
            .context("terminal receipt root missing")?;
        let task_id = self
            .task_id
            .as_deref()
            .context("terminal task id missing")?;
        match self.lease.context("terminal lease expectation missing")? {
            TerminalLeaseExpectation::Exact => {
                anyhow::ensure!(
                    find_receipt(
                        receipt_root,
                        &identity.root_task_id,
                        &identity.active,
                        Some(task_id),
                    )?
                    .is_none(),
                    "failed or canceled completion refuses a finalization receipt"
                );
                identity.revalidate_lease(TerminalLeaseExpectation::Exact)
            }
            TerminalLeaseExpectation::Missing => {
                let path = self
                    .receipt_path
                    .as_ref()
                    .context("terminal receipt path missing")?;
                let (current_path, current_receipt, current) = find_receipt(
                    receipt_root,
                    &identity.root_task_id,
                    &identity.active,
                    Some(task_id),
                )?
                .context("terminal finalization receipt disappeared after preflight")?;
                anyhow::ensure!(
                    current_path == *path
                        && self.original_bytes.as_deref() == Some(current.as_slice()),
                    "terminal finalization receipt changed after preflight"
                );
                validate_completed_identity(
                    &current_receipt,
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
                    let (durable_path, durable, _) = find_receipt(
                        receipt_root,
                        &identity.root_task_id,
                        &identity.active,
                        Some(task_id),
                    )?
                    .context("terminal finalization receipt disappeared after binding")?;
                    anyhow::ensure!(
                        durable_path == *path,
                        "terminal finalization receipt path changed after binding"
                    );
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
    preflight_with_roots(identity, completion, contract_root, None)
}

fn preflight_with_roots<'a>(
    identity: &'a VerifiedTerminalLeaseIdentity,
    completion: &CliCompletionEnvelope,
    contract_root: Option<&Path>,
    receipt_root: Option<&Path>,
) -> Result<TerminalFinalizationPreflight<'a>> {
    let status = crate::node_agent_task_journal_events::completion_terminal_status(
        completion.exit_ok,
        completion.error.as_deref(),
    );
    let receipt_root = receipt_root
        .map(Path::to_path_buf)
        .unwrap_or_else(default_receipt_root);
    let located = find_receipt(
        &receipt_root,
        &identity.root_task_id,
        &identity.active,
        Some(&completion.req_id),
    )?;
    if status != "done" {
        anyhow::ensure!(
            located.is_none(),
            "failed or canceled completion refuses a finalization receipt"
        );
        return Ok(TerminalFinalizationPreflight {
            identity: Some(identity),
            receipt_root: Some(receipt_root),
            task_id: Some(completion.req_id.clone()),
            receipt_path: None,
            original_bytes: None,
            receipt: None,
            bind_done: false,
            lease: Some(TerminalLeaseExpectation::Exact),
            contract_root: contract_root.map(Path::to_path_buf),
        });
    }

    let (path, receipt, bytes) = located
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
        receipt_root: Some(receipt_root),
        task_id: Some(completion.req_id.clone()),
        receipt_path: Some(path),
        original_bytes: Some(bytes),
        receipt: Some(receipt),
        bind_done,
        lease: Some(TerminalLeaseExpectation::Missing),
        contract_root: contract_root.map(Path::to_path_buf),
    })
}

pub(crate) async fn verify_completed_identity(
    runtime: &crate::NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    supervision: &crate::node_agent_local_task_supervision::SupervisionContract,
    completion: &CliCompletionEnvelope,
) -> Result<VerifiedTerminalLeaseIdentity> {
    verify_completed_identity_with_roots(runtime, task, supervision, completion, None, None).await
}

async fn verify_completed_identity_with_roots(
    runtime: &crate::NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    supervision: &crate::node_agent_local_task_supervision::SupervisionContract,
    completion: &CliCompletionEnvelope,
    contract_root: Option<&Path>,
    receipt_root: Option<&Path>,
) -> Result<VerifiedTerminalLeaseIdentity> {
    let root_task_id = supervision.root_task_id.as_deref().unwrap_or(&task.task_id);
    let (_, receipt, _) = find_receipt(
        receipt_root.unwrap_or(&default_receipt_root()),
        root_task_id,
        Path::new(&task.workspace_path),
        Some(&completion.req_id),
    )?
    .context("successful supervised completion requires a completed finalization receipt")?;
    let contract = load_contract(&receipt.task_contract_id, contract_root)?;
    let evidence = crate::node_agent_supervision_finalized_identity::FinalizedIdentityEvidence {
        worktree: PathBuf::from(&receipt.worktree),
        base_workspace: PathBuf::from(&receipt.base_workspace),
        git_dir: PathBuf::from(&receipt.git_dir),
        git_common_dir: PathBuf::from(&receipt.git_common_dir),
        branch: receipt.branch.clone(),
        origin: receipt.origin.clone(),
        final_head: receipt.final_head.clone(),
        base_commit: contract.base_commit.clone(),
    };
    let identity =
        crate::node_agent_supervision_terminal_lease_safety::verify_finalized_terminal_identity(
            runtime,
            task,
            supervision,
            &completion.req_id,
            &evidence,
        )
        .await?;
    validate_completed_identity(&receipt, &identity, contract_root)?;
    Ok(identity)
}

#[cfg(test)]
pub(crate) async fn verify_completed_identity_for_test(
    runtime: &crate::NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    supervision: &crate::node_agent_local_task_supervision::SupervisionContract,
    completion: &CliCompletionEnvelope,
    contract_root: &Path,
    receipt_root: &Path,
) -> Result<VerifiedTerminalLeaseIdentity> {
    verify_completed_identity_with_roots(
        runtime,
        task,
        supervision,
        completion,
        Some(contract_root),
        Some(receipt_root),
    )
    .await
}

#[cfg(test)]
pub(crate) fn preflight_with_roots_for_test<'a>(
    identity: &'a VerifiedTerminalLeaseIdentity,
    completion: &CliCompletionEnvelope,
    contract_root: &Path,
    receipt_root: &Path,
) -> Result<TerminalFinalizationPreflight<'a>> {
    preflight_with_roots(
        identity,
        completion,
        Some(contract_root),
        Some(receipt_root),
    )
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

fn default_receipt_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ElonNode")
        .join(RECEIPT_DIRECTORY)
}

fn receipt_root_key(root_task_id: &str) -> String {
    format!("{:x}", Sha256::digest(root_task_id.as_bytes()))
}

fn find_receipt(
    root: &Path,
    root_task_id: &str,
    worktree: &Path,
    task_id: Option<&str>,
) -> Result<Option<(PathBuf, TerminalFinalizationReceipt, Vec<u8>)>> {
    let directory = root.join(receipt_root_key(root_task_id));
    if !directory.is_dir() {
        return Ok(None);
    }
    let mut entries = fs::read_dir(&directory)
        .with_context(|| format!("read terminal receipt directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    anyhow::ensure!(
        entries.len() <= MAX_ROOT_RECEIPTS,
        "terminal receipt root exceeds the bounded scan limit"
    );
    entries.sort_by_key(|entry| entry.file_name());
    let mut matches = Vec::new();
    let mut observed = Vec::new();
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        anyhow::ensure!(
            metadata.file_type().is_file(),
            "terminal receipt root contains a non-regular entry"
        );
        anyhow::ensure!(
            metadata.len() <= MAX_RECEIPT_BYTES,
            "terminal receipt exceeds the size limit"
        );
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            anyhow::bail!("terminal receipt filename is not UTF-8");
        };
        anyhow::ensure!(
            path.extension().and_then(|value| value.to_str()) == Some("json")
                && lower_hex(stem, 64),
            "terminal receipt filename is not a TaskContract SHA-256 id"
        );
        let bytes = fs::read(&path)?;
        let receipt: TerminalFinalizationReceipt =
            serde_json::from_slice(&bytes).context("parse terminal finalization receipt")?;
        anyhow::ensure!(
            receipt.task_contract_id == stem,
            "terminal receipt filename and TaskContract binding differ"
        );
        observed.push(format!(
            "root={};worktree={};task={}",
            receipt.supervision_root_task_id,
            receipt.worktree,
            receipt.task_id.as_deref().unwrap_or("<unbound>")
        ));
        if receipt.supervision_root_task_id != root_task_id
            || !crate::node_agent_update_checkpoint::same_path(
                Path::new(&receipt.worktree),
                worktree,
            )
            || receipt
                .task_id
                .as_deref()
                .zip(task_id)
                .is_some_and(|(bound, expected)| bound != expected)
        {
            continue;
        }
        matches.push((path, receipt, bytes));
    }
    anyhow::ensure!(
        matches.len() <= 1,
        "multiple terminal finalization receipts match the same task identity"
    );
    if matches.is_empty() && !observed.is_empty() {
        anyhow::bail!(
            "terminal receipt directory has no identity match for root={root_task_id};worktree={}; candidates={}",
            worktree.display(),
            observed.join("|")
        );
    }
    Ok(matches.into_iter().next())
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
