use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const TOOL_TIMEOUT_SECONDS: u64 = 6 * 60 * 60;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GcNodeCommand {
    schema: String,
    command: String,
    request_id: String,
    node_id: String,
    expires_at: String,
    options: GcOptions,
    plan_id: Option<String>,
    plan_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
struct GcOptions {
    force_aged: bool,
    workspace_only: bool,
    recover_missing_workspaces: bool,
    shared_aliases_only: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GcPlanSummary {
    schema: String,
    request_id: String,
    plan_id: String,
    plan_digest: String,
    node_id: String,
    generated_at_utc: String,
    expires_at_utc: String,
    options: GcOptions,
    action_count: u64,
    reclaim_bytes: u64,
    active_writer_count: u64,
    reasons: Vec<GcReasonCount>,
    security: GcPlanSecurity,
}

#[derive(Debug, Deserialize, Serialize)]
struct GcReasonCount {
    reason: String,
    count: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct GcPlanSecurity {
    absolute_paths_included: bool,
    secrets_included: bool,
    destructive_actions_authorized: bool,
    approval_binds_plan_digest: bool,
    target_rescan_required: bool,
}

#[derive(Debug, Deserialize)]
struct LocalPlan {
    summary: GcPlanSummary,
}

#[derive(Debug, Deserialize)]
struct LocalReceipt {
    schema: String,
    request_id: String,
    plan_id: String,
    plan_digest: String,
    node_id: String,
    status: String,
    completed_at_utc: String,
    approved_action_count: u64,
    removed_action_count: u64,
    reclaimed_bytes: u64,
    security: LocalReceiptSecurity,
}

#[derive(Debug, Deserialize)]
struct LocalReceiptSecurity {
    absolute_paths_included: bool,
    secrets_included: bool,
    execution_bound_to_plan_digest: bool,
    local_rescan_completed: bool,
}

#[derive(Debug, Serialize)]
struct NodeGcResultSummary {
    schema: &'static str,
    request_id: String,
    node_id: String,
    phase: &'static str,
    status: String,
    plan_id: Option<String>,
    plan_digest: Option<String>,
    completed_at_utc: String,
    approved_action_count: u64,
    removed_action_count: u64,
    reclaimed_bytes: u64,
    failure_code: Option<&'static str>,
    security: NodeGcResultSecurity,
}

#[derive(Debug, Serialize)]
struct NodeGcResultSecurity {
    absolute_paths_included: bool,
    secrets_included: bool,
    execution_bound_to_plan_digest: bool,
    local_rescan_completed: bool,
}

pub(super) async fn poll_and_execute(
    client: &Client,
    base_url: &Url,
    node_id: &str,
    agent_secret: &str,
    cache_root: &Path,
) -> Result<()> {
    let endpoint = gc_endpoint(base_url, &["api", "node", "cache-gc", node_id, "next"])?;
    let response = client
        .get(endpoint)
        .bearer_auth(agent_secret)
        .send()
        .await?;
    if response.status() == StatusCode::NO_CONTENT {
        return Ok(());
    }
    if !response.status().is_success() {
        return Err(anyhow!("GC command poll rejected: {}", response.status()));
    }
    let bytes = bounded_bytes(response).await?;
    let command: GcNodeCommand = serde_json::from_slice(&bytes)?;
    validate_command(&command, node_id)?;

    let result = match command.command.as_str() {
        "generate_plan" => generate_plan(client, base_url, agent_secret, cache_root, command).await,
        "apply_plan" => apply_plan(client, base_url, agent_secret, cache_root, command).await,
        _ => Err(anyhow!("unsupported GC node command")),
    };
    if result.is_ok() {
        super::storage::prune_gc_artifacts(cache_root, 100)?;
    }
    result
}

async fn generate_plan(
    client: &Client,
    base_url: &Url,
    agent_secret: &str,
    cache_root: &Path,
    command: GcNodeCommand,
) -> Result<()> {
    let mut args = vec![
        "gc-plan".into(),
        "-CacheRoot".into(),
        cache_root.as_os_str().into(),
        "-ProjectRoot".into(),
        cache_root.as_os_str().into(),
        "-RequestId".into(),
        command.request_id.clone().into(),
        "-NodeId".into(),
        command.node_id.clone().into(),
    ];
    add_option_switches(&mut args, command.options);
    if !run_cache_tool(cache_root, &args).await.unwrap_or(false) {
        return upload_failure(
            client,
            base_url,
            agent_secret,
            &command,
            "plan",
            "local-plan-failed",
            false,
        )
        .await;
    }
    let local_plan = (|| -> Result<LocalPlan> {
        let plan_path = cache_root
            .join("reports")
            .join("gc")
            .join("plans")
            .join(format!("{}.json", command.request_id));
        let plan: LocalPlan =
            serde_json::from_slice(&std::fs::read(plan_path).context("read local GC plan")?)?;
        validate_plan(&plan.summary, &command)?;
        Ok(plan)
    })();
    let plan = match local_plan {
        Ok(plan) => plan,
        Err(_) => {
            return upload_failure(
                client,
                base_url,
                agent_secret,
                &command,
                "plan",
                "local-plan-invalid",
                false,
            )
            .await;
        }
    };
    let endpoint = gc_endpoint(
        base_url,
        &[
            "api",
            "node",
            "cache-gc",
            &command.node_id,
            &command.request_id,
            "plan",
        ],
    )?;
    let response = client
        .post(endpoint)
        .bearer_auth(agent_secret)
        .json(&serde_json::json!({ "summary": plan.summary }))
        .send()
        .await?;
    require_success(response, "GC plan upload").await
}

async fn apply_plan(
    client: &Client,
    base_url: &Url,
    agent_secret: &str,
    cache_root: &Path,
    command: GcNodeCommand,
) -> Result<()> {
    let plan_id = command
        .plan_id
        .as_deref()
        .ok_or_else(|| anyhow!("approved GC command has no plan ID"))?;
    let plan_digest = command
        .plan_digest
        .as_deref()
        .ok_or_else(|| anyhow!("approved GC command has no plan digest"))?;
    let args = vec![
        "gc-apply-approved".into(),
        "-CacheRoot".into(),
        cache_root.as_os_str().into(),
        "-ProjectRoot".into(),
        cache_root.as_os_str().into(),
        "-RequestId".into(),
        command.request_id.clone().into(),
        "-NodeId".into(),
        command.node_id.clone().into(),
        "-PlanId".into(),
        plan_id.into(),
        "-PlanDigest".into(),
        plan_digest.into(),
    ];
    if !run_cache_tool(cache_root, &args).await.unwrap_or(false) {
        return upload_failure(
            client,
            base_url,
            agent_secret,
            &command,
            "apply",
            "local-apply-refused",
            false,
        )
        .await;
    }
    let local_receipt = (|| -> Result<LocalReceipt> {
        let receipt_path = cache_root
            .join("reports")
            .join("gc")
            .join("receipts")
            .join(format!("{}.json", command.request_id));
        let receipt: LocalReceipt =
            serde_json::from_slice(&std::fs::read(receipt_path).context("read local GC receipt")?)?;
        validate_receipt(&receipt, &command)?;
        Ok(receipt)
    })();
    let receipt = match local_receipt {
        Ok(receipt) => receipt,
        Err(_) => {
            return upload_failure(
                client,
                base_url,
                agent_secret,
                &command,
                "apply",
                "local-receipt-invalid",
                false,
            )
            .await;
        }
    };
    let result = NodeGcResultSummary {
        schema: "elon.rust_cache.gc_result_summary.v1",
        request_id: receipt.request_id,
        node_id: receipt.node_id,
        phase: "apply",
        status: receipt.status,
        plan_id: Some(receipt.plan_id),
        plan_digest: Some(receipt.plan_digest),
        completed_at_utc: receipt.completed_at_utc,
        approved_action_count: receipt.approved_action_count,
        removed_action_count: receipt.removed_action_count,
        reclaimed_bytes: receipt.reclaimed_bytes,
        failure_code: None,
        security: NodeGcResultSecurity {
            absolute_paths_included: false,
            secrets_included: false,
            execution_bound_to_plan_digest: true,
            local_rescan_completed: true,
        },
    };
    upload_result(client, base_url, agent_secret, &command, &result).await
}

async fn upload_failure(
    client: &Client,
    base_url: &Url,
    agent_secret: &str,
    command: &GcNodeCommand,
    phase: &'static str,
    code: &'static str,
    local_rescan_completed: bool,
) -> Result<()> {
    let result = NodeGcResultSummary {
        schema: "elon.rust_cache.gc_result_summary.v1",
        request_id: command.request_id.clone(),
        node_id: command.node_id.clone(),
        phase,
        status: "failed".into(),
        plan_id: command.plan_id.clone(),
        plan_digest: command.plan_digest.clone(),
        completed_at_utc: Utc::now().to_rfc3339(),
        approved_action_count: 0,
        removed_action_count: 0,
        reclaimed_bytes: 0,
        failure_code: Some(code),
        security: NodeGcResultSecurity {
            absolute_paths_included: false,
            secrets_included: false,
            execution_bound_to_plan_digest: phase == "apply",
            local_rescan_completed,
        },
    };
    upload_result(client, base_url, agent_secret, command, &result).await
}

async fn upload_result(
    client: &Client,
    base_url: &Url,
    agent_secret: &str,
    command: &GcNodeCommand,
    result: &NodeGcResultSummary,
) -> Result<()> {
    let endpoint = gc_endpoint(
        base_url,
        &[
            "api",
            "node",
            "cache-gc",
            &command.node_id,
            &command.request_id,
            "result",
        ],
    )?;
    let response = client
        .post(endpoint)
        .bearer_auth(agent_secret)
        .json(result)
        .send()
        .await?;
    require_success(response, "GC result upload").await
}

async fn run_cache_tool(cache_root: &Path, args: &[std::ffi::OsString]) -> Result<bool> {
    let entry = cache_root.join("platform").join("rust-cache.ps1");
    if !entry.is_file() {
        return Err(anyhow!("installed Rust cache entry is missing"));
    }
    let mut command = if cfg!(windows) {
        Command::new("powershell.exe")
    } else {
        Command::new("pwsh")
    };
    if cfg!(windows) {
        command.arg("-WindowStyle").arg("Hidden");
    }
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(entry);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    hide_command_window(&mut command);
    let status = tokio::time::timeout(Duration::from_secs(TOOL_TIMEOUT_SECONDS), command.status())
        .await
        .map_err(|_| anyhow!("Rust cache GC tool timed out"))??;
    Ok(status.success())
}

fn add_option_switches(args: &mut Vec<std::ffi::OsString>, options: GcOptions) {
    if options.force_aged {
        args.push("-ForceAged".into());
    }
    if options.workspace_only {
        args.push("-WorkspaceOnly".into());
    }
    if options.recover_missing_workspaces {
        args.push("-RecoverMissingWorkspaces".into());
    }
    if options.shared_aliases_only {
        args.push("-SharedAliasesOnly".into());
    }
}

fn validate_command(command: &GcNodeCommand, node_id: &str) -> Result<()> {
    if command.schema != "elon.rust_cache.gc_node_command.v1"
        || command.node_id != node_id
        || !valid_hex(&command.request_id, 32)
        || !matches!(command.command.as_str(), "generate_plan" | "apply_plan")
        || command.options.workspace_only
        || command.options.recover_missing_workspaces
        || command.options.shared_aliases_only
        || chrono::DateTime::parse_from_rfc3339(&command.expires_at)? <= Utc::now()
    {
        return Err(anyhow!("invalid or expired GC node command"));
    }
    if command.command == "apply_plan"
        && (!command
            .plan_id
            .as_deref()
            .is_some_and(|id| valid_hex(id, 32))
            || !command
                .plan_digest
                .as_deref()
                .is_some_and(|digest| valid_hex(digest, 64)))
    {
        return Err(anyhow!("approved GC command identity is invalid"));
    }
    Ok(())
}

fn validate_plan(summary: &GcPlanSummary, command: &GcNodeCommand) -> Result<()> {
    if summary.schema != "elon.rust_cache.gc_plan_summary.v1"
        || summary.request_id != command.request_id
        || summary.node_id != command.node_id
        || summary.options != command.options
        || !valid_hex(&summary.plan_id, 32)
        || !valid_hex(&summary.plan_digest, 64)
        || summary.security.absolute_paths_included
        || summary.security.secrets_included
        || summary.security.destructive_actions_authorized
        || !summary.security.approval_binds_plan_digest
        || !summary.security.target_rescan_required
    {
        return Err(anyhow!("local GC plan summary contract mismatch"));
    }
    Ok(())
}

fn validate_receipt(receipt: &LocalReceipt, command: &GcNodeCommand) -> Result<()> {
    if receipt.schema != "elon.rust_cache.gc_receipt.v1"
        || receipt.request_id != command.request_id
        || receipt.node_id != command.node_id
        || receipt.plan_id.as_str() != command.plan_id.as_deref().unwrap_or_default()
        || receipt.plan_digest.as_str() != command.plan_digest.as_deref().unwrap_or_default()
        || !matches!(receipt.status.as_str(), "completed" | "partial")
        || receipt.security.absolute_paths_included
        || receipt.security.secrets_included
        || !receipt.security.execution_bound_to_plan_digest
        || !receipt.security.local_rescan_completed
    {
        return Err(anyhow!("local GC receipt contract mismatch"));
    }
    Ok(())
}

fn gc_endpoint(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut endpoint = base_url.clone();
    endpoint.set_query(None);
    endpoint.set_fragment(None);
    endpoint.set_path("/");
    endpoint
        .path_segments_mut()
        .map_err(|_| anyhow!("server URL cannot be a base"))?
        .extend(segments);
    Ok(endpoint)
}

async fn bounded_bytes(response: reqwest::Response) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(anyhow!("GC response is too large"));
    }
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(anyhow!("GC response is too large"));
    }
    Ok(bytes.to_vec())
}

async fn require_success(response: reqwest::Response, operation: &str) -> Result<()> {
    if !response.status().is_success() {
        return Err(anyhow!("{operation} rejected: {}", response.status()));
    }
    let _ = bounded_bytes(response).await?;
    Ok(())
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value == value.to_ascii_lowercase()
        && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[cfg(windows)]
fn hide_command_window(command: &mut Command) {
    command.creation_flags(0x0800_0000 | 0x0000_0200);
}

#[cfg(not(windows))]
fn hide_command_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_contract_rejects_wrong_node_and_unbound_apply() {
        let command = GcNodeCommand {
            schema: "elon.rust_cache.gc_node_command.v1".into(),
            command: "apply_plan".into(),
            request_id: "a".repeat(32),
            node_id: "node-a".into(),
            expires_at: (Utc::now() + chrono::Duration::minutes(5)).to_rfc3339(),
            options: GcOptions {
                force_aged: false,
                workspace_only: false,
                recover_missing_workspaces: false,
                shared_aliases_only: false,
            },
            plan_id: None,
            plan_digest: None,
        };
        assert!(validate_command(&command, "node-a").is_err());
        assert!(validate_command(&command, "node-b").is_err());

        let project_specific = GcNodeCommand {
            plan_id: None,
            plan_digest: None,
            command: "generate_plan".into(),
            options: GcOptions {
                force_aged: false,
                workspace_only: false,
                recover_missing_workspaces: false,
                shared_aliases_only: true,
            },
            ..command
        };
        assert!(validate_command(&project_specific, "node-a").is_err());
    }
}
