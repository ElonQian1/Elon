//! Download tests compile the production handler and path/MIME code. Unrelated upload
//! account state is only a compile-time stand-in and must never be invoked here.
#![allow(dead_code)]
#[path = "../../../src/chat_attachments.rs"]
mod chat_attachments;
#[path = "../../../src/project_attachment_paths.rs"]
mod project_attachment_paths;

mod types {
    pub struct AppState {
        pub workspace_root: String,
        pub public_url: String,
        pub store: UploadStore,
    }
    pub struct UploadStore;
    pub struct User {
        pub id: String,
    }
    impl UploadStore {
        pub fn ensure_device_user(&self, _: &str) -> Result<User, String> {
            panic!("upload identity is outside the download harness")
        }
    }
}
mod project_ws_protocol {
    pub struct ProjectAttachmentRef {
        pub display_name: Option<String>,
        pub file_name: Option<String>,
    }
}
mod project_auth {
    pub fn json_error(
        status: axum::http::StatusCode,
        message: impl Into<String>,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;
        (
            status,
            axum::Json(serde_json::json!({ "error": message.into() })),
        )
            .into_response()
    }
}
