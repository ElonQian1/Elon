use super::{
    artifact,
    browser::{self, BrowserIdentity},
    cdp::CdpClient,
    process::{locate_browser, BrowserProcess},
    security::{self, PreparedCapture},
    CaptureDiagnostic, PwaCaptureInput,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const MAX_ACTIVE_SESSIONS: usize = 4;
const MAX_OPERATIONS: u32 = 128;
const IDLE_TTL: Duration = Duration::from_secs(15 * 60);
const LIFETIME_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone)]
pub(crate) struct StatefulBrowserBinding {
    pub(crate) auth_profile: Option<String>,
    pub(crate) fixture_profile: Option<String>,
}

struct StatefulBrowser {
    runtime_id: String,
    project_root: PathBuf,
    target_origin: String,
    auth_profile: Option<String>,
    fixture_profile: Option<String>,
    viewport: (u32, u32, u64),
    page_session_id: String,
    browser: BrowserIdentity,
    process: BrowserProcess,
    cdp: CdpClient,
    operation_count: u32,
    created_at: String,
    last_used_at: String,
    created_clock: Instant,
    last_used_clock: Instant,
}

static SESSIONS: OnceLock<Mutex<HashMap<String, StatefulBrowser>>> = OnceLock::new();

pub(crate) async fn start(
    runtime_key: &str,
    project_root: &str,
    input: PwaCaptureInput,
    restart: bool,
) -> Value {
    let prepared = match security::prepare(project_root, input) {
        Ok(prepared) => prepared,
        Err(diagnostic) => return diagnostic.response(),
    };
    prune_expired().await;
    if restart {
        let _ = stop(runtime_key).await;
    } else if let Some(result) = existing_reuse(runtime_key, &prepared).await {
        return match result {
            Ok(view) => json!({"ok":true,"status":"READY","runtime":view,"reused":true}),
            Err(diagnostic) => diagnostic.response(),
        };
    }
    if sessions().lock().await.len() >= MAX_ACTIVE_SESSIONS {
        return diagnostic(
            "BROWSER_SESSION_CAPACITY_REACHED",
            "持久浏览器会话已达到节点上限",
            "停止不用的 designSession 浏览器后重试",
        );
    }
    let executable = match locate_browser() {
        Ok(executable) => executable,
        Err(diagnostic) => return diagnostic.response(),
    };
    let executable_name = executable
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("chromium")
        .to_string();
    let (mut process, socket) = match BrowserProcess::launch(&executable).await {
        Ok(value) => value,
        Err(diagnostic) => return diagnostic.response(),
    };
    let mut cdp = match cdp_client(&prepared, socket) {
        Ok(cdp) => cdp,
        Err(diagnostic) => return diagnostic.response(),
    };
    let total_timeout = capture_timeout(&prepared);
    let deadline = Instant::now() + total_timeout;
    let rendered = tokio::time::timeout(
        total_timeout,
        browser::render_page(&prepared, &mut cdp, deadline, executable_name),
    )
    .await;
    let mut rendered = match rendered {
        Ok(Ok(rendered)) => rendered,
        Ok(Err(diagnostic)) => {
            let (cleanup, stderr) = process.shutdown(&mut cdp).await;
            return diagnostic
                .with_detail("processCleanup", json!(cleanup))
                .with_detail("browserStderr", json!(stderr))
                .response();
        }
        Err(_) => {
            let (cleanup, stderr) = process.shutdown(&mut cdp).await;
            return CaptureDiagnostic::new(
                "CAPTURE_TIMEOUT",
                "持久浏览器首次捕获超过总时限",
                true,
                "缩短等待步骤，或停止当前 designSession 浏览器后重试",
            )
            .with_detail("processCleanup", json!(cleanup))
            .with_detail("browserStderr", json!(stderr))
            .response();
        }
    };
    let page_session_id = rendered.page_session_id.clone();
    let browser = rendered.browser.clone();
    rendered.process_cleanup.retained_for_stateful_session = true;
    let persisted = match artifact::persist(&prepared, rendered) {
        Ok(value) => value,
        Err(diagnostic) => {
            let _ = process.shutdown(&mut cdp).await;
            return diagnostic.response();
        }
    };
    let now = chrono::Utc::now().to_rfc3339();
    let runtime = StatefulBrowser {
        runtime_id: format!("browser_{}", uuid::Uuid::new_v4().simple()),
        project_root: prepared.project_root.clone(),
        target_origin: security::origin(&prepared.url).unwrap_or_default(),
        auth_profile: prepared.auth.profile.clone(),
        fixture_profile: prepared.fixture.profile.clone(),
        viewport: viewport_key(&prepared),
        page_session_id,
        browser,
        process,
        cdp,
        operation_count: 1,
        created_at: now.clone(),
        last_used_at: now,
        created_clock: Instant::now(),
        last_used_clock: Instant::now(),
    };
    let view = runtime_view(&runtime);
    sessions()
        .lock()
        .await
        .insert(runtime_key.to_string(), runtime);
    captured_response(&prepared, persisted, view, true)
}

pub(crate) async fn interact(
    runtime_key: &str,
    project_root: &str,
    input: PwaCaptureInput,
    navigate: bool,
) -> Value {
    let prepared = match security::prepare(project_root, input) {
        Ok(prepared) => prepared,
        Err(diagnostic) => return diagnostic.response(),
    };
    prune_expired().await;
    let mut sessions = sessions().lock().await;
    let Some(runtime) = sessions.get_mut(runtime_key) else {
        return diagnostic(
            "BROWSER_SESSION_NOT_PREPARED",
            "当前 designSession 没有持久浏览器",
            "先调用 ui_prepare_design_browser",
        );
    };
    if let Err(diagnostic) = validate_reuse(runtime, &prepared) {
        return diagnostic.response();
    }
    if runtime.operation_count >= MAX_OPERATIONS {
        return diagnostic(
            "BROWSER_SESSION_OPERATION_LIMIT",
            "持久浏览器已达到 128 次有界操作上限",
            "停止并重新准备该 designSession 浏览器",
        );
    }
    let deadline = Instant::now() + capture_timeout(&prepared);
    let rendered = browser::capture_existing_page(
        &prepared,
        &mut runtime.cdp,
        &runtime.page_session_id,
        runtime.browser.clone(),
        navigate,
        deadline,
    )
    .await;
    let mut rendered = match rendered {
        Ok(rendered) => rendered,
        Err(diagnostic) => return diagnostic.response(),
    };
    rendered.process_cleanup.retained_for_stateful_session = true;
    let persisted = match artifact::persist(&prepared, rendered) {
        Ok(value) => value,
        Err(diagnostic) => return diagnostic.response(),
    };
    runtime.operation_count = runtime.operation_count.saturating_add(1);
    runtime.last_used_clock = Instant::now();
    runtime.last_used_at = chrono::Utc::now().to_rfc3339();
    captured_response(&prepared, persisted, runtime_view(runtime), false)
}

pub(crate) async fn stop(runtime_key: &str) -> Value {
    let runtime = sessions().lock().await.remove(runtime_key);
    let Some(mut runtime) = runtime else {
        return json!({"ok":true,"status":"NOT_RUNNING","runtimeKey":runtime_key});
    };
    let view = runtime_view(&runtime);
    let (cleanup, stderr) = runtime.process.shutdown(&mut runtime.cdp).await;
    let complete = cleanup.browser_process_reaped && cleanup.temporary_profile_removed;
    json!({
        "ok":complete,"status":if complete {"STOPPED"} else {"STOP_INCOMPLETE"},
        "runtime":view,"processCleanup":cleanup,"browserStderr":stderr,
    })
}

async fn existing_reuse(
    runtime_key: &str,
    prepared: &PreparedCapture,
) -> Option<Result<Value, CaptureDiagnostic>> {
    sessions()
        .lock()
        .await
        .get(runtime_key)
        .map(|runtime| validate_reuse(runtime, prepared).map(|_| runtime_view(runtime)))
}

pub(crate) async fn binding(runtime_key: &str) -> Option<StatefulBrowserBinding> {
    sessions()
        .lock()
        .await
        .get(runtime_key)
        .map(|runtime| StatefulBrowserBinding {
            auth_profile: runtime.auth_profile.clone(),
            fixture_profile: runtime.fixture_profile.clone(),
        })
}

async fn prune_expired() {
    let expired = {
        let mut sessions = sessions().lock().await;
        let keys = sessions
            .iter()
            .filter(|(_, runtime)| {
                runtime.last_used_clock.elapsed() >= IDLE_TTL
                    || runtime.created_clock.elapsed() >= LIFETIME_TTL
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        keys.into_iter()
            .filter_map(|key| sessions.remove(&key))
            .collect::<Vec<_>>()
    };
    for mut runtime in expired {
        let _ = runtime.process.shutdown(&mut runtime.cdp).await;
    }
}

fn cdp_client(
    prepared: &PreparedCapture,
    socket: super::cdp::CdpSocket,
) -> Result<CdpClient, CaptureDiagnostic> {
    Ok(CdpClient::new(
        socket,
        prepared.allowed_origins.clone(),
        security::origin(&prepared.url)?,
        prepared.auth.headers.clone(),
    ))
}

fn validate_reuse(
    runtime: &StatefulBrowser,
    prepared: &PreparedCapture,
) -> Result<(), CaptureDiagnostic> {
    let same = runtime.project_root == prepared.project_root
        && runtime.target_origin == security::origin(&prepared.url)?
        && runtime.auth_profile == prepared.auth.profile
        && runtime.fixture_profile == prepared.fixture.profile
        && runtime.viewport == viewport_key(prepared);
    if !same {
        return Err(CaptureDiagnostic::new(
            "BROWSER_SESSION_BINDING_CHANGED",
            "持久浏览器的项目、origin、认证、fixture 或 viewport 已变化",
            false,
            "用 restart=true 重新准备 designSession 浏览器，禁止跨绑定复用",
        ));
    }
    Ok(())
}

fn viewport_key(prepared: &PreparedCapture) -> (u32, u32, u64) {
    (
        prepared.viewport.width,
        prepared.viewport.height,
        prepared.viewport.device_scale_factor.to_bits(),
    )
}

fn capture_timeout(prepared: &PreparedCapture) -> Duration {
    Duration::from_millis(
        prepared
            .wait_for
            .timeout_ms
            .saturating_add(prepared.interaction_timeout_ms)
            .saturating_add(10_000),
    )
}

fn runtime_view(runtime: &StatefulBrowser) -> Value {
    json!({
        "runtimeId":runtime.runtime_id,"status":"READY","projectRoot":runtime.project_root,
        "targetOrigin":runtime.target_origin,"authProfile":runtime.auth_profile,
        "fixtureProfile":runtime.fixture_profile,"operationCount":runtime.operation_count,
        "createdAt":runtime.created_at,"lastUsedAt":runtime.last_used_at,
        "limits":{"maxActiveSessions":MAX_ACTIVE_SESSIONS,"maxOperations":MAX_OPERATIONS,
            "idleTtlSeconds":IDLE_TTL.as_secs(),"lifetimeTtlSeconds":LIFETIME_TTL.as_secs()},
        "statePreserved":true,"base64Embedded":false,
    })
}

fn captured_response(
    prepared: &PreparedCapture,
    result: artifact::PersistedCapture,
    runtime: Value,
    prepared_now: bool,
) -> Value {
    let pixels_path = result.artifact.path.clone();
    let pixels_sha = result.artifact.sha256.clone();
    let tree_path = result.semantic_tree.path.clone();
    let tree_sha = result.semantic_tree.sha256.clone();
    json!({
        "ok":true,"status":"CAPTURED","runtime":runtime,"preparedNow":prepared_now,
        "artifact":result.artifact,"uiTree":result.semantic_tree,"route":result.route,
        "revision":prepared.evidence,"browser":result.browser,"viewport":result.viewport,
        "networkPolicy":result.network_policy,
        "authentication":{"mode":prepared.auth.mode,"profile":prepared.auth.profile},
        "testData":{"fixtureProfile":prepared.fixture.profile,"formValuesEmbedded":false},
        "interaction":{"executedStepCount":result.executed_step_count},
        "processLifecycle":result.process_cleanup,
        "pageDiagnostics":result.page_diagnostics,
        "contextPackReference":{"path":pixels_path,"sha256":pixels_sha,
            "pixels":{"path":pixels_path,"sha256":pixels_sha},
            "uiTree":{"path":tree_path,"sha256":tree_sha},"embedBase64":false,
            "preferredReadOrder":["uiTree","pixels"]},
        "base64Embedded":false,
    })
}

fn diagnostic(code: &'static str, message: &str, next: &str) -> Value {
    CaptureDiagnostic::new(code, message, true, next).response()
}

fn sessions() -> &'static Mutex<HashMap<String, StatefulBrowser>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
