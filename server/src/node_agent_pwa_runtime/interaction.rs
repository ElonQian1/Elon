use super::{
    cdp::{short_pause, CdpClient},
    CaptureDiagnostic, CaptureInteractionStep,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub(super) async fn execute_steps(
    steps: &[CaptureInteractionStep],
    cdp: &mut CdpClient,
    session: &str,
    deadline: Instant,
) -> Result<usize, CaptureDiagnostic> {
    for (index, step) in steps.iter().enumerate() {
        match step {
            CaptureInteractionStep::Click { selector } => {
                let selector = serde_json::to_string(selector)
                    .map_err(|_| interaction_error(index, "无法序列化 click selector"))?;
                let expression = format!(
                    r#"(() => {{ try {{ const element = document.querySelector({selector}); if (!element) return "missing"; element.click(); return "clicked"; }} catch (_) {{ return "invalid"; }} }})()"#
                );
                let value = evaluate_value(cdp, session, &expression, deadline).await?;
                match value.as_str() {
                    Some("clicked") => {}
                    Some("missing") => {
                        return Err(interaction_error(index, "click selector 在页面中不存在"))
                    }
                    _ => return Err(interaction_error(index, "click selector 无效")),
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            CaptureInteractionStep::WaitFor {
                selector,
                state,
                timeout_ms,
            } => {
                let selector = serde_json::to_string(selector)
                    .map_err(|_| interaction_error(index, "无法序列化 waitFor selector"))?;
                let state = serde_json::to_string(state)
                    .map_err(|_| interaction_error(index, "无法序列化 waitFor state"))?;
                let expression = format!(
                    r#"(() => {{ try {{ const element = document.querySelector({selector}); const state = {state}; if (state === "hidden") return !element || getComputedStyle(element).display === "none" || getComputedStyle(element).visibility === "hidden" || element.getBoundingClientRect().width <= 0 || element.getBoundingClientRect().height <= 0; if (state === "attached") return !!element; return !!element && getComputedStyle(element).display !== "none" && getComputedStyle(element).visibility !== "hidden" && element.getBoundingClientRect().width > 0 && element.getBoundingClientRect().height > 0; }} catch (_) {{ return null; }} }})()"#
                );
                let step_deadline =
                    (Instant::now() + Duration::from_millis(*timeout_ms)).min(deadline);
                loop {
                    let value = evaluate_value(cdp, session, &expression, step_deadline).await?;
                    if value.is_null() {
                        return Err(interaction_error(index, "waitFor selector 无效"));
                    }
                    if value.as_bool() == Some(true) {
                        break;
                    }
                    if Instant::now() >= step_deadline {
                        return Err(interaction_error(index, "waitFor 条件超时"));
                    }
                    short_pause().await;
                }
            }
            CaptureInteractionStep::AssertText { selector, text } => {
                let selector = serde_json::to_string(selector)
                    .map_err(|_| interaction_error(index, "无法序列化 assertText selector"))?;
                let expected = serde_json::to_string(text)
                    .map_err(|_| interaction_error(index, "无法序列化 assertText 文本"))?;
                let expression = format!(
                    r#"(() => {{ try {{ const element = document.querySelector({selector}); if (!element) return "missing"; return (element.textContent || "").includes({expected}) ? "matched" : "mismatch"; }} catch (_) {{ return "invalid"; }} }})()"#
                );
                let value = evaluate_value(cdp, session, &expression, deadline).await?;
                match value.as_str() {
                    Some("matched") => {}
                    Some("missing") => {
                        return Err(interaction_error(
                            index,
                            "assertText selector 在页面中不存在",
                        ))
                    }
                    Some("mismatch") => {
                        return Err(interaction_error(index, "assertText 文本断言不匹配"))
                    }
                    _ => return Err(interaction_error(index, "assertText selector 无效")),
                }
            }
        }
    }
    Ok(steps.len())
}

async fn evaluate_value(
    cdp: &mut CdpClient,
    session: &str,
    expression: &str,
    deadline: Instant,
) -> Result<Value, CaptureDiagnostic> {
    cdp.command(
        "Runtime.evaluate",
        json!({"expression":expression,"returnByValue":true}),
        Some(session),
        deadline,
    )
    .await?
    .pointer("/result/value")
    .cloned()
    .ok_or_else(|| protocol_error("PWA 交互没有返回可验证结果"))
}

fn interaction_error(index: usize, message: &str) -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "INTERACTION_STEP_FAILED",
        format!("PWA 交互步骤 {} 失败：{message}", index + 1),
        true,
        "检查稳定 selector、页面初始数据和步骤顺序后重试；不要改用任意脚本绕过",
    )
    .with_detail("stepIndex", json!(index))
}

fn protocol_error(message: &str) -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "BROWSER_PROTOCOL_ERROR",
        message,
        true,
        "重启 Windows 节点或升级本机 Edge/Chrome 后重试",
    )
}
