use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{process::Command, time::timeout};

const MAX_PREVIEWS: usize = 200;
const MAX_KOTLIN_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RENDER_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RendererCapabilitiesRequest {
    pub(crate) project_root: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComposePreviewEntry {
    id: String,
    kotlin_file: String,
    composable: String,
    label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RendererCapabilities {
    ok: bool,
    recommended_backend: String,
    layoutlib: LayoutlibCapability,
    preview_host: PreviewHostCapability,
    pwa_preview: PwaPreviewCapability,
    react_twin: ReactTwinCapability,
    compose_previews: Vec<ComposePreviewEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LayoutlibCapability {
    available: bool,
    command: Option<String>,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewHostCapability {
    available_after_debug_build: bool,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PwaPreviewCapability {
    available: bool,
    url: Option<String>,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReactTwinCapability {
    available: bool,
    authoritative: bool,
    detail: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComposeRenderRequest {
    project_root: String,
    kotlin_file: String,
    composable: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComposeRenderResult {
    ok: bool,
    backend: String,
    authoritative: bool,
    kotlin_file: String,
    composable: String,
    data_url: String,
    semantics_text: String,
}

pub(crate) fn capabilities(project_root: &str) -> Result<RendererCapabilities> {
    let root = canonical_project_root(project_root)?;
    let command = locate_android_cli();
    let compose_previews = discover_compose_previews(&root)?;
    let pwa_preview_url = discover_pwa_preview_url(&root);
    let recommended_backend = if command.is_some() && !compose_previews.is_empty() {
        "android_layoutlib"
    } else if pwa_preview_url.is_some() {
        "pwa_interactive"
    } else if !compose_previews.is_empty() {
        "android_preview_host"
    } else {
        "react_twin"
    };
    Ok(RendererCapabilities {
        ok: true,
        recommended_backend: recommended_backend.to_string(),
        layoutlib: LayoutlibCapability {
            available: command.is_some(),
            command: command.as_ref().map(|path| path.display().to_string()),
            detail: if command.is_some() {
                "可调用 Android Studio Compose Preview/Layoutlib；渲染时仍要求兼容版本的 Android Studio 已打开该项目".to_string()
            } else {
                "未找到 Android CLI；安装并启动兼容版本 Android Studio 后可启用真实 Compose Preview"
                    .to_string()
            },
        },
        preview_host: PreviewHostCapability {
            available_after_debug_build: true,
            detail: "不能由 Layoutlib 渲染的页面，使用现有 Debug Preview Host/模拟器作为权威画面"
                .to_string(),
        },
        pwa_preview: PwaPreviewCapability {
            available: pwa_preview_url.is_some(),
            url: pwa_preview_url,
            detail: if root.join("server/src/assets/web_page.html").is_file() {
                "已发现与 APK 同步维护的移动 PWA；可作为可交互即时草稿，最终仍由 Android 真帧校准"
                    .to_string()
            } else {
                "未发现项目 PWA 预览入口；可在 .elon/ui-pwa-preview.json 中声明同源 url".to_string()
            },
        },
        react_twin: ReactTwinCapability {
            available: true,
            authoritative: false,
            detail: "本地即时数字孪生，只负责低延迟草稿；必须用 Android 真帧校准和最终验证"
                .to_string(),
        },
        compose_previews,
    })
}

fn discover_pwa_preview_url(root: &Path) -> Option<String> {
    let config = root.join(".elon").join("ui-pwa-preview.json");
    if let Ok(source) = fs::read_to_string(config) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&source) {
            if let Some(url) = value.get("url").and_then(|item| item.as_str()) {
                let url = url.trim();
                if is_safe_pwa_preview_url(url) {
                    return Some(url.to_string());
                }
            }
        }
    }
    root.join("server/src/assets/web_page.html")
        .is_file()
        .then(|| "/web?ui_tuner_preview=1".to_string())
}

fn is_safe_pwa_preview_url(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && !value.contains('\n')
        && !value.contains('\r')
}

pub(crate) async fn render_compose_preview(
    request: ComposeRenderRequest,
) -> Result<ComposeRenderResult> {
    validate_composable(&request.composable)?;
    let root = canonical_project_root(&request.project_root)?;
    let kotlin_file = canonical_child(&root, &request.kotlin_file)?;
    if kotlin_file.extension().and_then(|value| value.to_str()) != Some("kt") {
        bail!("Compose Preview 源文件必须是 .kt");
    }
    let cli = locate_android_cli()
        .ok_or_else(|| anyhow!("未找到 Android CLI，无法调用真实 Compose Preview/Layoutlib"))?;
    let output_dir = root.join(".elon").join("ui-tuner").join("layoutlib");
    fs::create_dir_all(&output_dir).context("创建 Layoutlib 输出目录失败")?;
    let digest = hex::encode(Sha256::digest(format!(
        "{}:{}",
        kotlin_file.display(),
        request.composable
    )));
    let output_file = output_dir.join(format!("preview-{}.png", &digest[..20]));
    let mut command = Command::new(&cli);
    command
        .current_dir(&root)
        .arg("studio")
        .arg("render-compose-preview")
        .arg(format!("--output-image-file={}", output_file.display()))
        .arg("--print-semantics")
        .arg(&kotlin_file)
        .arg(&request.composable)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    let output = timeout(Duration::from_secs(120), command.output())
        .await
        .map_err(|_| anyhow!("Compose Preview 渲染超时（120 秒）"))?
        .with_context(|| format!("启动 Android CLI 失败: {}", cli.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "真实 Compose Preview 渲染失败：{}{}",
            stdout.trim(),
            stderr.trim()
        );
    }
    let png = fs::read(&output_file)
        .with_context(|| format!("Android CLI 未生成预览图: {}", output_file.display()))?;
    if png.is_empty() || png.len() > MAX_RENDER_BYTES {
        bail!("Compose Preview 图片为空或超过 16MiB");
    }
    Ok(ComposeRenderResult {
        ok: true,
        backend: "android_layoutlib".to_string(),
        authoritative: true,
        kotlin_file: relative_slash(&root, &kotlin_file),
        composable: request.composable,
        data_url: format!("data:image/png;base64,{}", B64.encode(png)),
        semantics_text: String::from_utf8_lossy(&output.stdout)
            .chars()
            .take(512_000)
            .collect(),
    })
}

fn discover_compose_previews(root: &Path) -> Result<Vec<ComposePreviewEntry>> {
    let mut previews = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .build();
    for entry in walker.filter_map(Result::ok) {
        if previews.len() >= MAX_PREVIEWS {
            break;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("kt") {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(value) => value,
            Err(_) => continue,
        };
        if metadata.len() > MAX_KOTLIN_BYTES {
            continue;
        }
        let source = match fs::read_to_string(path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        for composable in preview_functions(&source) {
            let relative = relative_slash(root, path);
            previews.push(ComposePreviewEntry {
                id: format!("{}#{}", relative, composable),
                kotlin_file: relative.clone(),
                label: format!("{} · {}", composable, relative),
                composable,
            });
            if previews.len() >= MAX_PREVIEWS {
                break;
            }
        }
    }
    Ok(previews)
}

fn preview_functions(source: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut awaiting_function = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("@Preview") || trimmed.contains(".Preview(") {
            awaiting_function = true;
        }
        if !awaiting_function {
            continue;
        }
        if let Some(fun_index) = trimmed.find("fun ") {
            let tail = &trimmed[fun_index + 4..];
            let name: String = tail
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .collect();
            if !name.is_empty() {
                result.push(name);
            }
            awaiting_function = false;
        }
    }
    result
}

fn canonical_project_root(value: &str) -> Result<PathBuf> {
    let root = PathBuf::from(value.trim())
        .canonicalize()
        .context("Android 项目目录不存在")?;
    if !root.is_dir() {
        bail!("Android 项目路径不是目录");
    }
    Ok(root)
}

fn canonical_child(root: &Path, value: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(value.trim());
    let path = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    let canonical = path
        .canonicalize()
        .context("Compose Preview 源文件不存在")?;
    if !canonical.starts_with(root) {
        bail!("Compose Preview 源文件必须位于项目目录内");
    }
    Ok(canonical)
}

fn validate_composable(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        bail!("Composable 名称非法");
    }
    Ok(())
}

fn locate_android_cli() -> Option<PathBuf> {
    if let Some(path) = env::var_os("ELON_ANDROID_CLI")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Some(path);
    }
    let names: &[&str] = if cfg!(windows) {
        &["android.exe"]
    } else {
        &["android"]
    };
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .find_map(|dir| {
            names
                .iter()
                .map(|name| dir.join(name))
                .find(|path| path.is_file())
        })
}

fn relative_slash(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_preview_functions_without_treating_other_composables_as_previews() {
        let source = r#"
            @Preview(showBackground = true)
            @Composable
            fun CheckoutPreview() = Checkout()

            @Composable fun Checkout() = Unit
        "#;
        assert_eq!(preview_functions(source), vec!["CheckoutPreview"]);
    }

    #[test]
    fn rejects_unsafe_composable_name() {
        assert!(validate_composable("CheckoutPreview").is_ok());
        assert!(validate_composable("CheckoutPreview; rm -rf").is_err());
    }

    #[test]
    fn pwa_preview_url_must_stay_on_the_current_origin() {
        assert!(is_safe_pwa_preview_url("/web?ui_tuner_preview=1"));
        assert!(!is_safe_pwa_preview_url("https://example.com/web"));
        assert!(!is_safe_pwa_preview_url("//example.com/web"));
    }
}
