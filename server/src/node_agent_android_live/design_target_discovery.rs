use std::{collections::BTreeSet, fs, path::Path};

use anyhow::Result;
use ignore::WalkBuilder;

use super::design_targets::{DesignPlatform, DesignTarget};

const MAX_SCANNED_FILES: usize = 4_000;

pub(super) fn discover_targets(root: &Path) -> Result<(Vec<DesignTarget>, usize, bool)> {
    let mut inspected = 0usize;
    let mut web_roots = BTreeSet::new();
    let mut web_configs = BTreeSet::new();
    let mut pwa_configs = BTreeSet::new();
    let mut tauri_configs = BTreeSet::new();
    let mut android_configs = BTreeSet::new();
    let mut has_pwa = false;
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .max_depth(Some(10))
        .build()
        .filter_map(|entry| entry.ok())
    {
        if inspected >= MAX_SCANNED_FILES {
            break;
        }
        let path = entry.path();
        if !entry.file_type().is_some_and(|kind| kind.is_file()) || ignored_path(path) {
            continue;
        }
        inspected += 1;
        let relative = path.strip_prefix(root).unwrap_or(path);
        let text = relative.to_string_lossy().replace('\\', "/");
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name == "package.json" || name == "index.html" {
            web_configs.insert(text.clone());
            web_roots.insert(relative_parent(relative));
        }
        if name == "package.json" {
            let prefix = read_prefix(path, 256 * 1024).unwrap_or_default();
            if ["vite-plugin-pwa", "workbox", "@angular/service-worker"]
                .iter()
                .any(|marker| prefix.contains(marker))
            {
                has_pwa = true;
                pwa_configs.insert(text.clone());
            }
        }
        if name.ends_with(".webmanifest")
            || matches!(
                name.as_str(),
                "service-worker.js" | "service-worker.ts" | "sw.js" | "sw.ts"
            )
        {
            has_pwa = true;
            pwa_configs.insert(text.clone());
        }
        if matches!(
            name.as_str(),
            "tauri.conf.json" | "tauri.conf.json5" | "tauri.conf.toml"
        ) && text.contains("src-tauri/")
        {
            tauri_configs.insert(text.clone());
            web_roots.insert(text.split("/src-tauri/").next().unwrap_or(".").to_string());
        }
        if name == "androidmanifest.xml" {
            android_configs.insert(text);
        }
    }
    let mut targets = Vec::new();
    if !web_configs.is_empty() || !tauri_configs.is_empty() {
        targets.push(target(
            DesignPlatform::Web,
            "Web",
            "HEADLESS_CHROMIUM",
            "BROWSER_RUNTIME",
            &web_roots,
            &web_configs,
        ));
    }
    if has_pwa {
        targets.push(target(
            DesignPlatform::Pwa,
            "PWA",
            "HEADLESS_CHROMIUM_PWA",
            "PWA_RUNTIME",
            &web_roots,
            &pwa_configs,
        ));
    }
    if !tauri_configs.is_empty() {
        targets.push(target(
            DesignPlatform::Tauri,
            "Tauri 桌面",
            "TAURI_FRONTEND_PLUS_NATIVE_HOST",
            "TAURI_NATIVE_ON_CAPTURE",
            &web_roots,
            &tauri_configs,
        ));
    }
    if !android_configs.is_empty() || root.join("android").is_dir() {
        targets.push(target(
            DesignPlatform::Android,
            "Android",
            "ANDROID_LIVE_RUNTIME",
            "ANDROID_RUNTIME",
            &BTreeSet::from(["android".to_string()]),
            &android_configs,
        ));
    }
    Ok((targets, inspected, inspected >= MAX_SCANNED_FILES))
}

fn target(
    platform: DesignPlatform,
    label: &'static str,
    adapter: &'static str,
    evidence_level: &'static str,
    roots: &BTreeSet<String>,
    configs: &BTreeSet<String>,
) -> DesignTarget {
    let mut capabilities = vec![
        "ROUTE_NAVIGATION",
        "COMPACT_PIXEL_EVIDENCE",
        "SOURCE_HANDOFF",
    ];
    if platform == DesignPlatform::Android {
        capabilities.extend(["RUNTIME_UI_TREE", "LIVE_STYLE_PATCH"]);
    } else {
        capabilities.extend(["SAFE_CLICK_REPLAY", "SEMANTIC_UI_TREE"]);
    }
    if platform == DesignPlatform::Tauri {
        capabilities.extend([
            "NATIVE_HOST_LIFECYCLE",
            "NATIVE_WINDOW_CAPTURE",
            "NATIVE_MENU_INSPECTION",
            "NATIVE_DIALOG_INSPECTION",
            "RUST_COMMAND_TRACE",
        ]);
    }
    DesignTarget {
        id: platform.as_str().to_string(),
        platform,
        label: label.to_string(),
        adapter: adapter.to_string(),
        evidence_level: evidence_level.to_string(),
        source_roots: roots.iter().cloned().collect(),
        config_files: configs.iter().cloned().collect(),
        capabilities: capabilities.into_iter().map(ToOwned::to_owned).collect(),
        native_host_verified: platform != DesignPlatform::Tauri,
    }
}

fn relative_parent(path: &Path) -> String {
    path.parent()
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_string_lossy().as_ref(),
            ".git" | ".elon" | "target" | "build" | ".gradle" | "node_modules" | "dist"
        )
    })
}

fn read_prefix(path: &Path, max: usize) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes[..bytes.len().min(max)]).to_string())
}
