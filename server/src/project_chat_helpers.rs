// server/src/project_chat_helpers.rs
//! project_chat_runner 辅助函数

use std::path::Path;
use std::sync::Arc;

use crate::{
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_auth::can_edit,
    project_ws_protocol::ProjectAttachmentRef,
    store::ProjectAccess,
    types::AppState,
};

pub(crate) fn append_project_icon_context(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
    message: String,
    project_icon_data_url: Option<&str>,
) -> String {
    let Some(icon_data_url) = clean_project_icon_context_data_url(project_icon_data_url) else {
        return message;
    };
    if can_edit(&project.role) {
        let _ = state
            .store
            .set_project_icon_data_url(&project.id, Some(&icon_data_url));
    }
    let wrote_metadata = write_project_icon_metadata(workspace, project, &icon_data_url);
    let note = if wrote_metadata {
        "用户已上传这个项目的 APK 图标。图标元数据已写入 `.elon/project-icon.json`；后续生成、修改或打包 Android APK 时，必须读取该文件并把其中的 `icon_data_url` 用作 launcher icon（含 `android:icon` / `android:roundIcon` / adaptive icon），应用内所有展示该用户 APK 的位置也使用同一图标。".to_string()
    } else {
        format!(
            "用户已上传这个项目的 APK 图标。后续生成、修改或打包 Android APK 时，必须把下面的 `icon_data_url` 用作 launcher icon（含 `android:icon` / `android:roundIcon` / adaptive icon），应用内所有展示该用户 APK 的位置也使用同一图标。\n\nicon_data_url:\n{}",
            icon_data_url
        )
    };
    format!("{message}\n\n[项目 APK 图标]\n{note}")
}

pub(crate) fn should_append_project_icon_context_for_pc_fast_path(needs_project_workflow: bool) -> bool {
    needs_project_workflow
}

pub(crate) fn should_use_pc_node_fast_path(
    is_pc_node_project: bool,
    needs_project_workflow: bool,
    direct_pc_cli_enabled: bool,
    _pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> bool {
    is_pc_node_project && (needs_project_workflow || direct_pc_cli_enabled)
}

pub(crate) const MAX_PROJECT_ICON_CONTEXT_DATA_URL_BYTES: usize = 512 * 1024;

pub(crate) fn looks_like_replaced_unicode_mojibake(message: &str) -> bool {
    let mut total = 0usize;
    let mut question_marks = 0usize;
    let mut replacement_chars = 0usize;
    let mut cjk = 0usize;

    for ch in message.chars() {
        if ch.is_whitespace() {
            continue;
        }
        total += 1;
        match ch {
            '?' => question_marks += 1,
            '\u{FFFD}' => replacement_chars += 1,
            '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
            | '\u{F900}'..='\u{FAFF}' => cjk += 1,
            _ => {}
        }
    }

    if total < 40 || cjk > 0 {
        return false;
    }
    let damaged = question_marks + replacement_chars;
    damaged >= 12 && damaged * 100 >= total * 20
}

#[cfg(test)]
mod tests {
    use super::{
        looks_like_replaced_unicode_mojibake, should_append_project_icon_context_for_pc_fast_path,
        should_use_pc_node_fast_path,
    };
    use crate::pc_agent_runtime_choice::PcRuntimeRoutePreference;

    #[test]
    fn detects_windows_question_mark_mojibake() {
        let message = "?????????? Win ? Codex ????????????????????????????????\n\
            1. ?? AGENTS.md ? .github/copilot-instructions.md???????????\n\
            2. ?? server/src/git_command_error.rs?server/src/node_agent_main.rs?";

        assert!(looks_like_replaced_unicode_mojibake(message));
    }

    #[test]
    fn allows_normal_chinese_and_question_marks() {
        assert!(!looks_like_replaced_unicode_mojibake(
            "这是一次 Win 端 Codex 产品链路实测，请读取项目源码并运行 git status？"
        ));
        assert!(!looks_like_replaced_unicode_mojibake(
            "Why??? Can Codex read AGENTS.md and run cargo check?"
        ));
    }

    #[test]
    fn pc_node_fast_path_keeps_lightweight_chat_message_plain() {
        assert!(!should_append_project_icon_context_for_pc_fast_path(false));
        assert!(should_append_project_icon_context_for_pc_fast_path(true));
    }

    #[test]
    fn pc_node_fast_path_skips_default_platform_chat() {
        assert!(!should_use_pc_node_fast_path(true, false, false, None));
        assert!(!should_use_pc_node_fast_path(
            true,
            false,
            false,
            Some(PcRuntimeRoutePreference::RouteC)
        ));
        assert!(!should_use_pc_node_fast_path(
            true,
            false,
            false,
            Some(PcRuntimeRoutePreference::RouteC3)
        ));
    }

    #[test]
    fn pc_node_fast_path_keeps_development_and_explicit_pc_routes() {
        assert!(should_use_pc_node_fast_path(true, true, false, None));
        assert!(should_use_pc_node_fast_path(true, false, true, None));
        assert!(should_use_pc_node_fast_path(
            true,
            true,
            false,
            Some(PcRuntimeRoutePreference::RouteC3)
        ));
    }
}

pub(crate) fn clean_project_icon_context_data_url(project_icon_data_url: Option<&str>) -> Option<String> {
    let value = project_icon_data_url?.trim();
    if value.is_empty() || value.len() > MAX_PROJECT_ICON_CONTEXT_DATA_URL_BYTES {
        return None;
    }
    if !value.starts_with("data:image/") || !value.contains(";base64,") {
        return None;
    }
    Some(value.to_string())
}

pub(crate) fn write_project_icon_metadata(
    workspace: &Path,
    project: &ProjectAccess,
    icon_data_url: &str,
) -> bool {
    let dir = workspace.join(".elon");
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let payload = serde_json::json!({
        "project_id": &project.id,
        "project_name": &project.name,
        "icon_data_url": icon_data_url,
        "usage": "Use this image as the Android APK launcher icon, including android:icon, android:roundIcon, adaptive icon foreground/background if present, and all in-app surfaces that represent this user APK."
    });
    serde_json::to_string_pretty(&payload)
        .ok()
        .and_then(|json| std::fs::write(dir.join("project-icon.json"), json).ok())
        .is_some()
}



