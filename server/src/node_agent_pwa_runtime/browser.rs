use super::{
    cdp::{number, safe_browser_field, short_pause, CdpClient, NetworkState},
    process::{locate_browser, BrowserProcess},
    security::{self, PreparedCapture, SanitizedRoute},
    CaptureDiagnostic,
};
use base64::Engine as _;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const MAX_PNG_BYTES: usize = 32 * 1024 * 1024;
const MAX_CAPTURE_SIDE: f64 = 16_384.0;
const MAX_CAPTURE_PIXELS: f64 = 40_000_000.0;

pub(super) struct RenderedCapture {
    pub(super) png: Vec<u8>,
    pub(super) route: SanitizedRoute,
    pub(super) browser: BrowserIdentity,
    pub(super) css_width: f64,
    pub(super) css_height: f64,
    pub(super) blocked_request_count: u32,
    pub(super) process_cleanup: ProcessCleanup,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrowserIdentity {
    pub(super) product: String,
    pub(super) revision: String,
    pub(super) user_agent: String,
    pub(super) protocol_version: String,
    pub(super) js_version: String,
    pub(super) executable_name: String,
    pub(super) headless: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProcessCleanup {
    pub(super) browser_process_reaped: bool,
    pub(super) temporary_profile_removed: bool,
}

pub(super) async fn render(
    prepared: &PreparedCapture,
) -> Result<RenderedCapture, CaptureDiagnostic> {
    let executable = locate_browser()?;
    let executable_name = executable
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("chromium")
        .to_string();
    let (mut browser, socket) = BrowserProcess::launch(&executable).await?;
    let mut cdp = CdpClient::new(
        socket,
        prepared.allowed_origins.clone(),
        security::origin(&prepared.url)?,
        prepared.auth.headers.clone(),
    );
    let deadline = Instant::now() + Duration::from_millis(prepared.wait_for.timeout_ms + 10_000);
    let result = tokio::time::timeout(
        Duration::from_millis(prepared.wait_for.timeout_ms + 10_000),
        render_page(prepared, &mut cdp, deadline, executable_name),
    )
    .await
    .unwrap_or_else(|_| {
        Err(CaptureDiagnostic::new(
            "CAPTURE_TIMEOUT",
            "无头浏览器总捕获时限已到",
            true,
            "缩短页面等待、修复卡住的本机 PWA，或在 500..120000ms 内调整 timeoutMs",
        ))
    });
    let cleanup = browser.shutdown(&mut cdp).await;
    if !cleanup.browser_process_reaped || !cleanup.temporary_profile_removed {
        return Err(CaptureDiagnostic::new(
            "BROWSER_CLEANUP_FAILED",
            "无头浏览器进程或临时会话目录未能完整回收",
            true,
            "确认安全软件未锁定临时目录后重试；不要复用本次 authProfile 会话",
        ));
    }
    result.map(|mut rendered| {
        rendered.process_cleanup = cleanup;
        rendered
    })
}

async fn render_page(
    prepared: &PreparedCapture,
    cdp: &mut CdpClient,
    deadline: Instant,
    executable_name: String,
) -> Result<RenderedCapture, CaptureDiagnostic> {
    let version = cdp
        .command("Browser.getVersion", json!({}), None, deadline)
        .await?;
    let browser = BrowserIdentity {
        product: safe_browser_field(&version, "product"),
        revision: safe_browser_field(&version, "revision"),
        user_agent: safe_browser_field(&version, "userAgent"),
        protocol_version: safe_browser_field(&version, "protocolVersion"),
        js_version: safe_browser_field(&version, "jsVersion"),
        executable_name,
        headless: true,
    };
    let target = cdp
        .command(
            "Target.createTarget",
            json!({"url":"about:blank"}),
            None,
            deadline,
        )
        .await?
        .get("targetId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| protocol_error("浏览器未返回 targetId"))?;
    let session = cdp
        .command(
            "Target.attachToTarget",
            json!({"targetId":target,"flatten":true}),
            None,
            deadline,
        )
        .await?
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| protocol_error("浏览器未返回 sessionId"))?;
    for method in ["Page.enable", "Runtime.enable", "Network.enable"] {
        cdp.command(method, json!({}), Some(&session), deadline)
            .await?;
    }
    cdp.command(
        "Fetch.enable",
        json!({"patterns":[
            {"urlPattern":"http://*","requestStage":"Request"},
            {"urlPattern":"https://*","requestStage":"Request"}
        ]}),
        Some(&session),
        deadline,
    )
    .await?;
    cdp.command(
        "Emulation.setDeviceMetricsOverride",
        json!({
            "width":prepared.viewport.width,"height":prepared.viewport.height,
            "deviceScaleFactor":prepared.viewport.device_scale_factor,"mobile":false,
            "screenWidth":prepared.viewport.width,"screenHeight":prepared.viewport.height
        }),
        Some(&session),
        deadline,
    )
    .await?;
    for cookie in &prepared.auth.cookies {
        let accepted = cdp
            .command(
                "Network.setCookie",
                json!({
                    "name":cookie.name,"value":cookie.value,"url":prepared.url.as_str(),
                    "path":cookie.path,"httpOnly":cookie.http_only,"secure":cookie.secure
                }),
                Some(&session),
                deadline,
            )
            .await?
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !accepted {
            return Err(CaptureDiagnostic::new(
                "AUTH_PROFILE_REJECTED",
                "浏览器拒绝 authProfile 中的 Cookie",
                false,
                "检查 Cookie path/secure 与目标本机 PWA origin 是否匹配",
            ));
        }
    }
    let navigation = cdp
        .command(
            "Page.navigate",
            json!({"url":prepared.url.as_str()}),
            Some(&session),
            deadline,
        )
        .await?;
    if navigation
        .get("errorText")
        .and_then(Value::as_str)
        .is_some()
    {
        return Err(CaptureDiagnostic::new(
            "NAVIGATION_FAILED",
            "本机 PWA 导航失败",
            true,
            "确认本机开发服务器已启动、端口可达且没有跳转到未授权 origin",
        ));
    }
    let wait_deadline = Instant::now() + Duration::from_millis(prepared.wait_for.timeout_ms);
    let final_url = wait_for_page(prepared, cdp, &session, wait_deadline).await?;
    let final_url =
        reqwest::Url::parse(&final_url).map_err(|_| protocol_error("页面返回了无效最终 URL"))?;
    if !prepared
        .allowed_origins
        .contains(&security::origin(&final_url)?)
    {
        return Err(CaptureDiagnostic::new(
            "UNTRUSTED_REDIRECT",
            "PWA 导航试图离开项目允许的 origin",
            false,
            "修正重定向，或仅在项目配置中显式登记确实可信的 origin",
        ));
    }
    let route = security::sanitize_url(&final_url)?;
    let (clip, css_width, css_height) = capture_geometry(prepared, cdp, &session, deadline).await?;
    validate_geometry(css_width, css_height, prepared.viewport.device_scale_factor)?;
    let mut params = json!({
        "format":"png","fromSurface":true,
        "captureBeyondViewport":prepared.capture.full_page || prepared.capture.selector.is_some()
    });
    if let Some(clip) = clip {
        params["clip"] = clip;
    }
    let data = cdp
        .command("Page.captureScreenshot", params, Some(&session), deadline)
        .await?
        .get("data")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| protocol_error("浏览器未返回 PNG 数据"))?;
    if data.len() > MAX_PNG_BYTES * 2 {
        return Err(output_limit());
    }
    let png = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| protocol_error("浏览器返回的 PNG Base64 无法解码"))?;
    if png.is_empty() || png.len() > MAX_PNG_BYTES {
        return Err(output_limit());
    }
    Ok(RenderedCapture {
        png,
        route,
        browser,
        css_width,
        css_height,
        blocked_request_count: cdp.blocked_request_count,
        process_cleanup: ProcessCleanup {
            browser_process_reaped: false,
            temporary_profile_removed: false,
        },
    })
}

async fn wait_for_page(
    prepared: &PreparedCapture,
    cdp: &mut CdpClient,
    session: &str,
    deadline: Instant,
) -> Result<String, CaptureDiagnostic> {
    let wait_selector = serde_json::to_string(&prepared.wait_for.selector).unwrap_or("null".into());
    let auth_selector =
        serde_json::to_string(&prepared.auth.ready_selector).unwrap_or("null".into());
    let expression = format!(
        r#"(() => {{
          const waitSelector = {wait_selector}; const authSelector = {auth_selector};
          const query = (selector) => {{ if (!selector) return true; try {{ return !!document.querySelector(selector); }} catch (_) {{ return null; }} }};
          return {{ readyState: document.readyState, waitFound: query(waitSelector), authReady: query(authSelector),
            authForm: !!document.querySelector('input[type="password"], form[action*="login" i], form[action*="signin" i]'), href: location.href }};
        }})()"#
    );
    let mut network = NetworkState::new();
    loop {
        if Instant::now() >= deadline {
            return Err(wait_timeout(prepared));
        }
        let result = cdp
            .command(
                "Runtime.evaluate",
                json!({"expression":expression,"returnByValue":true}),
                Some(session),
                deadline,
            )
            .await?;
        network.consume(&mut cdp.events);
        let value = result
            .pointer("/result/value")
            .ok_or_else(|| protocol_error("页面状态探测没有返回 value"))?;
        if value.get("waitFound").is_some_and(Value::is_null)
            || value.get("authReady").is_some_and(Value::is_null)
        {
            return Err(invalid_selector());
        }
        let href = value
            .get("href")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let auth_failed = network
            .document_status
            .is_some_and(|status| matches!(status, 401 | 403))
            || value
                .get("authForm")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            || looks_like_login_route(href);
        if auth_failed {
            return Err(auth_failure(prepared));
        }
        let ready_state = value
            .get("readyState")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let ready = match prepared.wait_for.condition.as_str() {
            "domcontentloaded" => matches!(ready_state, "interactive" | "complete"),
            "load" => ready_state == "complete",
            _ => {
                ready_state == "complete"
                    && network.inflight.is_empty()
                    && network.last_activity.elapsed()
                        >= Duration::from_millis(prepared.wait_for.settle_ms)
            }
        };
        let wait_found = value
            .get("waitFound")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let auth_ready = value
            .get("authReady")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if ready && wait_found && auth_ready && !href.is_empty() {
            return Ok(href.to_string());
        }
        short_pause().await;
    }
}

async fn capture_geometry(
    prepared: &PreparedCapture,
    cdp: &mut CdpClient,
    session: &str,
    deadline: Instant,
) -> Result<(Option<Value>, f64, f64), CaptureDiagnostic> {
    if let Some(selector) = prepared.capture.selector.as_deref() {
        let selector = serde_json::to_string(selector).unwrap_or("null".into());
        let expression = format!(
            r#"(() => {{ try {{ const el = document.querySelector({selector}); if (!el) return null; const r = el.getBoundingClientRect(); return {{x:r.left+scrollX,y:r.top+scrollY,width:r.width,height:r.height}}; }} catch (_) {{ return {{invalid:true}}; }} }})()"#
        );
        let result = cdp
            .command(
                "Runtime.evaluate",
                json!({"expression":expression,"returnByValue":true}),
                Some(session),
                deadline,
            )
            .await?;
        let value = result
            .pointer("/result/value")
            .cloned()
            .unwrap_or(Value::Null);
        if value.is_null() {
            return Err(CaptureDiagnostic::new(
                "CAPTURE_SELECTOR_NOT_FOUND",
                "capture.selector 在真实页面中不存在",
                true,
                "确认 route 和 selector 后重试",
            ));
        }
        if value.get("invalid").is_some() {
            return Err(invalid_selector());
        }
        let width = number(&value, "width")?;
        let height = number(&value, "height")?;
        return Ok((
            Some(json!({
                "x":number(&value,"x")?,"y":number(&value,"y")?,
                "width":width,"height":height,"scale":1
            })),
            width,
            height,
        ));
    }
    if prepared.capture.full_page {
        let metrics = cdp
            .command("Page.getLayoutMetrics", json!({}), Some(session), deadline)
            .await?;
        let content = metrics
            .get("cssContentSize")
            .or_else(|| metrics.get("contentSize"))
            .ok_or_else(|| protocol_error("浏览器未返回页面 content size"))?;
        let width = number(content, "width")?;
        let height = number(content, "height")?;
        return Ok((
            Some(json!({"x":0,"y":0,"width":width,"height":height,"scale":1})),
            width,
            height,
        ));
    }
    Ok((
        None,
        f64::from(prepared.viewport.width),
        f64::from(prepared.viewport.height),
    ))
}

fn validate_geometry(width: f64, height: f64, scale: f64) -> Result<(), CaptureDiagnostic> {
    let pixels = width * height * scale.powi(2);
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || width > MAX_CAPTURE_SIDE
        || height > MAX_CAPTURE_SIDE
        || pixels > MAX_CAPTURE_PIXELS
    {
        return Err(CaptureDiagnostic::new(
            "CAPTURE_DIMENSION_LIMIT",
            "实际捕获范围为空或超过单边 16384/总计 4000 万像素上限",
            false,
            "缩小页面、viewport 或改用 capture.selector",
        ));
    }
    Ok(())
}

fn looks_like_login_route(raw: &str) -> bool {
    reqwest::Url::parse(raw).ok().is_some_and(|url| {
        url.path().split('/').any(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                "login" | "signin" | "sign-in"
            )
        })
    })
}

fn auth_failure(prepared: &PreparedCapture) -> CaptureDiagnostic {
    if prepared.auth.profile.is_some() {
        CaptureDiagnostic::new(
            "AUTHENTICATION_FAILED",
            "已准备 authProfile，但页面仍停留在登录/未授权状态",
            false,
            "在本机刷新该 profile 的 Cookie/header，并配置 authenticatedReadySelector 后重试",
        )
    } else {
        CaptureDiagnostic::new(
            "AUTHENTICATION_REQUIRED",
            "真实页面需要认证，未将登录页误报为成功画面",
            false,
            "在 .elon/ui-tuner/pwa-sessions/<profile>.json 准备 version=1 会话，并通过 authProfile 名称引用；禁止把秘密放进 URL/MCP 参数",
        )
    }
}

fn wait_timeout(prepared: &PreparedCapture) -> CaptureDiagnostic {
    if prepared.auth.ready_selector.is_some() && prepared.auth.profile.is_some() {
        return CaptureDiagnostic::new(
            "AUTHENTICATION_FAILED",
            "认证就绪 selector 在超时前未出现",
            false,
            "刷新本机会话 profile，或修正 authenticatedReadySelector",
        );
    }
    CaptureDiagnostic::new(
        "WAIT_TIMEOUT",
        "页面等待条件或 selector 在 timeoutMs 内未满足",
        true,
        "检查本机 PWA 状态、等待 selector 和网络请求后重试",
    )
}

fn output_limit() -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "OUTPUT_SIZE_LIMIT",
        "无头浏览器 PNG 必须在 1..32MiB",
        false,
        "缩小 viewport、关闭 fullPage 或改用 capture.selector",
    )
}

fn invalid_selector() -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "SELECTOR_INVALID",
        "浏览器拒绝了等待、认证或捕获 selector",
        false,
        "修正 CSS selector 后重试",
    )
}

fn protocol_error(message: &str) -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "BROWSER_PROTOCOL_ERROR",
        message,
        true,
        "重启 Windows 节点或升级本机 Edge/Chrome 后重试",
    )
}
