fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "list_local_ai_web_providers",
            "open_local_ai_web_session",
            "clear_local_ai_web_session",
        ]),
    ))
    .expect("一龙桌面壳 Tauri 构建配置失败")
}
