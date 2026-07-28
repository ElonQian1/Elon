use std::sync::Arc;

use tracing::warn;

use crate::{
    store::{ProjectAccess, ProjectDevProfile},
    types::AppState,
};

pub(super) fn append_project_dev_profile_context(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    user_message: &str,
) -> String {
    let profile = match state
        .store
        .get_project_dev_profile_for_user(user_id, &project.id)
    {
        Ok(Some(profile)) if !profile.is_empty() => profile,
        Ok(_) => return user_message.to_string(),
        Err(error) => {
            warn!(
                project_id = %project.id,
                "读取项目开发命令 profile 失败，继续使用原始用户消息: {error}"
            );
            return user_message.to_string();
        }
    };
    format!(
        "{user_message}\n\n{}",
        project_dev_profile_prompt_block(&profile)
    )
}

fn project_dev_profile_prompt_block(profile: &ProjectDevProfile) -> String {
    let mut lines = vec![
        "系统自动识别的本地项目开发命令；执行 run/test/build 时优先参考，除非仓库文档给出更明确命令。".to_string(),
        "<project_dev_profile>".to_string(),
    ];
    push_profile_line(&mut lines, "project_type", profile.project_type.as_deref());
    push_profile_line(
        &mut lines,
        "package_manager",
        profile.package_manager.as_deref(),
    );
    push_profile_line(&mut lines, "run_command", profile.run_command.as_deref());
    push_profile_line(&mut lines, "test_command", profile.test_command.as_deref());
    push_profile_line(
        &mut lines,
        "build_command",
        profile.build_command.as_deref(),
    );
    if !profile.detected_files.is_empty() {
        lines.push(format!(
            "detected_files: {}",
            profile.detected_files.join(", ")
        ));
    }
    lines.push("</project_dev_profile>".to_string());
    lines.join("\n")
}

fn push_profile_line(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(format!("{key}: {value}"));
    }
}
