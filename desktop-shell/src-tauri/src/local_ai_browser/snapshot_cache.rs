use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CACHE_SCHEMA: &str = "elon.local_ai_web_snapshot.v1";
const DPAPI_ENTROPY: &[u8] = b"elon.local-ai-web-snapshot.v1";
const MAX_CACHE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub(super) struct LoadedSnapshot {
    pub semantic_event: Option<Value>,
    pub navigation_event: Option<Value>,
    pub updated_at_ms: u64,
}

#[derive(Deserialize, Serialize)]
struct SnapshotEnvelope {
    schema: String,
    provider_id: String,
    semantic_event: Option<Value>,
    navigation_event: Option<Value>,
    updated_at_ms: u64,
}

pub(super) fn load(path: &Path, provider_id: &str) -> Result<Option<LoadedSnapshot>> {
    if !cfg!(windows) || !path.is_file() {
        return Ok(None);
    }
    let protected = fs::read(path).context("read local AI snapshot cache")?;
    if protected.is_empty() || protected.len() > MAX_CACHE_BYTES * 2 {
        bail!("local AI snapshot cache size is invalid");
    }
    let plaintext = unprotect_for_current_user(&protected)?;
    if plaintext.is_empty() || plaintext.len() > MAX_CACHE_BYTES {
        bail!("local AI snapshot plaintext size is invalid");
    }
    let envelope: SnapshotEnvelope =
        serde_json::from_slice(&plaintext).context("decode local AI snapshot cache")?;
    if envelope.schema != CACHE_SCHEMA || envelope.provider_id != provider_id {
        bail!("local AI snapshot cache identity is invalid");
    }
    Ok(Some(LoadedSnapshot {
        semantic_event: envelope.semantic_event,
        navigation_event: envelope.navigation_event,
        updated_at_ms: envelope.updated_at_ms,
    }))
}

pub(super) fn store(
    path: &Path,
    provider_id: &str,
    semantic_event: Option<&Value>,
    navigation_event: Option<&Value>,
    updated_at_ms: u64,
) -> Result<bool> {
    if !cfg!(windows) {
        return Ok(false);
    }
    let Some(envelope) =
        cacheable_envelope(provider_id, semantic_event, navigation_event, updated_at_ms)
    else {
        return Ok(false);
    };
    let plaintext = serde_json::to_vec(&envelope).context("encode local AI snapshot cache")?;
    if plaintext.len() > MAX_CACHE_BYTES {
        bail!("local AI snapshot cache exceeds the local size limit");
    }
    let protected = protect_for_current_user(&plaintext)?;
    if protected.len() > MAX_CACHE_BYTES * 2 {
        bail!("protected local AI snapshot cache exceeds the local size limit");
    }
    let parent = path
        .parent()
        .context("local AI snapshot cache has no parent")?;
    fs::create_dir_all(parent).context("create local AI snapshot cache directory")?;
    atomic_replace(path, &protected)?;
    Ok(true)
}

pub(super) fn clear(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(temporary_path(path));
}

fn cacheable_envelope(
    provider_id: &str,
    semantic_event: Option<&Value>,
    navigation_event: Option<&Value>,
    updated_at_ms: u64,
) -> Option<SnapshotEnvelope> {
    if semantic_event.is_some_and(is_streaming_snapshot) {
        return None;
    }
    let mut semantic_event = semantic_event.cloned();
    if let Some(snapshot) = semantic_event.as_mut().and_then(Value::as_object_mut) {
        snapshot.insert("draft".to_string(), Value::String(String::new()));
        snapshot.insert("streaming".to_string(), Value::Bool(false));
    }
    let navigation_event = navigation_event.cloned();
    if semantic_event.is_none() && navigation_event.is_none() {
        return None;
    }
    Some(SnapshotEnvelope {
        schema: CACHE_SCHEMA.to_string(),
        provider_id: provider_id.to_string(),
        semantic_event,
        navigation_event,
        updated_at_ms,
    })
}

fn is_streaming_snapshot(value: &Value) -> bool {
    if value.get("streaming").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    value
        .get("messages")
        .and_then(Value::as_array)
        .is_some_and(|messages| {
            messages
                .iter()
                .any(|message| message.get("state").and_then(Value::as_str) == Some("streaming"))
        })
}

fn temporary_path(path: &Path) -> std::path::PathBuf {
    path.with_extension("dpapi.tmp")
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = temporary_path(path);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .context("open local AI snapshot cache temporary file")?;
    file.write_all(bytes)
        .context("write local AI snapshot cache temporary file")?;
    file.sync_all()
        .context("flush local AI snapshot cache temporary file")?;
    drop(file);
    replace_file(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let ok = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        bail!(
            "replace local AI snapshot cache: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<()> {
    fs::rename(source, target).context("replace local AI snapshot cache")
}

#[cfg(windows)]
fn protect_for_current_user(plaintext: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = blob(plaintext)?;
    let mut entropy = blob(DPAPI_ENTROPY)?;
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &mut input,
            std::ptr::null(),
            &mut entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        bail!(
            "protect local AI snapshot cache: {}",
            std::io::Error::last_os_error()
        );
    }
    take_local_blob(output)
}

#[cfg(windows)]
fn unprotect_for_current_user(protected: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = blob(protected)?;
    let mut entropy = blob(DPAPI_ENTROPY)?;
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let mut description = std::ptr::null_mut();
    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            &mut description,
            &mut entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if !description.is_null() {
        unsafe { LocalFree(description.cast()) };
    }
    if ok == 0 {
        bail!(
            "unprotect local AI snapshot cache: {}",
            std::io::Error::last_os_error()
        );
    }
    take_local_blob(output)
}

#[cfg(windows)]
fn blob(bytes: &[u8]) -> Result<windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB> {
    let len = u32::try_from(bytes.len()).context("local AI snapshot exceeds DPAPI input limit")?;
    Ok(
        windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB {
            cbData: len,
            pbData: bytes.as_ptr().cast_mut(),
        },
    )
}

#[cfg(windows)]
fn take_local_blob(
    output: windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB,
) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;

    if output.pbData.is_null() || output.cbData == 0 {
        if !output.pbData.is_null() {
            unsafe { LocalFree(output.pbData.cast()) };
        }
        bail!("Windows DPAPI returned an empty local AI snapshot");
    }
    let output_bytes =
        unsafe { std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize) };
    let bytes = output_bytes.to_vec();
    output_bytes.fill(0);
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(bytes)
}

#[cfg(not(windows))]
fn protect_for_current_user(_plaintext: &[u8]) -> Result<Vec<u8>> {
    bail!("local AI snapshot persistence is only available with Windows DPAPI")
}

#[cfg(not(windows))]
fn unprotect_for_current_user(_protected: &[u8]) -> Result<Vec<u8>> {
    bail!("local AI snapshot persistence is only available with Windows DPAPI")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cacheable_snapshot_strips_draft_and_rejects_streaming_content() {
        let complete = cacheable_envelope(
            "chatgpt",
            Some(&json!({
                "type": "message_snapshot",
                "draft": "private unfinished text",
                "streaming": false,
                "messages": [{"state": "completed", "content": [{"type": "text", "text": "answer"}]}]
            })),
            None,
            42,
        )
        .unwrap();
        assert_eq!(complete.semantic_event.unwrap()["draft"], "");

        let streaming = json!({
            "type": "message_snapshot",
            "draft": "",
            "streaming": true,
            "messages": [{"state": "streaming"}]
        });
        assert!(cacheable_envelope("chatgpt", Some(&streaming), None, 43).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_round_trip_is_bound_to_the_current_windows_user() {
        let protected = protect_for_current_user(b"local semantic snapshot").unwrap();
        assert_ne!(protected, b"local semantic snapshot");
        assert_eq!(
            unprotect_for_current_user(&protected).unwrap(),
            b"local semantic snapshot"
        );
    }

    #[cfg(windows)]
    #[test]
    fn encrypted_file_round_trip_and_corruption_are_fail_closed() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "elon-local-ai-cache-test-{}-{unique}",
            std::process::id()
        ));
        let path = directory.join("snapshot.dpapi");
        let semantic = json!({
            "type": "message_snapshot",
            "draft": "must not persist",
            "streaming": false,
            "messages": [{"state": "completed"}]
        });
        assert!(store(&path, "chatgpt", Some(&semantic), None, 99).unwrap());
        let loaded = load(&path, "chatgpt").unwrap().unwrap();
        assert_eq!(loaded.updated_at_ms, 99);
        assert_eq!(loaded.semantic_event.unwrap()["draft"], "");

        fs::write(&path, b"corrupt cache").unwrap();
        assert!(load(&path, "chatgpt").is_err());
        clear(&path);
        let _ = fs::remove_dir(&directory);
    }
}
