use super::*;

#[test]
fn apply_patch_changes_file() {
    let workspace = temp_workspace("apply_patch_changes_file");
    init_git(&workspace);
    std::fs::create_dir_all(workspace.join("src")).unwrap();
    std::fs::write(
        workspace.join("src/lib.rs"),
        "pub fn answer() -> i32 {\n    1\n}\n",
    )
    .unwrap();

    let patch = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 pub fn answer() -> i32 {
-    1
+    2
 }
"#;

    let result = apply_patch(&workspace, patch, false).unwrap();
    assert!(result.contains("src/lib.rs"));
    let content = std::fs::read_to_string(workspace.join("src/lib.rs")).unwrap();
    assert!(content.contains("    2"));
}

#[test]
fn check_only_does_not_change_file() {
    let workspace = temp_workspace("check_only_does_not_change_file");
    init_git(&workspace);
    std::fs::write(workspace.join("README.md"), "old\n").unwrap();

    let patch = r#"diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-old
+new
"#;

    let result = apply_patch(&workspace, patch, true).unwrap();
    assert!(result.contains("补丁检查通过"));
    assert_eq!(
        std::fs::read_to_string(workspace.join("README.md")).unwrap(),
        "old\n"
    );
}

#[test]
fn apply_patch_diff_preview_reports_checked_patch_and_base_hash() {
    let workspace = temp_workspace("apply_patch_diff_preview_reports_checked_patch");
    init_git(&workspace);
    std::fs::write(workspace.join("README.md"), "old\n").unwrap();

    let patch = r#"diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-old
+new
"#;

    let preview = apply_patch_diff_preview(&workspace, patch).unwrap();

    assert_eq!(preview["source"], "apply_patch");
    assert_eq!(preview["kind"], "patch");
    assert_eq!(preview["files"][0], "README.md");
    assert_eq!(preview["base_files"][0]["path"], "README.md");
    assert_eq!(preview["base_files"][0]["exists"], true);
    assert!(preview["base_files"][0]["sha256"].as_str().unwrap().len() >= 64);
    assert!(preview["patch_sha256"].as_str().unwrap().len() >= 64);
    assert!(preview["preview"].as_str().unwrap().contains("-old"));
    assert!(preview["preview"].as_str().unwrap().contains("+new"));
}

#[test]
fn verify_apply_patch_preview_detects_changed_base_file() {
    let workspace = temp_workspace("verify_apply_patch_preview_detects_changed_base_file");
    init_git(&workspace);
    std::fs::write(workspace.join("README.md"), "old\n").unwrap();

    let patch = r#"diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-old
+new
"#;

    let preview = apply_patch_diff_preview(&workspace, patch).unwrap();
    verify_apply_patch_preview_unchanged(&workspace, patch, &preview).unwrap();

    std::fs::write(workspace.join("README.md"), "changed elsewhere\n").unwrap();
    let error = verify_apply_patch_preview_unchanged(&workspace, patch, &preview).unwrap_err();

    assert!(error.to_string().contains("base changed"));
}

#[test]
fn rejects_path_escape() {
    let workspace = temp_workspace("rejects_path_escape");
    init_git(&workspace);

    let patch = r#"diff --git a/../outside.txt b/../outside.txt
--- a/../outside.txt
+++ b/../outside.txt
@@ -1 +1 @@
-old
+new
"#;

    let error = apply_patch(&workspace, patch, true).unwrap_err();
    assert!(error.to_string().contains("不安全"));
}

#[test]
fn rejects_git_directory_case_insensitively() {
    let workspace = temp_workspace("rejects_git_directory_case_insensitively");
    init_git(&workspace);

    let patch = r#"diff --git a/.GIT/config b/.GIT/config
--- a/.GIT/config
+++ b/.GIT/config
@@ -1 +1 @@
-old
+new
"#;

    let error = apply_patch(&workspace, patch, true).unwrap_err();
    assert!(error.to_string().contains(".git"));
}

fn temp_workspace(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("elon-tools-patch-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn init_git(workspace: &Path) {
    let output = crate::git_command_error::git_command()
        .args(["init"])
        .current_dir(workspace)
        .output()
        .unwrap();
    assert!(output.status.success());
}
