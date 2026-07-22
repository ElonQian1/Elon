use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use crate::node_agent_android_inspector::adb_capture::{
    capture_screen_png, wake_device_for_user_interaction,
};
use crate::node_agent_android_inspector::adb_command::{
    run_adb_text, validate_device_id, validate_package_name,
};

use super::broker::LiveUiSession;
use super::frame_artifact::persist_launcher_surface_artifact;
use super::launcher_xml::{launcher_candidates, page_identity, parse_nodes};
use super::visual_diff::PixelRect;

const MAX_XML_BYTES: usize = 4 * 1024 * 1024;
const RUNTIME_RESTORE_ATTEMPTS: usize = 60;

#[derive(Debug)]
struct LocatedIcon {
    rect: PixelRect,
    surface: &'static str,
    moves_from_origin: i32,
    pages_inspected: u32,
    candidate_launches: u32,
    evidence: Vec<String>,
}

pub(crate) async fn capture(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    match arguments
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("PACKAGE_ICON")
    {
        "PACKAGE_ICON" => super::launcher_icon::capture(session, arguments).await,
        "OEM_FIXED_POSITION" => {
            if arguments.get("iconRect").is_none() {
                bail!("OEM_FIXED_POSITION 必须提供固定测试位置 iconRect");
            }
            capture_oem_or_legacy(session, arguments, true).await
        }
        "LEGACY_BOUNDED_SEARCH" => capture_oem_or_legacy(session, arguments, false).await,
        mode => bail!("不支持的 Launcher 捕获 mode: {mode}"),
    }
}

async fn capture_oem_or_legacy(
    session: &LiveUiSession,
    arguments: &Value,
    fixed_position: bool,
) -> Result<Value> {
    let device_id = arguments
        .get("deviceId")
        .and_then(Value::as_str)
        .unwrap_or(&session.device_id)
        .trim();
    let package_name = arguments
        .get("packageName")
        .and_then(Value::as_str)
        .unwrap_or(&session.package_name)
        .trim();
    validate_device_id(device_id)?;
    validate_package_name(package_name)?;
    let settle_ms = arguments
        .get("settleMs")
        .and_then(Value::as_u64)
        .unwrap_or(900);
    if !(200..=5_000).contains(&settle_ms) {
        bail!("settleMs 必须在 200..5000");
    }
    let max_pages = arguments
        .get("maxPages")
        .and_then(Value::as_u64)
        .unwrap_or(24);
    if !(1..=32).contains(&max_pages) {
        bail!("maxPages 必须在 1..32");
    }
    let explicit_rect = arguments
        .get("iconRect")
        .cloned()
        .map(serde_json::from_value::<PixelRect>)
        .transpose()?;
    let was_connected = session.view().await.connected;

    wake_device_for_user_interaction(device_id).await;
    let app_label = match arguments.get("appLabel").and_then(Value::as_str) {
        Some(value) => value.trim().to_string(),
        None if explicit_rect.is_none() => {
            resolve_app_label(device_id, package_name, settle_ms).await?
        }
        None => String::new(),
    };
    let home_output = press_home(device_id).await?;
    tokio::time::sleep(Duration::from_millis(settle_ms)).await;
    let origin_xml = dump_xml(device_id).await?;
    let origin_page = page_identity(&origin_xml);

    let located = if let Some(rect) = explicit_rect {
        LocatedIcon {
            rect,
            surface: if fixed_position {
                "OEM_FIXED_POSITION"
            } else {
                "EXPLICIT_RECT"
            },
            moves_from_origin: 0,
            pages_inspected: 1,
            candidate_launches: 0,
            evidence: vec!["caller supplied iconRect".to_string()],
        }
    } else {
        locate_icon(
            device_id,
            package_name,
            &app_label,
            &origin_page,
            settle_ms,
            max_pages as u32,
        )
        .await?
    };
    let png = capture_screen_png(device_id).await?;
    let surface = persist_launcher_surface_artifact(session, &png, None)?;
    let icon = persist_launcher_surface_artifact(session, &png, Some(located.rect))?;

    let page_restored = restore_origin_page(
        device_id,
        located.surface,
        located.moves_from_origin,
        &origin_page,
        settle_ms,
    )
    .await;
    let runtime_restored = if was_connected {
        launch_package(device_id, &session.package_name).await?;
        wait_for_same_runtime(session, settle_ms).await
    } else {
        false
    };
    let foreground_line = foreground_line(device_id).await.unwrap_or_default();
    if was_connected && !runtime_restored {
        bail!(
            "LAUNCHER_CAPTURE_RUNTIME_RESTORE_FAILED: session={} package={} pageRestored={} foreground={}",
            session.id,
            package_name,
            page_restored,
            foreground_line
        );
    }
    Ok(json!({
        "surface": surface,
        "iconRect": located.rect,
        "iconCrop": icon,
        "deviceId": device_id,
        "packageName": package_name,
        "appLabel": app_label,
        "launcherForeground": foreground_line,
        "homeCommand": home_output.lines().take(8).collect::<Vec<_>>().join(" | "),
        "captureKind": "ANDROID_LAUNCHER_SURFACE",
        "captureMode": if fixed_position { "OEM_FIXED_POSITION" } else { "LEGACY_BOUNDED_SEARCH" },
        "locator": {
            "surface": located.surface,
            "bounded": true,
            "maxPages": max_pages,
            "pagesInspected": located.pages_inspected,
            "candidateLaunches": located.candidate_launches,
            "evidence": located.evidence,
        },
        "recovery": {
            "originPage": origin_page,
            "pageRestored": page_restored,
            "runtimeWasConnected": was_connected,
            "sameRuntimeSessionId": session.id,
            "runtimePackageName": session.package_name,
            "runtimeRestored": runtime_restored,
        },
        "maskAwareDiffTool": "ui_get_visual_diff"
    }))
}

async fn locate_icon(
    device_id: &str,
    package_name: &str,
    label: &str,
    origin_page: &str,
    settle_ms: u64,
    max_pages: u32,
) -> Result<LocatedIcon> {
    let mut evidence = Vec::new();
    let mut inspected = 0;
    let mut launches = 0;
    for direction in [1_i32, -1_i32] {
        if direction == -1 {
            press_home(device_id).await?;
            tokio::time::sleep(Duration::from_millis(settle_ms)).await;
            restore_page_by_identity(device_id, origin_page, -1, settle_ms, max_pages).await?;
        }
        let mut last_page = String::new();
        for step in 0..max_pages {
            let xml = dump_xml(device_id).await?;
            let page = page_identity(&xml);
            if step > 0 && page == last_page {
                evidence.push(format!(
                    "workspace boundary direction={direction} page={page}"
                ));
                break;
            }
            last_page = page.clone();
            inspected += 1;
            for node in launcher_candidates(&xml, label)? {
                launches += 1;
                tap(device_id, node.rect).await?;
                tokio::time::sleep(Duration::from_millis(settle_ms)).await;
                if foreground_line(device_id).await?.contains(package_name) {
                    press_home(device_id).await?;
                    tokio::time::sleep(Duration::from_millis(settle_ms)).await;
                    evidence.push(format!("verified package on workspace page={page}"));
                    return Ok(LocatedIcon {
                        rect: icon_rect(node.rect),
                        surface: "WORKSPACE",
                        moves_from_origin: direction * step as i32,
                        pages_inspected: inspected,
                        candidate_launches: launches,
                        evidence,
                    });
                }
                press_home(device_id).await?;
                tokio::time::sleep(Duration::from_millis(settle_ms)).await;
            }
            swipe_page(device_id, direction).await?;
            tokio::time::sleep(Duration::from_millis(settle_ms)).await;
        }
    }

    press_home(device_id).await?;
    swipe(device_id, 540, 1900, 540, 400, 350).await?;
    tokio::time::sleep(Duration::from_millis(settle_ms)).await;
    let mut last = String::new();
    for step in 0..max_pages.min(12) {
        let xml = dump_xml(device_id).await?;
        let identity = page_identity(&xml);
        if step > 0 && identity == last {
            break;
        }
        last = identity;
        inspected += 1;
        for node in launcher_candidates(&xml, label)? {
            launches += 1;
            tap(device_id, node.rect).await?;
            tokio::time::sleep(Duration::from_millis(settle_ms)).await;
            if foreground_line(device_id).await?.contains(package_name) {
                press_home(device_id).await?;
                evidence.push("verified package in app drawer".to_string());
                return Ok(LocatedIcon {
                    rect: icon_rect(node.rect),
                    surface: "APP_DRAWER",
                    moves_from_origin: 0,
                    pages_inspected: inspected,
                    candidate_launches: launches,
                    evidence,
                });
            }
            press_home(device_id).await?;
            swipe(device_id, 540, 1900, 540, 400, 350).await?;
            tokio::time::sleep(Duration::from_millis(settle_ms)).await;
        }
        swipe(device_id, 540, 1800, 540, 500, 350).await?;
        tokio::time::sleep(Duration::from_millis(settle_ms)).await;
    }
    bail!(
        "LAUNCHER_ICON_NOT_FOUND: package={package_name} label={label:?} pagesInspected={inspected} candidateLaunches={launches} maxPages={max_pages} evidence={}",
        evidence.join(" | ")
    )
}

fn icon_rect(cell: PixelRect) -> PixelRect {
    let width = cell.right - cell.left;
    let height = cell.bottom - cell.top;
    let side = ((width as f64 * 0.66).round() as i32)
        .min(height.saturating_sub(64))
        .max(1);
    let left = cell.left + (width - side) / 2;
    PixelRect {
        left,
        top: cell.top + ((height - side - 48).max(0) / 3),
        right: left + side,
        bottom: cell.top + ((height - side - 48).max(0) / 3) + side,
    }
}

async fn restore_page_by_identity(
    device_id: &str,
    expected: &str,
    direction: i32,
    settle_ms: u64,
    max_pages: u32,
) -> Result<()> {
    for _ in 0..=max_pages {
        let current = page_identity(&dump_xml(device_id).await?);
        if current == expected {
            return Ok(());
        }
        swipe_page(device_id, direction).await?;
        tokio::time::sleep(Duration::from_millis(settle_ms)).await;
    }
    bail!("LAUNCHER_ORIGIN_PAGE_NOT_RESTORED: expected={expected}")
}

async fn resolve_app_label(device_id: &str, package_name: &str, settle_ms: u64) -> Result<String> {
    run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "am".into(),
            "start".into(),
            "-W".into(),
            "-a".into(),
            "android.settings.APPLICATION_DETAILS_SETTINGS".into(),
            "-d".into(),
            format!("package:{package_name}"),
        ],
        Duration::from_secs(10),
        64 * 1024,
    )
    .await
    .context("打开目标 package 应用信息失败")?;
    tokio::time::sleep(Duration::from_millis(settle_ms)).await;
    let xml = dump_xml(device_id).await?;
    let nodes = parse_nodes(&xml)?;
    nodes
        .iter()
        .find(|node| {
            node.class_name.ends_with("TextView")
                && (node.content_desc.contains("applabel") || !node.text.is_empty())
                && node.text != "应用信息"
                && !node.text.starts_with("版本")
        })
        .map(|node| node.text.clone())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("LAUNCHER_APP_LABEL_UNAVAILABLE: package={package_name}"))
}

async fn restore_origin_page(
    device_id: &str,
    surface: &str,
    moves: i32,
    origin: &str,
    settle_ms: u64,
) -> bool {
    if surface == "APP_DRAWER" {
        let _ = press_home(device_id).await;
    } else {
        let reverse = -moves.signum();
        for _ in 0..moves.unsigned_abs() {
            let _ = swipe_page(device_id, reverse).await;
            tokio::time::sleep(Duration::from_millis(settle_ms)).await;
        }
    }
    dump_xml(device_id)
        .await
        .is_ok_and(|xml| page_identity(&xml) == origin)
}

async fn wait_for_same_runtime(session: &LiveUiSession, settle_ms: u64) -> bool {
    for _ in 0..RUNTIME_RESTORE_ATTEMPTS {
        if session.view().await.connected {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(settle_ms.min(500))).await;
    }
    false
}

async fn dump_xml(device_id: &str) -> Result<String> {
    let remote = format!(
        "/sdcard/elon-launcher-{}.xml",
        uuid::Uuid::new_v4().simple()
    );
    run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "uiautomator".into(),
            "dump".into(),
            remote.clone(),
        ],
        Duration::from_secs(10),
        128 * 1024,
    )
    .await?;
    let result = run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "exec-out".into(),
            "cat".into(),
            remote.clone(),
        ],
        Duration::from_secs(6),
        MAX_XML_BYTES,
    )
    .await;
    let _ = run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "rm".into(),
            "-f".into(),
            remote,
        ],
        Duration::from_secs(3),
        16 * 1024,
    )
    .await;
    result
}

async fn press_home(device_id: &str) -> Result<String> {
    run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "input".into(),
            "keyevent".into(),
            "HOME".into(),
        ],
        Duration::from_secs(5),
        16 * 1024,
    )
    .await
}

async fn launch_package(device_id: &str, package_name: &str) -> Result<String> {
    run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "monkey".into(),
            "-p".into(),
            package_name.into(),
            "-c".into(),
            "android.intent.category.LAUNCHER".into(),
            "1".into(),
        ],
        Duration::from_secs(12),
        64 * 1024,
    )
    .await
}

async fn tap(device_id: &str, rect: PixelRect) -> Result<()> {
    let x = (rect.left + rect.right) / 2;
    let y = (rect.top + rect.bottom) / 2;
    run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "input".into(),
            "tap".into(),
            x.to_string(),
            y.to_string(),
        ],
        Duration::from_secs(5),
        16 * 1024,
    )
    .await?;
    Ok(())
}

async fn swipe_page(device_id: &str, direction: i32) -> Result<()> {
    if direction > 0 {
        swipe(device_id, 900, 1100, 180, 1100, 280).await
    } else {
        swipe(device_id, 180, 1100, 900, 1100, 280).await
    }
}

async fn swipe(device_id: &str, x1: i32, y1: i32, x2: i32, y2: i32, duration: i32) -> Result<()> {
    run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "input".into(),
            "swipe".into(),
            x1.to_string(),
            y1.to_string(),
            x2.to_string(),
            y2.to_string(),
            duration.to_string(),
        ],
        Duration::from_secs(6),
        16 * 1024,
    )
    .await?;
    Ok(())
}

async fn foreground_line(device_id: &str) -> Result<String> {
    let window = run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "dumpsys".into(),
            "window".into(),
            "windows".into(),
        ],
        Duration::from_secs(6),
        512 * 1024,
    )
    .await?;
    if let Some(line) = window
        .lines()
        .find(|line| line.contains("mCurrentFocus") || line.contains("mFocusedApp"))
    {
        if !line.trim().is_empty() {
            return Ok(line.trim().to_string());
        }
    }
    let activities = run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "dumpsys".into(),
            "activity".into(),
            "activities".into(),
        ],
        Duration::from_secs(6),
        1024 * 1024,
    )
    .await?;
    Ok(activities
        .lines()
        .find(|line| line.contains("mResumedActivity") || line.contains("topResumedActivity"))
        .unwrap_or_default()
        .trim()
        .to_string())
}
