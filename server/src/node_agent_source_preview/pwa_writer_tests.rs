use super::{
    parser::load_document,
    pwa_writer::{commit_pwa_style, PwaCommitErrorKind},
    routes::commit_pwa_style_handler,
    types::{
        CommitPreviewRequest, CommitPwaStyleRequest, PwaExplicitStyleBinding, PwaSourceRange,
        PwaStyleBindingKind,
    },
    writer::commit_changes,
};
use axum::{routing::post, Router};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("elon-pwa-writeback-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
        fs::write(&path, content).expect("write fixture");
        path
    }

    fn request(
        &self,
        relative: &str,
        content: &str,
        kind: PwaStyleBindingKind,
        target: &str,
        range: std::ops::Range<usize>,
        property_map: &[(&str, &str)],
        changes: &[(&str, &str)],
    ) -> CommitPwaStyleRequest {
        let revision = revision(content);
        CommitPwaStyleRequest {
            project_root: self.root.to_string_lossy().to_string(),
            binding: PwaExplicitStyleBinding {
                version: 1,
                source_file: relative.to_string(),
                source_revision: revision.clone(),
                kind,
                target: target.to_string(),
                range: PwaSourceRange {
                    start: range.start,
                    end: range.end,
                },
                property_map: property_map
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            },
            source_revision: revision,
            changes: changes
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn commits_css_rule_binding() {
    let fixture = Fixture::new();
    let source = ".card { height: 40px; border-radius: 6px; }\n";
    let path = fixture.write("src/styles/card.css", source);
    let request = fixture.request(
        "src/styles/card.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.trim_end().len(),
        &[("height", "height"), ("borderRadius", "border-radius")],
        &[("height", "48px"), ("border-radius", "12px")],
    );

    let response = commit_pwa_style(&request).expect("commit css rule");

    assert!(response.ok);
    assert_eq!(response.source_revision.len(), 64);
    assert_eq!(response.changed_files, ["src/styles/card.css"]);
    let updated = fs::read_to_string(path).expect("read css");
    assert!(updated.contains("height: 48px"));
    assert!(updated.contains("border-radius: 12px"));
}

#[test]
fn commits_style_object_binding_with_escaped_string_value() {
    let fixture = Fixture::new();
    let source = "export const cardStyle = {\n  color: '#111',\n  fontSize: '16px',\n};\n";
    let path = fixture.write("src/cardStyle.ts", source);
    let request = fixture.request(
        "src/cardStyle.ts",
        source,
        PwaStyleBindingKind::StyleObject,
        "cardStyle",
        0..source.trim_end().len(),
        &[("color", "color"), ("fontSize", "fontSize")],
        &[("color", "rgb(1, 2, 3)"), ("fontSize", "18\"px")],
    );

    commit_pwa_style(&request).expect("commit style object");

    let updated = fs::read_to_string(path).expect("read style object");
    assert!(updated.contains("color: \"rgb(1, 2, 3)\""));
    assert!(updated.contains("fontSize: \"18\\\"px\""));
}

#[test]
fn commits_token_json_binding_at_exact_json_pointer() {
    let fixture = Fixture::new();
    let object = r##"{"color":"#111","radius":"4px"}"##;
    let source = format!(r##"{{"tokens":{{"card":{object}}}}}"##) + "\n";
    let start = source.find(object).expect("token object start");
    let path = fixture.write("src/tokens.json", &source);
    let request = fixture.request(
        "src/tokens.json",
        &source,
        PwaStyleBindingKind::TokenJson,
        "/tokens/card",
        start..start + object.len(),
        &[("color", "color"), ("borderRadius", "radius")],
        &[("color", "#fff"), ("radius", "8px")],
    );

    commit_pwa_style(&request).expect("commit token json");

    let updated: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read token json"))
            .expect("valid updated json");
    assert_eq!(updated["tokens"]["card"]["color"], "#fff");
    assert_eq!(updated["tokens"]["card"]["radius"], "8px");
}

#[test]
fn rejects_traversal_and_absolute_paths() {
    let fixture = Fixture::new();
    let source = ".card { color: red; }";
    fixture.write("src/card.css", source);
    let mut traversal = fixture.request(
        "src/card.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.len(),
        &[("color", "color")],
        &[("color", "blue")],
    );
    traversal.binding.source_file = "../outside.css".into();
    assert_eq!(
        commit_pwa_style(&traversal)
            .expect_err("reject traversal")
            .kind(),
        PwaCommitErrorKind::Invalid
    );

    traversal.binding.source_file = if cfg!(windows) {
        "C:/outside.css".into()
    } else {
        "/tmp/outside.css".into()
    };
    assert_eq!(
        commit_pwa_style(&traversal)
            .expect_err("reject absolute path")
            .kind(),
        PwaCommitErrorKind::Invalid
    );
}

#[test]
fn rejects_symlink_escape() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let source = ".card { color: red; }";
    let outside_file = outside.write("outside.css", source);
    let link = fixture.root.join("src/linked.css");
    fs::create_dir_all(link.parent().expect("link parent")).expect("create link parent");
    if !create_file_symlink(&outside_file, &link) {
        return;
    }
    let request = fixture.request(
        "src/linked.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.len(),
        &[("color", "color")],
        &[("color", "blue")],
    );

    let error = commit_pwa_style(&request).expect_err("reject symlink escape");

    assert_eq!(error.kind(), PwaCommitErrorKind::Invalid);
    assert_eq!(fs::read_to_string(outside_file).unwrap(), source);
}

#[test]
fn rejects_top_level_binding_and_file_revision_conflicts() {
    let fixture = Fixture::new();
    let source = ".card { color: red; }";
    fixture.write("src/card.css", source);
    let mut request = fixture.request(
        "src/card.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.len(),
        &[("color", "color")],
        &[("color", "blue")],
    );
    request.source_revision = "f".repeat(64);
    assert_eq!(
        commit_pwa_style(&request)
            .expect_err("reject top-binding mismatch")
            .kind(),
        PwaCommitErrorKind::Conflict
    );

    request.binding.source_revision = request.source_revision.clone();
    assert_eq!(
        commit_pwa_style(&request)
            .expect_err("reject stale file revision")
            .kind(),
        PwaCommitErrorKind::Conflict
    );
}

#[test]
fn rejects_non_boundary_utf8_range_without_writing() {
    let fixture = Fixture::new();
    let source = "/*中*/.card { color: red; }";
    let path = fixture.write("src/card.css", source);
    let mut request = fixture.request(
        "src/card.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.len(),
        &[("color", "color")],
        &[("color", "blue")],
    );
    request.binding.range.start = 3;

    assert_eq!(
        commit_pwa_style(&request)
            .expect_err("reject non-boundary UTF-8 range")
            .kind(),
        PwaCommitErrorKind::Invalid
    );
    assert_eq!(fs::read_to_string(path).unwrap(), source);
}

#[test]
fn rejects_anchor_property_and_change_overflow_atomically() {
    let fixture = Fixture::new();
    let source = ".card { color: red; height: 40px; }";
    let path = fixture.write("src/card.css", source);
    let base = fixture.request(
        "src/card.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.len(),
        &[("color", "color"), ("height", "height")],
        &[("color", "blue")],
    );

    let mut anchor = base.clone();
    anchor.binding.target = ".other".into();
    assert_eq!(
        commit_pwa_style(&anchor).expect_err("reject anchor").kind(),
        PwaCommitErrorKind::Conflict
    );

    let mut illegal = base.clone();
    illegal.changes.insert("position".into(), "fixed".into());
    assert_eq!(
        commit_pwa_style(&illegal)
            .expect_err("reject unmapped property")
            .kind(),
        PwaCommitErrorKind::Invalid
    );

    let mut overflow = base;
    overflow.changes = (0..33)
        .map(|index| (format!("property{index}"), "x".into()))
        .collect();
    assert_eq!(
        commit_pwa_style(&overflow)
            .expect_err("reject change overflow")
            .kind(),
        PwaCommitErrorKind::Invalid
    );
    assert_eq!(fs::read_to_string(path).unwrap(), source);
}

#[test]
fn rejects_one_invalid_value_before_any_multi_change_write() {
    let fixture = Fixture::new();
    let source = ".card { color: red; height: 40px; }";
    let path = fixture.write("src/card.css", source);
    let request = fixture.request(
        "src/card.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.len(),
        &[("color", "color"), ("height", "height")],
        &[("color", "blue"), ("height", "40px; position:fixed")],
    );

    assert_eq!(
        commit_pwa_style(&request)
            .expect_err("reject unsafe second value")
            .kind(),
        PwaCommitErrorKind::Invalid
    );
    assert_eq!(fs::read_to_string(path).unwrap(), source);
}

#[test]
fn existing_android_xml_commit_still_writes_layout() {
    let fixture = Fixture::new();
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/node_agent_source_preview/testdata");
    copy_tree(&source, &fixture.root).expect("copy Android fixture");
    let document = load_document(fixture.root.to_str().unwrap(), None).expect("load Android XML");
    let action = &document.root.children[1];
    let request = CommitPreviewRequest {
        project_root: fixture.root.to_string_lossy().to_string(),
        layout_file: document.selected_layout,
        source_revision: document.source_revision,
        node_key: action.key.clone(),
        start_tag_start: action.source.start_tag_start,
        start_tag_end: action.source.start_tag_end,
        changes: BTreeMap::from([("text".into(), "Android 回归".into())]),
    };

    commit_changes(&request).expect("commit existing Android endpoint writer");

    let layout = fs::read_to_string(
        fixture
            .root
            .join("app/src/main/res/layout/activity_main.xml"),
    )
    .expect("read Android layout");
    assert!(layout.contains("android:text=\"Android 回归\""));
}

#[tokio::test]
async fn real_http_endpoint_commits_then_returns_409_for_old_revision() {
    let fixture = Fixture::new();
    let source = ".card { color: red; }";
    fixture.write("src/card.css", source);
    let request = fixture.request(
        "src/card.css",
        source,
        PwaStyleBindingKind::CssRule,
        ".card",
        0..source.len(),
        &[("color", "color")],
        &[("color", "blue")],
    );
    let body = serde_json::json!({
        "projectRoot": request.project_root,
        "binding": request.binding,
        "sourceRevision": request.source_revision,
        "changes": request.changes,
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind smoke server");
    let address = listener.local_addr().expect("smoke address");
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route(
                "/api/source-preview/commit-pwa-style",
                post(commit_pwa_style_handler),
            ),
        )
        .await
        .expect("serve smoke endpoint");
    });
    let url = format!("http://{address}/api/source-preview/commit-pwa-style");
    let client = reqwest::Client::new();

    let success = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .expect("HTTP commit");
    assert_eq!(success.status(), reqwest::StatusCode::OK);
    let success_body: serde_json::Value = success.json().await.expect("success JSON");
    assert_eq!(success_body["ok"], true);
    assert_eq!(
        success_body["changedFiles"],
        serde_json::json!(["src/card.css"])
    );
    assert_eq!(success_body["sourceRevision"].as_str().unwrap().len(), 64);

    let stale = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .expect("HTTP stale commit");
    assert_eq!(stale.status(), reqwest::StatusCode::CONFLICT);

    let mut malformed = body.clone();
    malformed["binding"]["unexpected"] = serde_json::json!(true);
    let rejected = client
        .post(&url)
        .json(&malformed)
        .send()
        .await
        .expect("HTTP malformed binding");
    assert_eq!(rejected.status(), reqwest::StatusCode::BAD_REQUEST);
    server.abort();
}

fn revision(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn create_file_symlink(source: &std::path::Path, target: &std::path::Path) -> bool {
    std::os::unix::fs::symlink(source, target).is_ok()
}

#[cfg(windows)]
fn create_file_symlink(source: &std::path::Path, target: &std::path::Path) -> bool {
    std::os::windows::fs::symlink_file(source, target).is_ok()
}
