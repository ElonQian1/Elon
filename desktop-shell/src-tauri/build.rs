fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "list_local_ai_web_providers",
            "open_local_ai_web_session",
            "get_local_ai_web_session_state",
            "control_local_ai_web_session",
            "run_local_ai_web_adapter_command",
            "publish_local_ai_web_event",
            "clear_local_ai_web_session",
        ]),
    ))
    .expect("一龙桌面壳 Tauri 构建配置失败")
}
