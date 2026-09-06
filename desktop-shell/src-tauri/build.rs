fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "run_browser_research",
            "list_local_ai_web_providers",
            "open_local_ai_web_session",
            "present_local_ai_web_session_embedded",
            "hide_local_ai_web_session_embedded",
            "get_local_ai_web_session_state",
            "control_local_ai_web_session",
            "run_local_ai_web_adapter_command",
            "publish_local_ai_web_event",
            "clear_local_ai_web_session",
            "open_internal_browser_tab",
            "resize_internal_browser_tab",
            "control_internal_browser_tab",
            "get_internal_browser_tab_state",
        ]),
    ))
    .expect("一龙桌面壳 Tauri 构建配置失败")
}
