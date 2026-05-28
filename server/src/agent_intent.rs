//! 项目级 agent 调度时使用的意图分类 / 工作区识别辅助函数（从 agent.rs 抽出）。
//!
//! 这些函数只读取用户消息和 worktree 路径，决定是否进入"继续上次"、"交付 APK 地址"、
//! "短构建命令"等快速分支；不持有任何状态。

use std::path::Path;

pub(crate) fn is_short_resume_command(user_message: &str, workspace: &Path) -> bool {
    if !is_project_workspace(workspace) {
        return false;
    }
    let normalized = user_message.trim().to_lowercase();
    if normalized.contains("继续完成上一次未完成")
        || normalized.contains("未完成的开发任务")
        || normalized.contains("继续当前项目")
        || (normalized.contains("检查当前项目状态") && normalized.contains("apk"))
    {
        return true;
    }
    matches!(
        normalized.as_str(),
        "继续"
            | "继续吧"
            | "继续开发"
            | "继续做"
            | "继续完成"
            | "重试"
            | "再试一次"
            | "重新开始"
            | "再来一次"
    )
}

pub(crate) fn is_project_delivery_request(user_message: &str, workspace: &Path) -> bool {
    if !is_project_workspace(workspace) {
        return false;
    }
    let normalized = user_message.trim().to_lowercase();
    let asks_for_apk = normalized.contains("apk")
        || normalized.contains("安装包")
        || normalized.contains("下载包");
    let asks_for_delivery = normalized.contains("地址")
        || normalized.contains("链接")
        || normalized.contains("下载")
        || normalized.contains("发给我")
        || normalized.contains("给我")
        || normalized.contains("做好")
        || normalized.contains("做完")
        || normalized.contains("完成");

    asks_for_apk && asks_for_delivery
}

/// 消息是否**纯粹**只在问 APK 下载地址，不含任何额外的开发需求。
///
/// 用于区分两种情形：
/// - 纯交付（"apk在哪里"、"下载地址是什么"）→ 立即返回链接，无需 AI
/// - 混合（"发我APK，然后再帮我加第8步"）→ 先发链接，再走 AI 继续完成开发
pub(crate) fn is_pure_apk_delivery_request(user_message: &str, workspace: &Path) -> bool {
    if !is_project_delivery_request(user_message, workspace) {
        return false;
    }
    let normalized = user_message.trim().to_lowercase();
    // 含以下开发关键词说明是混合请求，不能直接短路
    let has_dev_keywords = normalized.contains("帮我")
        || normalized.contains("添加")
        || normalized.contains("增加")
        || normalized.contains("修改")
        || normalized.contains("改一下")
        || normalized.contains("加一个")
        || normalized.contains("加第")
        || normalized.contains("新增")
        || normalized.contains("实现")
        || normalized.contains("开发")
        || normalized.contains("做一个")
        || normalized.contains("编写")
        || normalized.contains("功能");
    !has_dev_keywords
}

pub(crate) fn is_short_build_command(user_message: &str, workspace: &Path) -> bool {
    if !is_project_workspace(workspace) {
        return false;
    }
    let normalized = user_message.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "打包" | "编译" | "生成apk" | "生成 apk" | "打包apk" | "打包 apk"
    )
}

pub(crate) fn is_project_workspace(workspace: &Path) -> bool {
    workspace.join(".git").exists()
        || workspace.join("gradlew").exists()
        || workspace.join("android").join("gradlew").exists()
        || workspace.join("Cargo.toml").exists()
        || workspace.join("server").join("Cargo.toml").exists()
        || workspace.join("package.json").exists()
        || workspace
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.contains("__"))
            .unwrap_or(false)
}

pub(crate) fn has_origin_remote(workspace: &Path) -> bool {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}
