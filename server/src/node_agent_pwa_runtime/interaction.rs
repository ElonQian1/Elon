use super::{
    cdp::{short_pause, CdpClient},
    security::PreparedCapture,
    CaptureDiagnostic, CaptureInteractionStep,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub(super) async fn execute_steps(
    prepared: &PreparedCapture,
    cdp: &mut CdpClient,
    session: &str,
    deadline: Instant,
) -> Result<usize, CaptureDiagnostic> {
    for (index, step) in prepared.steps.iter().enumerate() {
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
            CaptureInteractionStep::Fill {
                selector,
                fixture_key,
            } => {
                let selector = json_string(selector, index, "fill selector")?;
                let value = fixture_value(prepared, fixture_key, index)?;
                let expression = format!(
                    r#"(() => {{ try {{ const element = document.querySelector({selector}); if (!element) return "missing"; const type = (element.getAttribute("type") || "").toLowerCase(); if (["password","hidden","file"].includes(type)) return "forbidden"; if (!(element instanceof HTMLInputElement) && !(element instanceof HTMLTextAreaElement) && !element.isContentEditable) return "unsupported"; const value = {value}; if (element.isContentEditable) element.textContent = value; else element.value = value; element.dispatchEvent(new Event("input", {{bubbles:true}})); element.dispatchEvent(new Event("change", {{bubbles:true}})); return "filled"; }} catch (_) {{ return "invalid"; }} }})()"#
                );
                expect_status(
                    evaluate_value(cdp, session, &expression, deadline).await?,
                    index,
                    "filled",
                    "fill",
                )?;
            }
            CaptureInteractionStep::SelectOption {
                selector,
                fixture_key,
            } => {
                let selector = json_string(selector, index, "selectOption selector")?;
                let value = fixture_value(prepared, fixture_key, index)?;
                let expression = format!(
                    r#"(() => {{ try {{ const element = document.querySelector({selector}); if (!element) return "missing"; if (!(element instanceof HTMLSelectElement)) return "unsupported"; element.value = {value}; if (element.value !== {value}) return "option-missing"; element.dispatchEvent(new Event("input", {{bubbles:true}})); element.dispatchEvent(new Event("change", {{bubbles:true}})); return "selected"; }} catch (_) {{ return "invalid"; }} }})()"#
                );
                expect_status(
                    evaluate_value(cdp, session, &expression, deadline).await?,
                    index,
                    "selected",
                    "selectOption",
                )?;
            }
            CaptureInteractionStep::SetChecked { selector, checked } => {
                let selector = json_string(selector, index, "setChecked selector")?;
                let expression = format!(
                    r#"(() => {{ try {{ const element = document.querySelector({selector}); if (!element) return "missing"; if (!(element instanceof HTMLInputElement) || !["checkbox","radio"].includes(element.type)) return "unsupported"; const expected = {checked}; if (element.checked !== expected) element.click(); return element.checked === expected ? "checked" : "mismatch"; }} catch (_) {{ return "invalid"; }} }})()"#
                );
                expect_status(
                    evaluate_value(cdp, session, &expression, deadline).await?,
                    index,
                    "checked",
                    "setChecked",
                )?;
            }
            CaptureInteractionStep::PressKey { selector, key } => {
                if let Some(selector) = selector {
                    let selector = json_string(selector, index, "pressKey selector")?;
                    let expression = format!(
                        r#"(() => {{ try {{ const element = document.querySelector({selector}); if (!element) return "missing"; if (typeof element.focus !== "function") return "unsupported"; element.focus(); return document.activeElement === element ? "focused" : "focus-failed"; }} catch (_) {{ return "invalid"; }} }})()"#
                    );
                    expect_status(
                        evaluate_value(cdp, session, &expression, deadline).await?,
                        index,
                        "focused",
                        "pressKey",
                    )?;
                }
                dispatch_key(cdp, session, key, deadline).await?;
            }
            CaptureInteractionStep::ScrollIntoView { selector } => {
                let selector = json_string(selector, index, "scrollIntoView selector")?;
                let expression = format!(
                    r#"(() => {{ try {{ const element = document.querySelector({selector}); if (!element) return "missing"; element.scrollIntoView({{block:"center",inline:"center",behavior:"instant"}}); return "scrolled"; }} catch (_) {{ return "invalid"; }} }})()"#
                );
                expect_status(
                    evaluate_value(cdp, session, &expression, deadline).await?,
                    index,
                    "scrolled",
                    "scrollIntoView",
                )?;
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
            CaptureInteractionStep::PreviewStyle { selector, patches } => {
                let value =
                    super::style_preview::preview(cdp, session, selector, patches, deadline)
                        .await?;
                expect_status(value, index, "previewed", "previewStyle")?;
            }
            CaptureInteractionStep::RestoreStyle { selector } => {
                let value = super::style_preview::restore(cdp, session, selector, deadline).await?;
                expect_status(value, index, "restored", "restoreStyle")?;
            }
        }
    }
    Ok(prepared.steps.len())
}

fn fixture_value(
    prepared: &PreparedCapture,
    fixture_key: &str,
    index: usize,
) -> Result<String, CaptureDiagnostic> {
    let value = prepared
        .fixture
        .form_values
        .get(fixture_key)
        .ok_or_else(|| {
            interaction_error(
                index,
                "fixtureKey 不存在；表单值必须来自 fixtureProfile.formValues",
            )
        })?;
    json_string(value, index, "fixture value")
}

fn json_string(value: &str, index: usize, field: &str) -> Result<String, CaptureDiagnostic> {
    serde_json::to_string(value)
        .map_err(|_| interaction_error(index, &format!("无法序列化 {field}")))
}

fn expect_status(
    value: Value,
    index: usize,
    expected: &str,
    action: &str,
) -> Result<(), CaptureDiagnostic> {
    if value.as_str() == Some(expected) {
        return Ok(());
    }
    let reason = match value.as_str() {
        Some("missing") => "selector 在页面中不存在",
        Some("forbidden") => "目标是密码、隐藏或文件输入，安全策略拒绝填充",
        Some("unsupported") => "目标元素类型不支持该操作",
        Some("option-missing") => "fixture 值不是目标 select 的可用选项",
        Some("mismatch") => "操作后元素状态与期望不一致",
        Some("focus-failed") => "目标元素无法获得焦点",
        _ => "selector 或页面状态无效",
    };
    Err(interaction_error(index, &format!("{action} {reason}")))
}

async fn dispatch_key(
    cdp: &mut CdpClient,
    session: &str,
    key: &str,
    deadline: Instant,
) -> Result<(), CaptureDiagnostic> {
    let text = if key == "Space" { " " } else { "" };
    let key = if key == "Space" { " " } else { key };
    for event_type in ["keyDown", "keyUp"] {
        cdp.command(
            "Input.dispatchKeyEvent",
            json!({"type":event_type,"key":key,"text":if event_type == "keyDown" {text} else {""}}),
            Some(session),
            deadline,
        )
        .await?;
    }
    Ok(())
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
