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
    let request = task
        .and_then(|value| value.pointer("/task/task/request"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if is_android_profile(profile) {
        if let Some(reason) = real_device_requirement_reason(request) {
            required.push("REAL_ANDROID_RENDERER");
            reasons.push(json!({
                "reason": reason,
                "capabilities": ["REAL_ANDROID_RENDERER"],
            }));
        }
    }
    if task_has_target_design(task) {
        required.extend([
            "TARGET_DESIGN_BINDING",
            "LOCAL_VISUAL_SOLVER",
            "PERSISTENT_FIT_RUN",
        ]);
        reasons.push(json!({
            "reason": "CLEAN_TARGET_DESIGN_PRESENT",
            "capabilities": [
                "TARGET_DESIGN_BINDING",
                "LOCAL_VISUAL_SOLVER",
                "PERSISTENT_FIT_RUN"
            ],
        }));
    }
    if request_requires_cross_platform_parity(request)
        || (profile_requires_apk_web_parity(profile) && !task_is_launcher_only(task))
    {
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

fn real_device_requirement_reason(request: &str) -> Option<&'static str> {
    let normalized = request.to_lowercase();
    let explicitly_requested = [
        "真机测试",
        "真机验证",
        "真机复现",
        "真机验收",
        "physical device",
        "real device",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    if explicitly_requested {
        return Some("USER_EXPLICITLY_REQUESTS_REAL_DEVICE");
    }

    let reports_incorrect_result = [
        "修改不对",
        "改得不对",
        "还是不对",
        "显示不对",
        "效果不对",
        "图标不对",
        "位置不对",
        "尺寸不对",
        "和设计不一致",
        "与设计不一致",
        "手机上有问题",
        "真机上有问题",
        "does not look right",
        "looks wrong",
        "incorrect on device",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    reports_incorrect_result.then_some("USER_REPORTS_VISUAL_RESULT_INCORRECT")
}

pub(super) fn task_is_launcher_only(task: Option<&Value>) -> bool {
    let request = task
        .and_then(|value| value.pointer("/task/task/request"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    if request_requires_cross_platform_parity(&request) {
        return false;
    }
    let launcher_marker = [
        "launcher",
        "app icon",
        "application icon",
        "adaptive icon",
        "启动器图标",
        "桌面图标",
        "应用图标",
        "自适应图标",
    ]
    .iter()
    .any(|marker| request.contains(marker));
    let app_logo = request.contains("logo") && request.contains("app");
    launcher_marker || app_logo
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
    use super::{derive_capabilities, requested_capabilities, task_is_launcher_only};
    use serde_json::{json, Value};

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

    #[test]
    fn launcher_only_task_does_not_inherit_project_web_parity() {
        let task = json!({"task":{"task":{
            "mode":"MODIFY_EXISTING",
            "request":"替换 APP Launcher 图标并验证 adaptive icon mask"
        }}});
        let profile = json!({
            "android":{"applicationId":"com.elon.app"},
            "capabilities":{"apkWebUiParityRequired":true}
        });
        let (derived, reasons) = derive_capabilities(Some(&task), Some(&profile));

        assert!(task_is_launcher_only(Some(&task)));
        assert!(!derived.contains(&"CROSS_PLATFORM_STYLE_WRITEBACK".to_string()));
        assert!(!reasons.iter().any(|reason| {
            reason.get("reason").and_then(Value::as_str)
                == Some("PROJECT_REQUIRES_APK_WEB_UI_PARITY")
        }));
    }

    #[test]
    fn explicit_web_request_still_requires_cross_platform_parity_for_launcher() {
        let task = json!({"task":{"task":{
            "request":"同步 APP Launcher 图标到 APK 与 Web"
        }}});
        assert!(!task_is_launcher_only(Some(&task)));
    }

    #[test]
    fn ordinary_android_and_launcher_tasks_do_not_require_a_physical_device() {
        let profile = json!({"android":{"applicationId":"com.elon.app"}});
        for request in ["按设计图重做首页", "替换 APP Launcher 图标"] {
            let task = json!({"task":{"task":{"request":request}}});
            let (derived, _) = derive_capabilities(Some(&task), Some(&profile));
            assert!(!derived.contains(&"REAL_ANDROID_RENDERER".to_string()));
        }
    }

    #[test]
    fn user_feedback_or_explicit_request_enables_real_device_verification() {
        let profile = json!({"android":{"applicationId":"com.elon.app"}});
        for (request, expected_reason) in [
            (
                "刚刚修改不对，手机上图标显示不对",
                "USER_REPORTS_VISUAL_RESULT_INCORRECT",
            ),
            ("请执行真机验证", "USER_EXPLICITLY_REQUESTS_REAL_DEVICE"),
        ] {
            let task = json!({"task":{"task":{"request":request}}});
            let (derived, reasons) = derive_capabilities(Some(&task), Some(&profile));
            assert!(derived.contains(&"REAL_ANDROID_RENDERER".to_string()));
            assert!(reasons.iter().any(|reason| {
                reason.get("reason").and_then(Value::as_str) == Some(expected_reason)
            }));
        }
    }
}
