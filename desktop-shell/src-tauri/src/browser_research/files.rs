use super::model::{digest_id, hash, Session, SiteManifest, BODY_LIMIT};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;

pub fn ensure_directory(path: &Path) -> Result<(), String> {
    directory_chain(path, true)
}
fn reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}
fn directory_chain(path: &Path, create: bool) -> Result<(), String> {
    // Check caller-supplied segments before any OS normalization or directory creation.
    #[cfg(windows)]
    let parent_segment = {
        use std::os::windows::ffi::OsStrExt;
        let raw: Vec<u16> = path.as_os_str().encode_wide().collect();
        raw.split(|v| *v == u16::from(b'\\') || *v == u16::from(b'/'))
            .any(|part| part == [u16::from(b'.'), u16::from(b'.')])
    };
    #[cfg(not(windows))]
    let parent_segment = path.components().any(|c| matches!(c, Component::ParentDir));
    if !path.is_absolute() || parent_segment {
        return Err("invalid_storage_directory".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) => {
                current.push(component.as_os_str());
                continue;
            }
            Component::RootDir | Component::Normal(_) => current.push(component.as_os_str()),
            _ => return Err("invalid_storage_directory".into()),
        }
        let metadata = match fs::symlink_metadata(&current) {
            Ok(value) => value,
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                if let Err(error) = fs::create_dir(&current) {
                    if error.kind() != std::io::ErrorKind::AlreadyExists {
                        return Err("storage_unavailable".into());
                    }
                }
                fs::symlink_metadata(&current).map_err(|_| "storage_unavailable")?
            }
            Err(_) => return Err("storage_unavailable".into()),
        };
        if reparse(&metadata) || !metadata.is_dir() {
            return Err("invalid_storage_directory".into());
        }
    }
    Ok(())
}
fn bounded_read(reader: impl Read, max: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take((max as u64).checked_add(1).ok_or("invalid_stored_item")?)
        .read_to_end(&mut bytes)
        .map_err(|_| "stored_item_unavailable")?;
    if bytes.len() > max {
        return Err("invalid_stored_item".into());
    }
    Ok(bytes)
}
fn read(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    directory_chain(path.parent().ok_or("invalid_storage_directory")?, false)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| "stored_item_unavailable")?;
    if !metadata.is_file() || reparse(&metadata) || metadata.len() > max as u64 {
        return Err("invalid_stored_item".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let file = options.open(path).map_err(|_| "stored_item_unavailable")?;
    let opened = file.metadata().map_err(|_| "stored_item_unavailable")?;
    if !opened.is_file() || reparse(&opened) || opened.len() > max as u64 {
        return Err("invalid_stored_item".into());
    }
    // fstat is not a size reservation: a writer may append while this handle is being read.
    bounded_read(file, max)
}
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("invalid_storage_directory")?;
    ensure_directory(parent)?;
    if fs::symlink_metadata(path).is_ok_and(|m| reparse(&m) || !m.is_file()) {
        return Err("invalid_stored_item".into());
    }
    let temporary = path.with_extension("pending");
    if fs::symlink_metadata(&temporary).is_ok_and(|m| reparse(&m) || !m.is_file()) {
        return Err("invalid_stored_item".into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|_| "storage_unavailable")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "storage_unavailable")?;
    fs::rename(temporary, path).map_err(|_| "storage_unavailable".into())
}
pub fn save_session(root: &Path, session: &Session) -> Result<(), String> {
    if !digest_id(&session.id) {
        return Err("invalid_session".into());
    }
    let bytes = serde_json::to_vec(session).map_err(|_| "invalid_session")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("metadata_limit".into());
    }
    write(&root.join(&session.id).join("session.json"), &bytes)
}
pub fn load_sessions(root: &Path, project: &str, owner: &str) -> Vec<Session> {
    if !digest_id(project) || !digest_id(owner) || directory_chain(root, false).is_err() {
        return vec![];
    }
    let Ok(entries) = fs::read_dir(root) else {
        return vec![];
    };
    entries
        .take(128)
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            if !digest_id(&id) || reparse(&fs::symlink_metadata(entry.path()).ok()?) {
                return None;
            }
            let bytes = read(&entry.path().join("session.json"), 2 * 1024 * 1024).ok()?;
            let mut session: Session = serde_json::from_slice(&bytes).ok()?;
            if session.schema != "yilong.browser-research.session.v1"
                || session.id != id
                || session.project_key != project
                || session.owner_hash != owner
                || session.site.validate().is_err()
                || session.resources.len() > 512
                || session.requests.len() > 512
            {
                return None;
            }
            session.active = false;
            session.phase = if super::model::now_ms() >= session.expires_at_ms {
                "expired"
            } else {
                "restored"
            }
            .into();
            Some(session)
        })
        .collect()
}
pub fn save_body(root: &Path, session: &str, body: &str) -> Result<String, String> {
    if body.len() > BODY_LIMIT || !digest_id(session) {
        return Err("body_too_large".into());
    }
    let digest = hash(body.as_bytes());
    let path = root
        .join(session)
        .join("content")
        .join(format!("{digest}.txt"));
    if !path.exists() {
        write(&path, body.as_bytes())?;
    }
    Ok(digest)
}
pub fn read_body(root: &Path, session: &str, digest: &str) -> Result<String, String> {
    if !digest_id(session) || !digest_id(digest) {
        return Err("invalid_resource".into());
    }
    let bytes = read(
        &root
            .join(session)
            .join("content")
            .join(format!("{digest}.txt")),
        BODY_LIMIT,
    )?;
    if hash(&bytes) != digest {
        return Err("resource_integrity_changed".into());
    }
    String::from_utf8(bytes).map_err(|_| "resource_not_text".into())
}
pub fn manifest_path(root: &Path) -> PathBuf {
    root.join("sites.json")
}
pub fn manifests(root: &Path) -> Vec<SiteManifest> {
    let path = manifest_path(root);
    if !path.exists() {
        return super::model::defaults();
    }
    let Ok(bytes) = read(&path, 128 * 1024) else {
        return vec![];
    };
    let Ok(values) = serde_json::from_slice::<Vec<SiteManifest>>(&bytes) else {
        return vec![];
    };
    if values.len() > 32 || values.iter().any(|v| v.validate().is_err()) {
        return vec![];
    }
    values
}
pub fn register(root: &Path, manifest: SiteManifest) -> Result<Vec<SiteManifest>, String> {
    manifest.validate()?;
    let mut values = manifests(root);
    values.retain(|v| v.id != manifest.id);
    if values.len() >= 32 {
        return Err("site_limit".into());
    }
    values.push(manifest);
    write(
        &manifest_path(root),
        &serde_json::to_vec(&values).map_err(|_| "invalid_site_manifest")?,
    )?;
    Ok(values)
}
