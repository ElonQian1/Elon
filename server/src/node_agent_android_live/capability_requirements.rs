use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

#[derive(Debug, Default, PartialEq)]
pub(super) struct CapabilityRequirements {
    pub(super) declared: Vec<String>,
    pub(super) derived: Vec<String>,
    pub(super) effective: Vec<String>,
    pub(super) reasons: Vec<Value>,
}

pub(super) fn requested_capabilities(
    arguments: &Value,
    task: Option<&Value>,
    profile: Option<&Value>,
) -> Result<CapabilityRequirements> {
    let declared = if arguments.get("requiredCapabilities").is_some() {
        normalize_capabilities(&string_array(arguments, "requiredCapabilities", 1, 32)?)
    } else {
        Vec::new()
    };
    let (derived, reasons) = derive_capabilities(task, profile);
    let effective = normalize_capabilities(
        &declared
            .iter()
            .chain(derived.iter())
            .cloned()
            .collect::<Vec<_>>(),
    );
    Ok(CapabilityRequirements {
        declared,
        derived,
        effective,
        reasons,
    })
}

fn derive_capabilities(task: Option<&Value>, profile: Option<&Value>) -> (Vec<String>, Vec<Value>) {
    let mode = task
        .and_then(|value| value.pointer("/task/task/mode"))
        .and_then(Value::as_str)
        .unwrap_or("AUTO");
    let mut required = vec![
        "DESKTOP_TASK_IMPORT",
        "PROJECT_UI_PROFILE",
        "CODEX_SOURCE_HANDOFF",
        "PATCH_FREE_BUILD_VERIFY",
    ];
    let mut reasons = vec![json!({
        "reason": "UI_WORKFLOW_BASELINE",
        "capabilities": required,
    })];
    if mode == "CREATE_NEW" {
        required.push("NEW_SCREEN_BOOTSTRAP");
        reasons.push(json!({
            "reason": "CREATE_NEW_SCREEN",
            "capabilities": ["NEW_SCREEN_BOOTSTRAP"],
        }));
    }
    if is_android_profile(profile) {
        required.push("REAL_ANDROID_RENDERER");
        reasons.push(json!({
            "reason": "ANDROID_UI_PROJECT",
            "capabilities": ["REAL_ANDROID_RENDERER"],
        }));
    }
    if task_has_target_design(task) {
        required.extend([
            "TARGET_DESIGN_BINDING",
            "REAL_ANDROID_RENDERER",
            "LOCAL_VISUAL_SOLVER",
            "PERSISTENT_FIT_RUN",
        ]);
        reasons.push(json!({
            "reason": "CLEAN_TARGET_DESIGN_PRESENT",
            "capabilities": [
                "TARGET_DESIGN_BINDING",
                "REAL_ANDROID_RENDERER",
                "LOCAL_VISUAL_SOLVER",
                "PERSISTENT_FIT_RUN"
            ],
        }));
    }
    let request = task
        .and_then(|value| value.pointer("/task/task/request"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if profile_requires_apk_web_parity(profile) || request_requires_cross_platform_parity(request) {
        required.push("CROSS_PLATFORM_STYLE_WRITEBACK");
        reasons.push(json!({
            "reason": if profile_requires_apk_web_parity(profile) {
                "PROJECT_REQUIRES_APK_WEB_UI_PARITY"
            } else {
                "TASK_REQUESTS_CROSS_PLATFORM_PARITY"
            },
            "capabilities": ["CROSS_PLATFORM_STYLE_WRITEBACK"],
        }));
    }
    if request_requires_pwa_generation(request) {
        required.push("PWA_CODE_GENERATION");
        reasons.push(json!({
            "reason": if mode == "CREATE_NEW" {
                "TASK_REQUESTS_NEW_PWA_SCREEN"
            } else {
                "TASK_REQUESTS_PWA_SOURCE_WRITEBACK"
            },
            "capabilities": ["PWA_CODE_GENERATION"],
        }));
    }
    (
        normalize_capabilities(&required.into_iter().map(str::to_string).collect::<Vec<_>>()),
        reasons,
    )
}

fn is_android_profile(profile: Option<&Value>) -> bool {
    profile.is_some_and(super::design_bootstrap::is_android_project_profile)
}

fn profile_requires_apk_web_parity(profile: Option<&Value>) -> bool {
    profile.and_then(|value| {
        value
            .pointer("/capabilities/apkWebUiParityRequired")
            .and_then(Value::as_bool)
    }) == Some(true)
}

fn task_has_target_design(task: Option<&Value>) -> bool {
    let Some(task) = task else {
        return false;
    };
    if task
        .pointer("/task/task/targetDesignAttachmentId")
        .or_else(|| task.pointer("/task/task/target_design_attachment_id"))
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        return true;
    }
    if task
        .pointer("/task/task/attachmentIntent")
        .or_else(|| task.pointer("/task/task/attachment_intent"))
        .and_then(Value::as_str)
        == Some("TARGET_DESIGN")
    {
        return true;
    }
    task.pointer("/attachments/attachments")
        .and_then(Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.pointer("/metadata/intent").and_then(Value::as_str) == Some("TARGET_DESIGN")
            })
        })
}

fn request_requires_cross_platform_parity(request: &str) -> bool {
    let normalized = request.to_lowercase();
    [
        "apk/web",
        "android/web",
        "apk与web",
        "apk 与 web",
        "网页同步",
        "网页端同步",
        "web同步",
        "web 同步",
        "pwa同步",
        "pwa 同步",
        "cross-platform",
        "cross platform",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn request_requires_pwa_generation(request: &str) -> bool {
    let normalized = request.to_lowercase();
    normalized.contains("生成pwa")
        || normalized.contains("创建pwa")
        || normalized.contains("create pwa")
        || normalized.contains("new pwa")
        || normalized.contains("pwa_code_generation")
        || normalized.contains("pwa code generation")
        || normalized.contains("pwa源码")
        || normalized.contains("pwa 源码")
        || normalized.contains("写回pwa")
        || normalized.contains("写回 pwa")
        || normalized.contains("同步到 apk 与 pwa")
}

fn string_array(value: &Value, field: &str, min: usize, max: usize) -> Result<Vec<String>> {
    let items = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("缺少 {field}"))?;
    if items.len() < min || items.len() > max {
        bail!("{field} 数量必须在 {min}..{max}");
    }
    items
        .iter()
        .map(|item| {
            let text = item.as_str().map(str::trim).unwrap_or_default();
            if text.is_empty() || text.chars().count() > 80 {
                bail!("{field} 包含空值或超长能力名");
            }
            Ok(text.to_string())
        })
        .collect()
}

fn normalize_capabilities(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| item.trim().to_ascii_uppercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{derive_capabilities, requested_capabilities};
    use serde_json::json;

    #[test]
    fn caller_declared_capabilities_cannot_remove_derived_platform_requirements() {
        let task = json!({
            "task": {"task": {
                "mode": "MODIFY_EXISTING",
                "attachmentIntent": "AUTO",
                "targetDesignAttachmentId": "desktop_image_01",
                "request": "按设计图重做首页"
            }},
            "attachments": {"attachments": [{
                "metadata": {"attachmentId":"desktop_image_01", "intent":"TARGET_DESIGN"}
            }]}
        });
        let profile = json!({
            "android": {"applicationId":"com.elon.app"},
            "capabilities": {"apkWebUiParityRequired":true}
        });
        let requirements = requested_capabilities(
            &json!({"requiredCapabilities":["REAL_ANDROID_RENDERER"]}),
            Some(&task),
            Some(&profile),
        )
        .unwrap();

        assert_eq!(requirements.declared, vec!["REAL_ANDROID_RENDERER"]);
        assert!(requirements
            .effective
            .contains(&"PERSISTENT_FIT_RUN".to_string()));
        assert!(requirements
            .effective
            .contains(&"CROSS_PLATFORM_STYLE_WRITEBACK".to_string()));
    }

    #[test]
    fn attachment_level_target_design_drives_fit_run_even_when_overall_intent_is_auto() {
        let task = json!({
            "task": {"task": {"mode":"AUTO", "attachmentIntent":"AUTO"}},
            "attachments": {"attachments": [{
                "metadata": {"attachmentId":"desktop_image_01", "intent":"TARGET_DESIGN"}
            }]}
        });
        let (derived, _) = derive_capabilities(Some(&task), None);

        assert!(derived.contains(&"TARGET_DESIGN_BINDING".to_string()));
        assert!(derived.contains(&"PERSISTENT_FIT_RUN".to_string()));
    }

    #[test]
    fn explicit_new_pwa_request_derives_generation_and_cross_platform_writeback() {
        let task = json!({
            "task": {"task": {
                "mode":"CREATE_NEW",
                "attachmentIntent":"AUTO",
                "request":"创建PWA页面并与网页端同步"
            }}
        });
        let (derived, _) = derive_capabilities(Some(&task), None);

        assert!(derived.contains(&"PWA_CODE_GENERATION".to_string()));
        assert!(derived.contains(&"CROSS_PLATFORM_STYLE_WRITEBACK".to_string()));
    }

    #[test]
    fn existing_pwa_source_writeback_exposes_code_generation_capability() {
        let task = json!({
            "task": {"task": {
                "mode":"EXTEND_EXISTING",
                "request":"把真实 DOM 草稿写回 PWA 源码，并同步到 APK 与 PWA"
            }}
        });
        let (derived, reasons) = derive_capabilities(Some(&task), None);

        assert!(derived.contains(&"PWA_CODE_GENERATION".to_string()));
        assert!(reasons.iter().any(|reason| {
            reason.get("reason").and_then(|value| value.as_str())
                == Some("TASK_REQUESTS_PWA_SOURCE_WRITEBACK")
        }));
    }
}
