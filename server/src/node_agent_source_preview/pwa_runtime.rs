use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CapturePwaRuntimeRequest {
    pub(super) project_root: String,
    #[serde(flatten)]
    pub(super) capture: crate::node_agent_pwa_runtime::PwaCaptureInput,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn source_preview_contract_accepts_capture_and_rejects_secret_fields() {
        let request = json!({
            "projectRoot":"C:/fixture",
            "url":"http://127.0.0.1:4173/app",
            "viewport":{"width":360,"height":640,"deviceScaleFactor":1},
            "waitFor":{"condition":"load","timeoutMs":5000},
            "authProfile":"pc_ui_tuner_0123456789abcdef0123456789abcdef",
            "evidence":{"sourceRevision":"source-r1","routeRevision":"route-r1"}
        });
        let parsed = serde_json::from_value::<CapturePwaRuntimeRequest>(request.clone());
        assert!(
            parsed.is_ok(),
            "valid source-preview request must parse: {parsed:?}"
        );
        assert_eq!(
            parsed.unwrap().capture.auth_profile.as_deref(),
            Some("pc_ui_tuner_0123456789abcdef0123456789abcdef")
        );
        let mut unsafe_request = request;
        unsafe_request["authorization"] = json!("Bearer must-not-enter-api");
        assert!(serde_json::from_value::<CapturePwaRuntimeRequest>(unsafe_request).is_err());
    }
}
