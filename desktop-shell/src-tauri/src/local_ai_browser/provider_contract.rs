use serde::Serialize;

// The PC UI can update independently from the installed Tauri executable.
// Increment this whenever the native host contract or bundled shared assets
// change in a way that a newer UI must not drive through an older executable.
pub(super) const DESKTOP_RUNTIME_VERSION: u32 = 6;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiWebProvider {
    pub(super) id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) start_host: &'static str,
    pub(super) login_mode: &'static str,
    pub(super) profile_scope: &'static str,
    pub(super) renderer_protocol: &'static str,
    pub(super) renderer_status: &'static str,
    pub(super) research_capture_status: &'static str,
    pub(super) research_capture_retention_days: u16,
    pub(super) desktop_runtime_version: u32,
    pub(super) adapter_version: u32,
    pub(super) adapter_actions: &'static [&'static str],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiWebSession {
    pub(super) provider_id: &'static str,
    pub(super) window_label: String,
    pub(super) status: &'static str,
    pub(super) profile_scope: &'static str,
    pub(super) cookie_access: &'static str,
    pub(super) renderer_protocol: &'static str,
    pub(super) renderer_status: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearLocalAiWebSession {
    pub(super) provider_id: &'static str,
    pub(super) status: &'static str,
}
