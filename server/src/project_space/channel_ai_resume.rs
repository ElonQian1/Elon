use axum::http::StatusCode;

use crate::{store::Store, types::AppState};

const RESUME_PROMPT: &str = "继续上一轮任务。先恢复原会话并检查当前工作区与任务快照，从中断处继续；不要重复已经完成的提交、发布或其他有副作用的步骤。";
const RESUME_DISPLAY: &str = "继续上一轮任务";

pub(super) struct ResolvedChannelAiTaskStart {
    pub(super) content: String,
    pub(super) display_content: String,
    pub(super) conversation_id: Option<String>,
    pub(super) source_task_id: Option<String>,
}

pub(super) fn resolve_channel_ai_task_start(
    state: &AppState,
    project_id: &str,
    channel_id: &str,
    user_id: &str,
    requested_content: &str,
    requested_conversation_id: Option<&str>,
    resume_task_id: Option<&str>,
) -> Result<ResolvedChannelAiTaskStart, (StatusCode, String)> {
    let Some(source_task_id) = resume_task_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(ResolvedChannelAiTaskStart {
            content: requested_content.to_string(),
            display_content: requested_content.to_string(),
            conversation_id: requested_conversation_id.map(str::to_string),
            source_task_id: None,
        });
    };

    let source = load_resume_source(&state.store, project_id, channel_id, source_task_id)?;
    validate_resume_source(&source, user_id)?;
    Ok(ResolvedChannelAiTaskStart {
        content: RESUME_PROMPT.to_string(),
        display_content: RESUME_DISPLAY.to_string(),
        conversation_id: source
            .conversation_id
            .or_else(|| requested_conversation_id.map(str::to_string)),
        source_task_id: Some(source_task_id.to_string()),
    })
}

struct ResumeTaskSource {
    user_id: String,
    conversation_id: Option<String>,
    status: String,
}

fn load_resume_source(
    store: &Store,
    project_id: &str,
    channel_id: &str,
    source_task_id: &str,
) -> Result<ResumeTaskSource, (StatusCode, String)> {
    let snapshot = store
        .get_channel_task_snapshot(project_id, channel_id, source_task_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "找不到要继续的原任务".to_string()))?;
    Ok(ResumeTaskSource {
        user_id: snapshot.user_id,
        conversation_id: snapshot.conversation_id,
        status: snapshot.status,
    })
}

fn validate_resume_source(
    source: &ResumeTaskSource,
    user_id: &str,
) -> Result<(), (StatusCode, String)> {
    if source.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "只能继续自己发起的任务".to_string()));
    }
    let status = source.status.trim().to_ascii_lowercase();
    if matches!(status.as_str(), "running" | "queued" | "recovering") {
        return Err((
            StatusCode::CONFLICT,
            "原任务仍在运行，无需重复继续".to_string(),
        ));
    }
    if matches!(
        status.as_str(),
        "done" | "completed" | "success" | "succeeded"
    ) {
        return Err((StatusCode::CONFLICT, "原任务已经完成".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(status: &str) -> ResumeTaskSource {
        ResumeTaskSource {
            user_id: "usr-owner".to_string(),
            conversation_id: Some("conv-original".to_string()),
            status: status.to_string(),
        }
    }

    #[test]
    fn failed_and_canceled_tasks_can_resume_without_repeating_the_prompt() {
        assert!(validate_resume_source(&source("failed"), "usr-owner").is_ok());
        assert!(validate_resume_source(&source("canceled"), "usr-owner").is_ok());
        assert!(!RESUME_PROMPT.contains("原始请求"));
    }

    #[test]
    fn active_or_completed_tasks_cannot_be_started_twice() {
        assert_eq!(
            validate_resume_source(&source("running"), "usr-owner")
                .expect_err("running task must be rejected")
                .0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            validate_resume_source(&source("done"), "usr-owner")
                .expect_err("completed task must be rejected")
                .0,
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn another_user_cannot_resume_the_task() {
        assert_eq!(
            validate_resume_source(&source("failed"), "usr-other")
                .expect_err("foreign task must be rejected")
                .0,
            StatusCode::FORBIDDEN
        );
    }
}
