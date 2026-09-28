//! Explicit, capture-only acceptance of a declared public login surface.
use super::{security, CaptureDiagnostic, CaptureInteractionStep, PwaCaptureInput};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExpectedPage {
    pub(crate) kind: ExpectedPageKind,
    pub(crate) page_id: String,
    pub(crate) path: String,
    pub(crate) ready_selector: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum ExpectedPageKind {
    #[serde(rename = "PUBLIC_LOGIN")]
    PublicLogin,
}

pub(super) fn schema() -> Value {
    json!({
        "type":"object","additionalProperties":false,
        "description":"显式验收公开登录页；仅捕获，不代表认证成功。必须无显式或默认 authProfile；只允许等待、文字断言、滚动步骤。",
        "required":["kind","pageId","path","readySelector"],
        "properties":{
            "kind":{"const":"PUBLIC_LOGIN"},
            "pageId":{"type":"string","pattern":"^[A-Za-z0-9_-]{1,64}$"},
            "path":{"type":"string","minLength":1,"maxLength":1024},
            "readySelector":{"type":"string","minLength":1,"maxLength":1000}
        }
    })
}

pub(super) fn validate(
    input: &PwaCaptureInput,
    default_auth_profile: Option<&str>,
    url: &Url,
) -> Result<(), CaptureDiagnostic> {
    let Some(expected) = &input.expected_page else {
        return Ok(());
    };
    if input.auth_profile.is_some() || default_auth_profile.is_some() {
        return Err(security::invalid(
            "PUBLIC_LOGIN_AUTH_PROFILE_CONFLICT",
            "公开登录页验收不能携带显式或项目默认 authProfile；不会静默忽略认证配置",
        ));
    }
    if !security::valid_profile(&expected.page_id)
        || expected.path.len() > 1024
        || expected.path != url.path()
        || expected.path.contains(['?', '#', '\\', '\0', '\r', '\n'])
    {
        return Err(security::invalid(
            "PUBLIC_LOGIN_DECLARATION_INVALID",
            "expectedPage 需要安全 pageId 和与请求 URL 一致、不含 query/fragment 的精确路径",
        ));
    }
    security::validate_selector(Some(&expected.ready_selector))?;
    if input.steps.iter().any(|step| {
        !matches!(
            step,
            CaptureInteractionStep::WaitFor { .. }
                | CaptureInteractionStep::AssertText { .. }
                | CaptureInteractionStep::ScrollIntoView { .. }
        )
    }) {
        return Err(security::invalid(
            "PUBLIC_LOGIN_CAPTURE_ONLY",
            "公开登录页验收只允许等待、文字断言和滚动；禁止提交表单、输入或预览样式修改",
        ));
    }
    Ok(())
}

pub(super) fn ready(
    prepared: &security::PreparedCapture,
    href: &str,
    page: &Value,
) -> Result<bool, CaptureDiagnostic> {
    let Some(expected) = &prepared.expected_page else {
        return Ok(true);
    };
    let actual = Url::parse(href).map_err(|_| mismatch())?;
    if actual.origin() != prepared.url.origin() || actual.path() != expected.path {
        return Err(mismatch());
    }
    Ok(page["publicReady"] == true)
}

fn mismatch() -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "PUBLIC_LOGIN_PAGE_MISMATCH",
        "实际页面与声明的公开登录页 origin/path 不一致，未保存截图",
        false,
        "核对 expectedPage、真实路由和页面状态；不要以其他页面代替登录页验收",
    )
}
