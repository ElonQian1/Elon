use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = fs::canonicalize(std::env::temp_dir()).unwrap();
        let path = base.join(format!(
            "elon-research-files-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        ensure_directory(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let (Ok(root), Ok(base)) = (
            fs::canonicalize(&self.0),
            fs::canonicalize(std::env::temp_dir()),
        ) {
            if root != base
                && root.starts_with(&base)
                && root
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("elon-research-files-test-"))
            {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}

#[test]
fn read_limit_holds_when_a_file_grows_after_its_initial_metadata_check() {
    let fixture = Fixture::new();
    let path = fixture.0.join("growing.txt");
    fs::write(&path, b"1234").unwrap();
    let opened = fs::File::open(&path).unwrap();
    assert_eq!(opened.metadata().unwrap().len(), 4);
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(&[b'x'; 64])
        .unwrap();
    assert_eq!(bounded_read(opened, 32).unwrap_err(), "invalid_stored_item");
    assert_eq!(read(&path, 32).unwrap_err(), "invalid_stored_item");
    assert_eq!(read(&path, 68).unwrap().len(), 68);
}

#[test]
fn stored_content_still_requires_regular_file_utf8_and_exact_digest() {
    let fixture = Fixture::new();
    let session = hash(b"session");
    let digest = save_body(&fixture.0, &session, "synthetic body").unwrap();
    assert_eq!(
        read_body(&fixture.0, &session, &digest).unwrap(),
        "synthetic body"
    );
    let path = fixture
        .0
        .join(&session)
        .join("content")
        .join(format!("{digest}.txt"));
    fs::write(&path, "tampered body").unwrap();
    assert_eq!(
        read_body(&fixture.0, &session, &digest).unwrap_err(),
        "resource_integrity_changed"
    );
    assert!(read(&fixture.0, 128).is_err());
    let invalid = hash(&[0xff]);
    fs::write(path.with_file_name(format!("{invalid}.txt")), [0xff]).unwrap();
    assert_eq!(
        read_body(&fixture.0, &session, &invalid).unwrap_err(),
        "resource_not_text"
    );
}

#[test]
fn directory_creation_checks_every_component_and_rejects_parent_traversal() {
    let fixture = Fixture::new();
    ensure_directory(&fixture.0.join("one").join("two")).unwrap();
    fs::write(fixture.0.join("regular"), b"synthetic").unwrap();
    assert!(ensure_directory(&fixture.0.join("regular").join("child")).is_err());
    // Joining onto a Windows verbatim path normalizes `..` before the function sees it.
    // Construct an actual unnormalized caller input to exercise the traversal guard.
    let mut raw = fixture.0.as_os_str().to_os_string();
    let separator = std::path::MAIN_SEPARATOR;
    raw.push(format!("{separator}one{separator}..{separator}escaped"));
    assert!(ensure_directory(&PathBuf::from(raw)).is_err());
}

#[cfg(unix)]
#[test]
fn a_symlink_parent_is_rejected_even_when_the_leaf_does_not_exist() {
    let fixture = Fixture::new();
    let target = fixture.0.join("target");
    ensure_directory(&target).unwrap();
    let link = fixture.0.join("linked");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(ensure_directory(&link.join("new-child")).is_err());
    assert!(!target.join("new-child").exists());
    fs::write(target.join("existing.txt"), b"synthetic").unwrap();
    assert!(read(&link.join("existing.txt"), 128).is_err());
    let file_link = fixture.0.join("file-link");
    std::os::unix::fs::symlink(target.join("existing.txt"), &file_link).unwrap();
    assert!(read(&file_link, 128).is_err());
}

#[test]
fn restoring_metadata_never_changes_owner_project_or_revives_expiry() {
    let fixture = Fixture::new();
    let site = super::super::model::defaults().remove(0);
    let project = hash(b"project");
    let owner = hash(b"owner");
    let session = Session {
        schema: "yilong.browser-research.session.v1".into(),
        id: hash(b"session"),
        project_key: project.clone(),
        owner_hash: owner.clone(),
        site_fingerprint: site.fingerprint(),
        site,
        active: true,
        generation: 7,
        expires_at_ms: 1,
        phase: "observing".into(),
        bytes: 0,
        resources: vec![],
        requests: vec![],
        gaps: vec![],
    };
    save_session(&fixture.0, &session).unwrap();
    assert!(load_sessions(&fixture.0, &hash(b"different-project"), &owner).is_empty());
    assert!(load_sessions(&fixture.0, &project, &hash(b"different-owner")).is_empty());
    let restored = load_sessions(&fixture.0, &project, &owner);
    assert_eq!(restored.len(), 1);
    assert!(!restored[0].active);
    assert_eq!(restored[0].phase, "expired");
}
