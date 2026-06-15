use std::{path::PathBuf, sync::Arc};

use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    agent_llm_call::call_chat_llm_with_options,
    types::{AgentConfig, AppState},
};

use super::{
    config::ContextCompilerConfig,
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_repair_attempt::{
        PatchRepairAttemptStatus, SymbolPatchRepairAttemptResponse,
        build_symbol_patch_repair_attempt_response,
    },
    symbol_index_patch_verification_run::run_symbol_patch_verification,
    symbol_index_patch_verification_run_types::SymbolPatchVerificationRunResponse,
};

const PROMPT_EXCERPT_LIMIT: usize = 8_000;
const PATCH_EXCERPT_LIMIT: usize = 6_000;
const MODEL_OUTPUT_EXCERPT_LIMIT: usize = 6_000;
const DEFAULT_REPAIR_MAX_TOKENS: usize = 2_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchRepairGenerateResponse {
    pub(crate) task: String,
    pub(crate) status: PatchRepairGenerationStatus,
    pub(crate) agent: String,
    pub(crate) model: String,
    pub(crate) attempt: usize,
    pub(crate) max_attempts: usize,
    pub(crate) original_verification: SymbolPatchVerificationRunResponse,
    pub(crate) llm_output: PatchRepairGeneratorOutput,
    pub(crate) repair_attempt: Option<SymbolPatchRepairAttemptResponse>,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchRepairGenerationStatus {
    RepairNotNeeded,
    OriginalPatchNotReadyForRepair,
    ModelOutputInvalid,
    GeneratedPatchVerified,
    GeneratedPatchNeedsManualVerification,
    GeneratedPatchRejected,
    GeneratedPatchStillFailing,
    AttemptsExhausted,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchRepairGeneratorOutput {
    pub(crate) extracted: bool,
    pub(crate) repair_patch: Option<String>,
    pub(crate) raw_output_excerpt: String,
    pub(crate) error: Option<String>,
}

pub(crate) async fn generate_symbol_patch_repair(
    state: &Arc<AppState>,
    generation: SymbolPatchGeneration,
    original_patch: String,
    workspace: PathBuf,
    user_id: String,
    attempt: Option<usize>,
    max_attempts: Option<usize>,
) -> Result<SymbolPatchRepairGenerateResponse, String> {
    let attempt = attempt.unwrap_or(1).max(1);
    let max_attempts = max_attempts.unwrap_or(2).clamp(1, 2);
    let original_verification = verify_original_patch(
        generation.clone(),
        original_patch.clone(),
        workspace.clone(),
    )
    .await?;

    let config = ContextCompilerConfig::from_env();
    let agent_name = config.agent_name.clone();

    if !original_verification
        .verification_repair_context
        .model_repair_required
    {
        let status = if original_verification.execution.patch_applied {
            PatchRepairGenerationStatus::RepairNotNeeded
        } else {
            PatchRepairGenerationStatus::OriginalPatchNotReadyForRepair
        };
        return Ok(SymbolPatchRepairGenerateResponse {
            task: generation.task.clone(),
            status,
            agent: agent_name,
            model: "not_required".to_string(),
            attempt,
            max_attempts,
            original_verification,
            llm_output: PatchRepairGeneratorOutput {
                extracted: false,
                repair_patch: None,
                raw_output_excerpt: String::new(),
                error: Some("original_patch_did_not_request_model_repair".to_string()),
            },
            repair_attempt: None,
            next_steps: vec![
                "Use patch-verify-run output directly; no model repair was requested.".to_string(),
            ],
        });
    }

    let agent = resolve_repair_agent(state, &config).await?;
    let model = agent.model.clone();

    let messages = build_repair_generation_messages(
        &generation,
        &original_patch,
        original_verification
            .verification_repair_context
            .retry_prompt
            .as_deref(),
        attempt,
        max_attempts,
    );
    let response = call_chat_llm_with_options(
        state,
        &agent,
        &messages,
        &user_id,
        "context_compiler_patch_repair",
        0.2,
        repair_max_tokens(),
    )
    .await
    .map_err(|error| error.to_string())?;
    let raw_output = extract_message_content(&response)
        .ok_or_else(|| "model_response_missing_message_content".to_string())?;

    let Some(repair_patch) = extract_repair_patch_from_model_output(&raw_output) else {
        return Ok(SymbolPatchRepairGenerateResponse {
            task: generation.task.clone(),
            status: PatchRepairGenerationStatus::ModelOutputInvalid,
            agent: agent_name,
            model,
            attempt,
            max_attempts,
            original_verification,
            llm_output: PatchRepairGeneratorOutput {
                extracted: false,
                repair_patch: None,
                raw_output_excerpt: truncate_text(&raw_output, MODEL_OUTPUT_EXCERPT_LIMIT),
                error: Some("model_output_did_not_contain_unified_diff".to_string()),
            },
            repair_attempt: None,
            next_steps: vec![
                "Regenerate the repair patch and return a unified diff only.".to_string(),
            ],
        });
    };

    let repair_attempt = verify_generated_repair_patch(
        generation.clone(),
        original_patch,
        repair_patch.clone(),
        workspace,
        attempt,
        max_attempts,
    )
    .await?;
    let status = generation_status_from_attempt(&repair_attempt.status);
    let next_steps = generation_next_steps(status);

    Ok(SymbolPatchRepairGenerateResponse {
        task: generation.task.clone(),
        status,
        agent: agent_name,
        model,
        attempt,
        max_attempts,
        original_verification,
        llm_output: PatchRepairGeneratorOutput {
            extracted: true,
            repair_patch: Some(repair_patch),
            raw_output_excerpt: truncate_text(&raw_output, MODEL_OUTPUT_EXCERPT_LIMIT),
            error: None,
        },
        repair_attempt: Some(repair_attempt),
        next_steps,
    })
}

pub(crate) fn build_repair_generation_messages(
    generation: &SymbolPatchGeneration,
    original_patch: &str,
    repair_context_prompt: Option<&str>,
    attempt: usize,
    max_attempts: usize,
) -> Vec<Value> {
    let retry_prompt = repair_context_prompt.unwrap_or("No retry prompt was generated.");
    let allowed_files = bullet_list(&generation.diff_contract.allowed_files);
    let forbidden_patterns = bullet_list(&generation.diff_contract.forbidden_patterns);
    let generation_prompt = truncate_text(&generation.prompt, PROMPT_EXCERPT_LIMIT);
    let original_patch = truncate_text(original_patch, PATCH_EXCERPT_LIMIT);
    let retry_prompt = truncate_text(retry_prompt, PROMPT_EXCERPT_LIMIT);

    vec![
        json!({
            "role": "system",
            "content": "You are a strict code patch repair generator. Return only an incremental unified diff. Do not include markdown fences, explanations, or files outside allowed_files."
        }),
        json!({
            "role": "user",
            "content": format!(
                "<patch_repair_generation>\n\
        Task:\n{task}\n\n\
        Attempt: {attempt}/{max_attempts}\n\n\
        Allowed files:\n{allowed_files}\n\n\
        Forbidden patterns:\n{forbidden_patterns}\n\n\
        Patch generation plan excerpt:\n{generation_prompt}\n\n\
        Original patch excerpt:\n```diff\n{original_patch}\n```\n\n\
        Verification repair context:\n{retry_prompt}\n\n\
        Output rules:\n\
        - Return an incremental unified diff only.\n\
        - The repair diff must apply after the original patch.\n\
        - Touch only allowed_files.\n\
        - Do not undo unrelated successful changes.\n\
        - Prefer the smallest repair that fixes the failed verification.\n\
        </patch_repair_generation>",
                task = generation.task,
            )
        }),
    ]
}

pub(crate) fn extract_repair_patch_from_model_output(raw_output: &str) -> Option<String> {
    let fenced = extract_fenced_diff(raw_output);
    let candidate = fenced.as_deref().unwrap_or(raw_output);
    let candidate = strip_to_first_diff(candidate)?;
    is_unified_diff(&candidate).then_some(candidate)
}

async fn verify_original_patch(
    generation: SymbolPatchGeneration,
    original_patch: String,
    workspace: PathBuf,
) -> Result<SymbolPatchVerificationRunResponse, String> {
    tokio::task::spawn_blocking(move || {
        run_symbol_patch_verification(&generation, &original_patch, &workspace)
    })
    .await
    .map_err(|error| error.to_string())
}

async fn verify_generated_repair_patch(
    generation: SymbolPatchGeneration,
    original_patch: String,
    repair_patch: String,
    workspace: PathBuf,
    attempt: usize,
    max_attempts: usize,
) -> Result<SymbolPatchRepairAttemptResponse, String> {
    tokio::task::spawn_blocking(move || {
        build_symbol_patch_repair_attempt_response(
            &generation,
            &original_patch,
            &repair_patch,
            &workspace,
            Some(attempt),
            Some(max_attempts),
        )
    })
    .await
    .map_err(|error| error.to_string())
}

async fn resolve_repair_agent(
    state: &Arc<AppState>,
    config: &ContextCompilerConfig,
) -> Result<AgentConfig, String> {
    let agents = state.agents_config.read().await;
    agents
        .get_agent(Some(&config.agent_name))
        .cloned()
        .ok_or_else(|| format!("repair agent '{}' not found", config.agent_name))
}

fn generation_status_from_attempt(
    status: &PatchRepairAttemptStatus,
) -> PatchRepairGenerationStatus {
    match status {
        PatchRepairAttemptStatus::RepairedPatchPassed => {
            PatchRepairGenerationStatus::GeneratedPatchVerified
        }
        PatchRepairAttemptStatus::ManualVerificationRequired => {
            PatchRepairGenerationStatus::GeneratedPatchNeedsManualVerification
        }
        PatchRepairAttemptStatus::AttemptRejected => {
            PatchRepairGenerationStatus::GeneratedPatchRejected
        }
        PatchRepairAttemptStatus::AttemptsExhausted => {
            PatchRepairGenerationStatus::AttemptsExhausted
        }
        PatchRepairAttemptStatus::RepairStillFailing => {
            PatchRepairGenerationStatus::GeneratedPatchStillFailing
        }
        PatchRepairAttemptStatus::OriginalPatchNotReadyForRepair => {
            PatchRepairGenerationStatus::OriginalPatchNotReadyForRepair
        }
    }
}

fn generation_next_steps(status: PatchRepairGenerationStatus) -> Vec<String> {
    match status {
        PatchRepairGenerationStatus::GeneratedPatchVerified => vec![
            "Model-generated repair patch passed isolated verification.".to_string(),
            "Use repairAttempt.repairResult.combinedPatchSha256 to track the final patch stream."
                .to_string(),
        ],
        PatchRepairGenerationStatus::GeneratedPatchNeedsManualVerification => vec![
            "Auto-runnable verification passed, but manual verification remains.".to_string(),
            "Run skipped commands before applying the repaired patch.".to_string(),
        ],
        PatchRepairGenerationStatus::GeneratedPatchRejected => vec![
            "Model-generated repair patch violated the patch contract.".to_string(),
            "Regenerate with stricter allowed_files and forbidden_patterns.".to_string(),
        ],
        PatchRepairGenerationStatus::GeneratedPatchStillFailing => vec![
            "Model-generated repair patch did not fix verification.".to_string(),
            "Use repairAttempt.repairedVerification.verificationRepairContext for the next attempt."
                .to_string(),
        ],
        PatchRepairGenerationStatus::AttemptsExhausted => vec![
            "Repair verification failed and max_attempts has been reached.".to_string(),
            "Return the full repairAttempt report to an operator.".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn extract_message_content(response: &Value) -> Option<String> {
    response
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
}

fn extract_fenced_diff(raw_output: &str) -> Option<String> {
    let mut in_fence = false;
    let mut fence_buffer = Vec::new();
    for line in raw_output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_fence {
                let candidate = fence_buffer.join("\n");
                if is_unified_diff(&candidate) {
                    return Some(candidate);
                }
                fence_buffer.clear();
                in_fence = false;
            } else {
                in_fence = true;
                fence_buffer.clear();
            }
            continue;
        }
        if in_fence {
            fence_buffer.push(line);
        }
    }
    None
}

fn strip_to_first_diff(text: &str) -> Option<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|line| {
        line.starts_with("diff --git ") || line.starts_with("--- a/") || line.starts_with("--- /")
    })?;
    let mut out = lines[start..].join("\n");
    out = out.trim().trim_matches('`').trim().to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Some(out)
}

fn is_unified_diff(text: &str) -> bool {
    let trimmed = text.trim();
    (trimmed.contains("diff --git ") || trimmed.contains("--- "))
        && trimmed.contains("+++ ")
        && trimmed.contains("@@")
}

fn bullet_list(values: &[String]) -> String {
    if values.is_empty() {
        return "- none".to_string();
    }
    values
        .iter()
        .map(|value| format!("- {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn repair_max_tokens() -> usize {
    std::env::var("ELON_PATCH_REPAIR_MAX_TOKENS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_REPAIR_MAX_TOKENS)
        .clamp(512, 8_000)
}

fn truncate_text(text: &str, limit: usize) -> String {
    let mut out = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= limit {
            out.push_str("...<truncated>");
            return out;
        }
        out.push(ch);
    }
    out
}
