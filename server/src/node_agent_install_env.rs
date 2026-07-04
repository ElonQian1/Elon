use serde::Deserialize;
use std::path::PathBuf;

const SETUP_ENV_SCRIPT: &str = include_str!("../../scripts/setup-node-env.ps1");

#[derive(Deserialize)]
pub(crate) struct InstallEnvReq {
    target: Option<String>,
}

/// POST /api/install-env - 用户主动触发的后台安装/修复任务。
pub(crate) async fn admin_install_env(
    body: Option<axum::Json<InstallEnvReq>>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    #[cfg(windows)]
    {
        let codex_only = body
            .as_ref()
            .and_then(|payload| payload.target.as_deref())
            .map(|target| target.eq_ignore_ascii_case("codex"))
            .unwrap_or(false);
        let tmp = std::env::temp_dir().join("elon-setup-node-env.ps1");
        let mut script_bytes = vec![0xEF, 0xBB, 0xBF];
        script_bytes.extend_from_slice(SETUP_ENV_SCRIPT.as_bytes());
        if let Err(e) = std::fs::write(&tmp, script_bytes) {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": format!("写入临时脚本失败: {e}")
                })),
            );
        }

        let script_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("setup-node-env.ps1")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| tmp.to_string_lossy().to_string());

        let shell = elon_pc_dev_runtime::command_path("pwsh")
            .or_else(|| elon_pc_dev_runtime::command_path("powershell"))
            .unwrap_or_else(|| PathBuf::from("powershell"));
        let mut command = std::process::Command::new(shell);
        command.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
            &script_path,
            "-Silent",
        ]);
        if codex_only {
            command.arg("-CodexOnly");
        }
        command.stdin(std::process::Stdio::null());
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());

        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
        }

        match command.spawn() {
            Ok(_) => (
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({
                    "ok": true,
                    "msg": if codex_only {
                        "Codex CLI 安装/修复任务已在后台启动，稍后点击重新检测"
                    } else {
                        "安装/修复任务已在后台启动，稍后刷新本页查看结果"
                    }
                })),
            ),
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": format!("启动脚本失败: {e}")
                })),
            ),
        }
    }
    #[cfg(not(windows))]
    {
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": "自动安装向导仅限 Windows。Linux 用户请手动执行：\nbash scripts/setup-node-env.sh\n（或参照文档手动安装 git / jdk17 / node / codex / android-sdk）"
            })),
        )
    }
}
