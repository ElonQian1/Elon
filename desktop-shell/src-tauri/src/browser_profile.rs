//! Persistent browser profiles: directory-backed on Windows, UUID-backed on macOS 14+.
use std::path::Path;
use tauri::{webview::WebviewBuilder, Runtime};

pub(crate) fn persistent<R: Runtime>(
    builder: WebviewBuilder<R>,
    profile: &Path,
) -> WebviewBuilder<R> {
    #[cfg(target_os = "macos")]
    {
        builder.data_store_identifier(identifier(profile))
    }
    #[cfg(not(target_os = "macos"))]
    {
        builder.data_directory(profile.to_path_buf())
    }
}

#[cfg(any(target_os = "macos", test))]
fn identifier(profile: &Path) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"elon.wkwebview.profile.v1\0");
    hash.update(profile.to_string_lossy().as_bytes());
    let digest = hash.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    // Stable UUID with variant/version bits; never use the shared default data store.
    id[6] = (id[6] & 0x0f) | 0x50;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_are_stable_and_isolate_owners_providers_and_reading() {
        let profile = Path::new("profiles/owner-a/chatgpt");
        let id = identifier(profile);
        assert_eq!(id, identifier(profile));
        for other in [
            "profiles/owner-b/chatgpt",
            "profiles/owner-a/google",
            "profiles/owner-a/binance",
            "reading-tabs-profile",
        ] {
            assert_ne!(id, identifier(Path::new(other)));
        }
        assert_eq!(id[6] >> 4, 5);
        assert_eq!(id[8] >> 6, 2);
    }
}
