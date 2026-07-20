//! Project-scoped PWA runtime capture for the local Windows node.
//!
//! The renderer is intentionally independent from the Android live renderer. Both the
//! vendor-neutral UI MCP and the source-preview HTTP workflow call this module so there is only
//! one browser, URL, authentication, artifact, and redaction policy.

mod artifact;
mod auth;
mod browser;
mod cdp;
mod process;
mod security;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(crate) const TOOL_NAME: &str = "ui_capture_pwa_runtime";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PwaCaptureInput {
    pub(crate) url: String,
    pub(crate) viewport: CaptureViewport,
    #[serde(default)]
    pub(crate) wait_for: CaptureWait,
    #[serde(default)]
    pub(crate) capture: CaptureScope,
    #[serde(default)]
    pub(crate) auth_profile: Option<String>,
    pub(crate) evidence: CaptureEvidenceInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureViewport {
    pub(crate) width: u32,
    pub(crate) height: u32,
    #[serde(default = "default_device_scale_factor")]
    pub(crate) device_scale_factor: f64,
}

fn default_device_scale_factor() -> f64 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureWait {
    #[serde(default = "default_wait_condition")]
    pub(crate) condition: String,
    #[serde(default = "default_timeout_ms")]
    pub(crate) timeout_ms: u64,
    #[serde(default = "default_settle_ms")]
    pub(crate) settle_ms: u64,
    #[serde(default)]
    pub(crate) selector: Option<String>,
}

impl Default for CaptureWait {
    fn default() -> Self {
        Self {
            condition: default_wait_condition(),
            timeout_ms: default_timeout_ms(),
            settle_ms: default_settle_ms(),
            selector: None,
        }
    }
}

fn default_wait_condition() -> String {
    "networkidle".to_string()
}

fn default_timeout_ms() -> u64 {
    30_000
}

fn default_settle_ms() -> u64 {
    500
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureScope {
    #[serde(default)]
    pub(crate) full_page: bool,
    #[serde(default)]
    pub(crate) selector: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureEvidenceInput {
    #[serde(default)]
    pub(crate) source_revision: Option<String>,
    #[serde(default)]
    pub(crate) source_revisions: BTreeMap<String, String>,
    pub(crate) route_revision: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureDiagnostic {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) retryable: bool,
    pub(crate) next_step: String,
}

impl CaptureDiagnostic {
    pub(crate) fn new(
        code: &'static str,
        message: impl Into<String>,
        retryable: bool,
        next_step: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
            next_step: next_step.into(),
        }
    }

    fn response(self) -> Value {
        json!({
            "ok": false,
            "status": "CAPTURE_FAILED",
            "diagnostic": self,
            "base64Embedded": false,
        })
    }
}

pub(crate) fn tool_definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "在 PC 节点本机用受控无头 Edge/Chrome 渲染项目 PWA，并保存可复核 PNG。只允许 loopback 或项目白名单来源；认证只引用本机准备的 profile 文件，禁止在参数中传 token/Cookie/Authorization。",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["url", "viewport", "evidence"],
            "properties": {
                "url": {"type":"string","minLength":1,"maxLength":4096,"description":"绝对 http(s) URL；默认只允许 localhost/loopback，秘密不得放在 query 中"},
                "viewport": {
                    "type":"object","additionalProperties":false,"required":["width","height"],
                    "properties": {
                        "width":{"type":"integer","minimum":240,"maximum":4096},
                        "height":{"type":"integer","minimum":240,"maximum":4096},
                        "deviceScaleFactor":{"type":"number","minimum":0.5,"maximum":4,"default":1}
                    }
                },
                "waitFor": {
                    "type":"object","additionalProperties":false,
                    "properties": {
                        "condition":{"enum":["domcontentloaded","load","networkidle"],"default":"networkidle"},
                        "timeoutMs":{"type":"integer","minimum":500,"maximum":120000,"default":30000},
                        "settleMs":{"type":"integer","minimum":0,"maximum":5000,"default":500},
                        "selector":{"type":"string","minLength":1,"maxLength":1000}
                    }
                },
                "capture": {
                    "type":"object","additionalProperties":false,
                    "properties": {
                        "fullPage":{"type":"boolean","default":false},
                        "selector":{"type":"string","minLength":1,"maxLength":1000}
                    }
                },
                "authProfile":{"type":"string","pattern":"^[A-Za-z0-9_-]{1,64}$","description":"只传 profile 名；秘密保存在项目 .elon/ui-tuner/pwa-sessions/<profile>.json"},
                "evidence": {
                    "type":"object","additionalProperties":false,"required":["routeRevision"],
                    "properties": {
                        "sourceRevision":{"type":"string","minLength":1,"maxLength":160},
                        "sourceRevisions":{"type":"object","maxProperties":64,"additionalProperties":{"type":"string","pattern":"^[a-fA-F0-9]{64}$"}},
                        "routeRevision":{"type":"string","minLength":1,"maxLength":160}
                    },
                    "anyOf":[{"required":["sourceRevision"]},{"required":["sourceRevisions"]}]
                }
            }
        },
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": false,
            "idempotentHint": false,
            "openWorldHint": false
        }
    })
}

pub(crate) async fn capture_tool(project_root: Option<&str>, arguments: Value) -> Value {
    let Some(project_root) = project_root.filter(|value| !value.trim().is_empty()) else {
        return CaptureDiagnostic::new(
            "PROJECT_ROOT_REQUIRED",
            "PWA Runtime 捕获需要绑定本机项目目录",
            false,
            "重新为当前 EDIT_ROOT 创建 yilong_ui_live 项目会话",
        )
        .response();
    };
    let input = match serde_json::from_value::<PwaCaptureInput>(arguments) {
        Ok(input) => input,
        Err(_) => {
            return CaptureDiagnostic::new(
                "INVALID_ARGUMENTS",
                "PWA Runtime 捕获参数不符合工具 schema",
                false,
                "按 tools/list 返回的 schema 修正 URL、viewport、waitFor、capture 和 evidence",
            )
            .response()
        }
    };
    capture(project_root, input).await
}

pub(crate) async fn capture(project_root: &str, input: PwaCaptureInput) -> Value {
    let prepared = match security::prepare(project_root, input) {
        Ok(prepared) => prepared,
        Err(diagnostic) => return diagnostic.response(),
    };
    let rendered = match browser::render(&prepared).await {
        Ok(rendered) => rendered,
        Err(diagnostic) => return diagnostic.response(),
    };
    match artifact::persist(&prepared, rendered) {
        Ok(result) => {
            let context_path = result.artifact.path.clone();
            let context_sha256 = result.artifact.sha256.clone();
            json!({
                "ok": true,
                "status": "CAPTURED",
                "artifact": result.artifact,
                "route": result.route,
                "revision": prepared.evidence,
                "browser": result.browser,
                "viewport": result.viewport,
                "networkPolicy": result.network_policy,
                "authentication": {"mode": prepared.auth.mode, "profile": prepared.auth.profile},
                "processCleanup": result.process_cleanup,
                "contextPackReference": {
                    "path": context_path,
                    "sha256": context_sha256,
                    "embedBase64": false
                },
                "base64Embedded": false,
            })
        }
        Err(diagnostic) => diagnostic.response(),
    }
}
