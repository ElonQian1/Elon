// Test infrastructure only: production SQL and all balance decisions are #[path] included.
pub(crate) fn hash_token(token: &str) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(token.as_bytes()))
}

pub(super) fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4().simple())
}

pub(super) fn now() -> String {
    "2026-09-04T10:00:00Z".to_owned()
}

#[path = "../../../src/store/common/esk_platform_assets/mod.rs"]
mod esk_platform_assets;
