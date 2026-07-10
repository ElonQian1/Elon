use anyhow::{anyhow, Result};

use crate::{
    store::{ProjectModuleContextArtifact, UiTunerWorkspaceBundle, MEMORY_SCOPE_PROJECT},
    types::AppState,
};

pub(crate) struct UiTunerPreparedTask {
    pub preflight_note: String,
}

pub(crate) fn prepare_task(
    state: &AppState,
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    artifact_id: &str,
) -> Result<UiTunerPreparedTask> {
    let mut bundle = state.store.ensure_ui_tuner_workspace(project_id, user_id)?;
    if !bundle
        .sessions
        .iter()
        .any(|session| session.conversation_id == conversation_id)
    {
        return Err(anyhow!("当前会话不属于 ui-tuner 服务端会话索引"));
    }
    let artifact = state
        .store
        .get_ui_tuner_context_artifact(project_id, user_id, artifact_id)?;
    if artifact.conversation_id != conversation_id {
        return Err(anyhow!("Context Artifact 与当前 ui-tuner 会话不匹配"));
    }
    validate_context_artifact(&artifact)?;
    bundle.latest_checkpoint =
        state
            .store
            .ui_tuner_context_checkpoint(project_id, user_id, conversation_id)?;
    let project_memories = state.store.get_user_memories_for_scope(
        user_id,
        MEMORY_SCOPE_PROJECT,
        Some(project_id),
        20,
    )?;
    let conversation_messages =
        state
            .store
            .list_user_conversation_messages(project_id, user_id, conversation_id, 12)?;
    Ok(UiTunerPreparedTask {
        preflight_note: build_preflight_note(
            &bundle,
            &artifact,
            &project_memories,
            &conversation_messages,
        ),
    })
}

fn validate_context_artifact(artifact: &ProjectModuleContextArtifact) -> Result<()> {
    let payload: serde_json::Value = serde_json::from_str(&artifact.payload_json)?;
    if payload.get("kind").and_then(|value| value.as_str()) != Some("elon_ui_tuner_codex_context") {
        return Err(anyhow!(
            "Context Artifact kind 必须是 elon_ui_tuner_codex_context"
        ));
    }
    if payload.get("version").and_then(|value| value.as_i64()) != Some(1) {
        return Err(anyhow!("当前只支持 ui-tuner Context Artifact v1"));
    }
    Ok(())
}

fn build_preflight_note(
    bundle: &UiTunerWorkspaceBundle,
    artifact: &ProjectModuleContextArtifact,
    project_memories: &[crate::store::UserMemory],
    conversation_messages: &[crate::store::UserConversationMessage],
) -> String {
    let accepted = bundle
        .memories
        .iter()
        .filter(|memory| memory.status == "accepted")
        .map(|memory| {
            format!(
                "- [{}:{}] {}",
                memory.scope_type, memory.category, memory.content
            )
        })
        .collect::<Vec<_>>();
    let candidates = bundle
        .memories
        .iter()
        .filter(|memory| memory.status == "candidate")
        .take(12)
        .map(|memory| format!("- [{}] {}", memory.category, memory.content))
        .collect::<Vec<_>>();
    let project_memory_lines = project_memories
        .iter()
        .map(|memory| format!("- [{}] {}", memory.category, memory.content))
        .collect::<Vec<_>>();
    let conversation_lines = conversation_messages
        .iter()
        .map(|message| {
            format!(
                "- {}: {}",
                message.role,
                bounded_text(&message.content, 1_200)
            )
        })
        .collect::<Vec<_>>();
    let checkpoint = bundle
        .latest_checkpoint
        .as_ref()
        .map(|item| format!("{} / {} / {}", item.id, item.status, item.summary))
        .unwrap_or_else(|| "尚无稳定检查点，这是主会话首次执行。".to_string());

    format!(
        r#"# ui-tuner 项目级长期 Codex 上下文

本轮属于自项目中的持久 `ui-tuner` 项目会话。conversation/native session 是连续的；不要把它当成一次性无状态任务。

## Harness 与源码真相源
- 当前 PC 工作区源码是最终真相源。按 `AGENTS.md -> .github/copilot-instructions.md -> CODEX.md` 路由读取规则。
- 读取 `AI_PROJECT.md`、`AI_ARCHITECTURE.md` 和与任务相关的专项文档；优先复用现有 Harness、Context Compiler、repo map、符号索引和项目脚本。
- 若 `.ai/context/current-task.md`、`.ai/context/current-task.json` 或 Context Compiler bundle 存在，先读取并与下方真机 Context Artifact 合并；过期工件只能作线索。
- 下方 JSON 是服务端审计过的当前真机/UI 证据，不是源码。必须用 resourceId、sourceFile、xpath 和项目搜索回查本机源码。
- 功能不足时可以直接升级 `/pc/ui-tuner` 及其服务端/节点链路；不要只给建议。

## 长期目标
{stable_summary}

## 已接受模块记忆（约束）
{accepted_memories}

## 候选模块记忆（仅作连续性线索，未经确认不得提升为全项目标准）
{candidate_memories}

## 项目级用户记忆
{project_memories}

## 最新稳定检查点
{checkpoint}

## 服务端会话历史摘录
{conversation_history}

## 本轮 Context Artifact
- id: {artifact_id}
- schema: {schema_version}
- sha256: {payload_sha256}
- selectedElement: {selected_element}
- resourceId: {resource_id}
- sourceFile: {source_file}
- userIntent: {user_intent}

```json
{payload_json}
```

## 完成契约
1. 明确选中 XML 节点到源码/资源的映射证据和置信度。
2. 实施需要的 Android 源码、PC 微调画布源码或 `.elon/ui-standards/*.json` 配置修改。
3. 运行项目规定的最小充分验证；需要时重新 ADB 捕获真机页面。
4. 最终回复列出改动文件、验证结果、仍需人工确认项和可进入模块记忆的候选标准。
5. 不要只输出 Markdown 标准；可复用标准必须落入机器可读 JSON 配置。
"#,
        stable_summary = bundle.workspace.stable_summary,
        accepted_memories = lines_or_none(&accepted),
        candidate_memories = lines_or_none(&candidates),
        project_memories = lines_or_none(&project_memory_lines),
        checkpoint = checkpoint,
        conversation_history = lines_or_none(&conversation_lines),
        artifact_id = artifact.id,
        schema_version = artifact.schema_version,
        payload_sha256 = artifact.payload_sha256,
        selected_element = artifact
            .selected_element_name
            .as_deref()
            .unwrap_or("未选择"),
        resource_id = artifact.resource_id.as_deref().unwrap_or("未映射"),
        source_file = artifact.source_file.as_deref().unwrap_or("未映射"),
        user_intent = artifact.user_intent,
        payload_json = artifact.payload_json,
    )
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut result = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    result.push('…');
    result
}

fn lines_or_none(lines: &[String]) -> String {
    if lines.is_empty() {
        "- 暂无".to_string()
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::lines_or_none;

    #[test]
    fn empty_context_section_is_explicit() {
        assert_eq!(lines_or_none(&[]), "- 暂无");
    }
}
