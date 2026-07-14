use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn load_workspace_landing_normalizes_manifest() {
    let workspace = temp_workspace("normalizes");
    let manifest_dir = workspace.join(".elon");
    fs::create_dir_all(&manifest_dir).unwrap();
    fs::write(
        manifest_dir.join("project-landing.json"),
        r#"{
              "title": "Demo",
              "tagline": "Fast client downloads",
              "releaseManifestUrl": "https://example.com/project-downloads.json",
              "downloads": {
                "apk": {
                  "url": "https://example.com/app.apk",
                  "version_name": "1.2.3",
                  "size": "45 MB"
                },
                "ios": {
                  "url": "javascript:alert(1)",
                  "status": "unavailable",
                  "health_error": "guide 404"
                }
              },
              "resources": [
                { "label": "官网", "url": "https://example.com" },
                { "label": "bad", "url": "javascript:alert(1)" }
              ],
              "target_users": ["新用户", { "title": "团队成员" }]
            }"#,
    )
    .unwrap();

    let landing = load_workspace_landing(&workspace).unwrap();
    let object = landing.as_object().unwrap();
    assert_eq!(object["title"], "Demo");
    assert_eq!(
        object["release_manifest_url"],
        "https://example.com/project-downloads.json"
    );
    assert_eq!(object["source"]["status"], "available");
    assert_eq!(object["downloads"][0]["platform"], "android");
    assert_eq!(object["downloads"][0]["status"], "available");
    assert_eq!(object["downloads"][1]["platform"], "ios");
    assert!(object["downloads"][1].get("url").is_none());
    assert_eq!(object["resources"].as_array().unwrap().len(), 1);
    assert_eq!(object["target_users"].as_array().unwrap().len(), 2);

    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn invalid_manifest_returns_source_error_without_absolute_path() {
    let workspace = temp_workspace("invalid");
    let manifest_dir = workspace.join(".elon");
    fs::create_dir_all(&manifest_dir).unwrap();
    fs::write(manifest_dir.join("project-landing.json"), "{").unwrap();

    let landing = load_workspace_landing(&workspace).unwrap();
    assert_eq!(landing["source"]["status"], "invalid");
    assert_eq!(landing["source"]["file"], ".elon/project-landing.json");
    assert!(landing["source"]["health_error"]
        .as_str()
        .unwrap()
        .contains("EOF"));
    assert!(!landing["source"]["health_error"]
        .as_str()
        .unwrap()
        .contains(workspace.to_string_lossy().as_ref()));

    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn normalize_landing_snapshot_rewrites_source_and_filters_urls() {
    let snapshot = normalize_landing_snapshot(&json!({
            "source": { "mode": "workspace_manifest", "status": "available", "file": ".elon/project-landing.json" },
            "title": "Demo",
            "downloads": {
                "windows": {
                    "fallback_url": "https://example.com/app.exe",
                    "fileSize": 7141343,
                    "changelog": "fix tun"
                },
                "ios": {
                    "status": "needs_configuration",
                    "url": "http://example.com/ios.html"
                },
                "macos": {
                    "kind": "third_party_client",
                    "status": "needs_configuration",
                    "options": [
                        {
                            "id": "apple_silicon",
                            "label": "Apple M 芯片",
                            "arch": "arm64",
                            "url": "https://example.com/mac-arm.dmg",
                            "status": "available",
                            "file_size": 2048
                        },
                        {
                            "id": "intel",
                            "label": "Intel 芯片",
                            "arch": "x64",
                            "status": "not_deployed",
                            "health_error": "404"
                        }
                    ]
                }
            }
        }))
        .unwrap();

    assert_eq!(snapshot["source"]["mode"], "node_agent_snapshot");
    assert_eq!(snapshot["title"], "Demo");
    let downloads = snapshot["downloads"].as_array().unwrap();
    let windows = downloads
        .iter()
        .find(|download| download["platform"] == "windows")
        .unwrap();
    let ios = downloads
        .iter()
        .find(|download| download["platform"] == "ios")
        .unwrap();
    let macos = downloads
        .iter()
        .find(|download| download["platform"] == "macos")
        .unwrap();
    assert_eq!(windows["url"], "https://example.com/app.exe");
    assert_eq!(windows["size_bytes"], "7141343");
    assert_eq!(windows["note"], "fix tun");
    assert_eq!(ios["status"], "needs_configuration");
    assert_eq!(macos["kind"], "third_party_client");
    assert_eq!(macos["status"], "partial");
    let variants = macos["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0]["label"], "Apple M 芯片");
    assert_eq!(variants[0]["status"], "available");
    assert_eq!(variants[0]["size_bytes"], "2048");
    assert_eq!(variants[1]["status"], "not_deployed");
    assert_eq!(variants[1]["health_error"], "404");
    assert!(has_display_content(&snapshot));
}

fn temp_workspace(label: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("elon-project-landing-{label}-{id}"))
}
