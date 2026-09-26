fn main() {
    // The running shell must identify its own binary, never the node or a disk manifest.
    println!("cargo:rerun-if-env-changed=ELON_DESKTOP_RELEASE_IDENTITY");
    if let Ok(identity) = std::env::var("ELON_DESKTOP_RELEASE_IDENTITY") {
        let (version, sha) = identity
            .rsplit_once('+')
            .expect("exact desktop release required");
        assert!(!version.is_empty() && version.len() <= 48);
        assert!(version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)));
        assert!((40..=64).contains(&sha.len()) && sha.bytes().all(|b| b.is_ascii_hexdigit()));
        println!("cargo:rustc-env=ELON_DESKTOP_RELEASE_IDENTITY={identity}");
    }
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "run_browser_research",
            "browser_research_host",
            "list_local_ai_web_providers",
            "list_exchange_web_providers",
            "open_local_ai_web_session",
            "group_ai_web_session",
            "open_exchange_web_session",
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
