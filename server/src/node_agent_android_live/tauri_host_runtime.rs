use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::OnceLock,
};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

use super::{
    broker::LiveUiSession,
    design_session_store::{
        persist_record, read_record, validate_design_session_id, DesignSessionRecord,
        VerifiedPixelArtifact,
    },
    design_targets::DesignPlatform,
    tauri_host_windows::{capture_png, find_best_window, NativeWindow},
};

const PREPARE_TOOL: &str = "ui_prepare_tauri_runtime";
const CAPTURE_TOOL: &str = "ui_capture_tauri_host";
const STOP_TOOL: &str = "ui_stop_tauri_runtime";

struct RuntimeState {
    runtime_id: String,
    project_root: PathBuf,
    module_root: PathBuf,
    command: String,
    launcher_process_id: u32,
    started_at: String,
    child: Child,
}

static RUNTIMES: OnceLock<Mutex<HashMap<String, RuntimeState>>> = OnceLock::new();

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(PREPARE_TOOL, "为已发现的 Tauri designSession 受控启动项目 CLI；重复调用只轮询本进程树中的原生窗口，不接管用户其他进程。", json!({
            "type":"object","additionalProperties":false,"required":["designSessionId"],
            "properties":{"designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},"restart":{"type":"boolean","default":false}}
        }), false),
        tool(CAPTURE_TOOL, "捕获由当前项目 Tauri Runtime 启动的最大可见原生窗口，保存 PNG、窗口边界、标题、PID 与 SHA-256；这才可声明 nativeHostVerified。", json!({
            "type":"object","additionalProperties":false,"required":["designSessionId"],
            "properties":{"designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"}}
        }), false),
        tool(STOP_TOOL, "只停止由当前 designSession 启动并登记的 Tauri Runtime 进程树，不影响用户自行启动的应用。", json!({
            "type":"object","additionalProperties":false,"required":["designSessionId"],
            "properties":{"designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"}}
        }), false),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, PREPARE_TOOL | CAPTURE_TOOL | STOP_TOOL)
}

pub(super) async fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    match name {
        PREPARE_TOOL => prepare(session, &arguments).await,
        CAPTURE_TOOL => capture(session, &arguments).await,
        STOP_TOOL => stop(session, &arguments).await,
        _ => bail!("未知 Tauri Runtime 工具: {name}"),
    }
}

async fn prepare(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let (root, mut record) = tauri_record(session, arguments)?;
    let restart = arguments
        .get("restart")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if restart {
        terminate_registered(&record.design_session_id).await?;
    }
    let mut runtimes = runtimes().lock().await;
    if let Some(state) = runtimes.get_mut(&record.design_session_id) {
        if let Some(exit) = state.child.try_wait()? {
            let details = runtime_view(state, "FAILED", None);
            runtimes.remove(&record.design_session_id);
            record.state = "TAURI_RUNTIME_FAILED".into();
            record.updated_at = chrono::Utc::now().to_rfc3339();
            persist_record(&root, &record)?;
            return Ok(
                json!({"ok":false,"status":"FAILED","exitCode":exit.code(),"runtime":details,"next":"修复 Tauri dev 命令或依赖后传 restart=true"}),
            );
        }
        let launcher = state.launcher_process_id;
        let view = runtime_view(state, "STARTING", None);
        drop(runtimes);
        let window = tokio::task::spawn_blocking(move || find_best_window(launcher)).await??;
        let status = if window.is_some() {
            "READY"
        } else {
            "STARTING"
        };
        record.state = format!("TAURI_RUNTIME_{status}");
        record.updated_at = chrono::Utc::now().to_rfc3339();
        persist_record(&root, &record)?;
        return Ok(
            json!({"ok":true,"status":status,"runtime":runtime_with_window(view, window.as_ref()),"retryAfterMs":if status == "STARTING" {2000} else {0}}),
        );
    }
    let module_root = tauri_module_root(&root, &record)?;
    let spec = launch_spec(&module_root)?;
    let log_dir = root
        .join(".elon/ui-tuner/headless-design/tauri")
        .join(&record.design_session_id);
    fs::create_dir_all(&log_dir)?;
    let log = fs::File::create(log_dir.join("runtime.log"))?;
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&module_root)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    crate::node_agent_cli_sidecar_runner::hide_tokio_command_window(&mut command);
    command.kill_on_drop(true);
    let child = command
        .spawn()
        .with_context(|| format!("无法启动 {}", spec.display))?;
    let launcher_process_id = child.id().context("Tauri CLI 未返回进程 ID")?;
    let state = RuntimeState {
        runtime_id: format!("tauri_{}", uuid::Uuid::new_v4().simple()),
        project_root: root.clone(),
        module_root,
        command: spec.display,
        launcher_process_id,
        started_at: chrono::Utc::now().to_rfc3339(),
        child,
    };
    let view = runtime_view(&state, "STARTING", None);
    runtimes.insert(record.design_session_id.clone(), state);
    record.state = "TAURI_RUNTIME_STARTING".into();
    record.updated_at = chrono::Utc::now().to_rfc3339();
    persist_record(&root, &record)?;
    Ok(
        json!({"ok":true,"status":"STARTING","runtime":view,"retryAfterMs":2000,"next":PREPARE_TOOL}),
    )
}

async fn capture(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let (root, mut record) = tauri_record(session, arguments)?;
    let (launcher, runtime_id, started_at) = {
        let mut runtimes = runtimes().lock().await;
        let state = runtimes
            .get_mut(&record.design_session_id)
            .ok_or_else(|| anyhow!("TAURI_RUNTIME_NOT_PREPARED：先调用 {PREPARE_TOOL}"))?;
        if let Some(exit) = state.child.try_wait()? {
            bail!(
                "TAURI_RUNTIME_EXITED：Tauri CLI 已退出 code={:?}",
                exit.code()
            );
        }
        (
            state.launcher_process_id,
            state.runtime_id.clone(),
            state.started_at.clone(),
        )
    };
    let window = tokio::task::spawn_blocking(move || find_best_window(launcher))
        .await??
        .ok_or_else(|| {
            anyhow!("TAURI_WINDOW_NOT_READY：尚未发现 Tauri 原生窗口，请继续轮询 {PREPARE_TOOL}")
        })?;
    let artifact_dir = root
        .join(".elon/ui-tuner/headless-design/tauri")
        .join(&record.design_session_id);
    fs::create_dir_all(&artifact_dir)?;
    let artifact_path = artifact_dir.join(format!(
        "native-{}.png",
        chrono::Utc::now().timestamp_millis()
    ));
    let capture_window = window.clone();
    let capture_path = artifact_path.clone();
    let bytes =
        tokio::task::spawn_blocking(move || capture_png(&capture_window, &capture_path)).await??;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let native = json!({
        "runtimeId":runtime_id,"nativeHostVerified":true,"hostCoverage":"TAURI_NATIVE_WINDOW",
        "artifact":{"path":artifact_path.to_string_lossy(),"sha256":sha256,"width":window.width,"height":window.height,"mediaType":"image/png"},
        "window":{"title":window.title,"processId":window.process_id,"bounds":{"left":window.left,"top":window.top,"width":window.width,"height":window.height}},
        "launcherProcessId":launcher,"runtimeStartedAt":started_at,"capturedAt":chrono::Utc::now().to_rfc3339(),"base64Embedded":false
    });
    let mut evidence = record.last_evidence.take().unwrap_or_else(|| json!({}));
    evidence
        .as_object_mut()
        .context("design session evidence 必须是对象")?
        .insert("nativeHost".into(), native.clone());
    record.last_evidence = Some(evidence);
    record.state = "TAURI_NATIVE_CAPTURED".into();
    record.updated_at = chrono::Utc::now().to_rfc3339();
    persist_record(&root, &record)?;
    Ok(
        json!({"ok":true,"status":"CAPTURED","designSessionId":record.design_session_id,"platform":"tauri","nativeHost":native}),
    )
}

async fn stop(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let (root, mut record) = tauri_record(session, arguments)?;
    let stopped = terminate_registered(&record.design_session_id).await?;
    record.state = if stopped {
        "TAURI_RUNTIME_STOPPED"
    } else {
        "READY_FOR_CAPTURE"
    }
    .into();
    record.updated_at = chrono::Utc::now().to_rfc3339();
    persist_record(&root, &record)?;
    Ok(
        json!({"ok":true,"status":if stopped {"STOPPED"} else {"NOT_RUNNING"},"designSessionId":record.design_session_id}),
    )
}

pub(super) fn native_artifact(
    session: &LiveUiSession,
    design_session_id: &str,
) -> Result<VerifiedPixelArtifact> {
    validate_design_session_id(design_session_id)?;
    let root = canonical_project_root(session)?;
    let record = read_record(&root, design_session_id)?;
    ensure_tauri(&record)?;
    let native = record
        .last_evidence
        .as_ref()
        .and_then(|value| value.get("nativeHost"))
        .context("Tauri designSession 还没有原生宿主证据")?;
    let path = PathBuf::from(
        native
            .pointer("/artifact/path")
            .and_then(Value::as_str)
            .context("原生宿主证据缺少 path")?,
    )
    .canonicalize()
    .context("原生宿主 PNG 不存在")?;
    let expected = native
        .pointer("/artifact/sha256")
        .and_then(Value::as_str)
        .context("原生宿主证据缺少 sha256")?;
    if !path.starts_with(&root) || fs::metadata(&path)?.len() > 64 * 1024 * 1024 {
        bail!("原生宿主 PNG 越出项目或超过大小上限");
    }
    let bytes = fs::read(path)?;
    let actual = hex::encode(Sha256::digest(&bytes));
    if !expected.eq_ignore_ascii_case(&actual) {
        bail!("原生宿主 PNG 哈希不匹配");
    }
    Ok(VerifiedPixelArtifact {
        bytes,
        media_type: "image/png".into(),
        sha256: actual,
    })
}

async fn terminate_registered(design_session_id: &str) -> Result<bool> {
    let state = runtimes().lock().await.remove(design_session_id);
    let Some(mut state) = state else {
        return Ok(false);
    };
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill.exe");
        command.args(["/PID", &state.launcher_process_id.to_string(), "/T", "/F"]);
        crate::node_agent_cli_sidecar_runner::hide_tokio_command_window(&mut command);
        let _ = command.output().await;
    }
    let _ = state.child.kill().await;
    Ok(true)
}

fn tauri_record(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<(PathBuf, DesignSessionRecord)> {
    let id = arguments
        .get("designSessionId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 designSessionId"))?;
    validate_design_session_id(id)?;
    let root = canonical_project_root(session)?;
    let record = read_record(&root, id)?;
    ensure_tauri(&record)?;
    Ok((root, record))
}

fn ensure_tauri(record: &DesignSessionRecord) -> Result<()> {
    if record.platform != DesignPlatform::Tauri {
        bail!("TAURI_DESIGN_SESSION_REQUIRED：该会话不是 Tauri 目标");
    }
    Ok(())
}

fn canonical_project_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("Tauri Runtime 需要绑定项目目录")?,
    )
    .canonicalize()
    .context("Tauri 项目目录不存在")
}

fn tauri_module_root(root: &Path, record: &DesignSessionRecord) -> Result<PathBuf> {
    let config = record
        .target
        .config_files
        .iter()
        .find(|path| path.replace('\\', "/").contains("/src-tauri/"))
        .ok_or_else(|| anyhow!("TAURI_CONFIG_NOT_FOUND：目标缺少 src-tauri 配置"))?;
    let normalized = config.replace('\\', "/");
    if normalized
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("Tauri 配置路径不安全");
    }
    let config_path = root
        .join(&normalized)
        .canonicalize()
        .context("Tauri 配置文件不存在")?;
    if !config_path.starts_with(root) || !config_path.is_file() {
        bail!("Tauri 配置越出项目");
    }
    config_path
        .parent()
        .and_then(Path::parent)
        .context("无法确定 Tauri 模块目录")?
        .canonicalize()
        .context("Tauri 模块目录不存在")
}

struct LaunchSpec {
    program: String,
    args: Vec<String>,
    display: String,
}

fn launch_spec(module: &Path) -> Result<LaunchSpec> {
    let (program, args) = if module.join("pnpm-lock.yaml").is_file() {
        ("pnpm.cmd", vec!["exec", "tauri", "dev"])
    } else if module.join("yarn.lock").is_file() {
        ("yarn.cmd", vec!["tauri", "dev"])
    } else if module.join("bun.lock").is_file() || module.join("bun.lockb").is_file() {
        ("bun.exe", vec!["x", "tauri", "dev"])
    } else if module.join("package.json").is_file() {
        ("npm.cmd", vec!["exec", "tauri", "--", "dev"])
    } else if module.join("src-tauri/Cargo.toml").is_file() {
        ("cargo.exe", vec!["tauri", "dev"])
    } else {
        bail!("TAURI_LAUNCHER_NOT_FOUND：模块缺少 package.json 或 src-tauri/Cargo.toml");
    };
    Ok(LaunchSpec {
        program: program.into(),
        args: args.iter().map(|value| value.to_string()).collect(),
        display: format!(
            "{} {}",
            program.trim_end_matches(".cmd").trim_end_matches(".exe"),
            args.join(" ")
        ),
    })
}

fn runtime_view(state: &RuntimeState, status: &str, window: Option<&NativeWindow>) -> Value {
    json!({"runtimeId":state.runtime_id,"status":status,"launcherProcessId":state.launcher_process_id,"projectRoot":state.project_root.to_string_lossy(),"moduleRoot":state.module_root.to_string_lossy(),"command":state.command,"startedAt":state.started_at,"window":window.map(window_view)})
}

fn runtime_with_window(mut view: Value, window: Option<&NativeWindow>) -> Value {
    view["window"] = window
        .map(window_view)
        .map(Value::from)
        .unwrap_or(Value::Null);
    view
}

fn window_view(window: &NativeWindow) -> Value {
    json!({"title":window.title,"processId":window.process_id,"bounds":{"left":window.left,"top":window.top,"width":window.width,"height":window.height}})
}

fn runtimes() -> &'static Mutex<HashMap<String, RuntimeState>> {
    RUNTIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{"readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":read_only,"openWorldHint":false}})
}
