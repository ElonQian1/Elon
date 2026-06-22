use crate::project_agent_runtime::ensure_project_agent_runtime_files;
use crate::project_commands::ensure_project_command_files;
use crate::project_environment::ensure_project_environment_files;
use crate::project_workflow::ensure_project_workflow_files;
use serde_json::json;
use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

pub struct ProjectScaffoldRequest<'a> {
    pub project_id: &'a str,
    pub user_id: &'a str,
    pub name: &'a str,
    pub template: &'a str,
    pub repo_url: Option<&'a str>,
    pub branch: Option<&'a str>,
}

pub fn ensure_project_scaffold(repo: &Path, req: &ProjectScaffoldRequest<'_>) -> io::Result<()> {
    fs::create_dir_all(repo)?;
    ensure_file(repo.join("README.md"), || readme(req))?;
    ensure_file(repo.join("AGENTS.md"), || agents(req))?;
    ensure_file(repo.join(".gitignore"), gitignore)?;
    ensure_file(repo.join(".gitattributes"), gitattributes)?;
    ensure_file(repo.join(".env.example"), || env_example(req))?;
    ensure_file(repo.join(".elon").join("project.json"), || {
        project_json(req)
    })?;
    ensure_project_command_files(repo, req)?;
    ensure_project_environment_files(repo, req)?;
    ensure_project_agent_runtime_files(repo, req)?;
    ensure_project_workflow_files(repo, req)?;
    if req.template.eq_ignore_ascii_case("android") {
        ensure_file(
            repo.join("local.properties.example"),
            android_local_properties,
        )?;
    }
    Ok(())
}

fn ensure_file(
    path: impl AsRef<Path>,
    content: impl FnOnce() -> io::Result<String>,
) -> io::Result<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content()?)
}

fn readme(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    Ok(format!(
        "# {}\n\nThis workspace was created by Elon PC Dev Runtime.\n\n- project_id: {}\n- template: {}\n- owner_user_id: {}\n\nThe cloud server stores project metadata and chat history. Source code, build caches, and task worktrees live on this PC node.\n",
        req.name.trim(),
        req.project_id,
        req.template,
        req.user_id
    ))
}

fn agents(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    Ok(format!(
        "# Project Workspace\n\nThis project is managed by an Elon PC node.\n\nRules:\n- Keep source code and build outputs inside this repository.\n- Use git for every meaningful code change.\n- Prefer task-specific worktrees for parallel conversations.\n- Do not write build artifacts to the cloud server workspace.\n\nProject metadata:\n- project_id: {}\n- template: {}\n",
        req.project_id, req.template
    ))
}

fn gitignore() -> io::Result<String> {
    Ok([
        ".gradle/",
        "build/",
        "app/build/",
        "target/",
        "node_modules/",
        "dist/",
        "*.apk",
        "*.aab",
        "local.properties",
        ".env",
        ".env.*",
        "!.env.example",
        "",
    ]
    .join("\n"))
}

fn gitattributes() -> io::Result<String> {
    Ok([
        "* text=auto",
        "*.sh text eol=lf",
        "*.ps1 text eol=crlf",
        "*.cmd text eol=crlf",
        "*.bat text eol=crlf",
        "",
    ]
    .join("\n"))
}

fn env_example(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    Ok(format!(
        "ELON_PROJECT_ID=\"{}\"\nELON_PROJECT_NAME=\"{}\"\nELON_PROJECT_TEMPLATE=\"{}\"\n# Route C server-runtime, used when this PC has no AI CLI and no API key.\n# Windows client login is reused automatically; set these only for portable or custom-server setups.\n# ELON_SERVER_URL=\"http://43.139.149.158:8080\"\n# ELON_SERVER_TOKEN=\"<login-token>\"\n# ELON_SERVER_AGENT=\"\"  # Optional; server must allow it via ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS.\n# ANDROID_HOME=\"C:\\\\Users\\\\<you>\\\\AppData\\\\Local\\\\Android\\\\Sdk\"\n# RUST_LOG=\"info\"\n",
        env_escape(req.project_id),
        env_escape(req.name.trim()),
        env_escape(req.template)
    ))
}

fn android_local_properties() -> io::Result<String> {
    Ok("# Copy this file to local.properties and set your Android SDK path.\n# sdk.dir=C:\\\\Users\\\\<you>\\\\AppData\\\\Local\\\\Android\\\\Sdk\n".to_string())
}

fn project_json(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    serde_json::to_string_pretty(&json!({
        "schema_version": 1,
        "managed_by": "elon_pc_dev_runtime",
        "project_id": req.project_id,
        "owner_user_id": req.user_id,
        "name": req.name.trim(),
        "template": req.template,
        "repo_url": req.repo_url,
        "branch": req.branch,
    }))
    .map(|value| format!("{value}\n"))
    .map_err(|error| io::Error::new(ErrorKind::Other, error))
}

fn env_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::{ensure_project_scaffold, ProjectScaffoldRequest};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn scaffold_creates_runtime_project_files() {
        let root = temp_dir("scaffold_creates_runtime_project_files");
        let req = request();
        ensure_project_scaffold(&root, &req).unwrap();

        assert!(root.join("README.md").exists());
        assert!(root.join("AGENTS.md").exists());
        assert!(root.join(".gitignore").exists());
        assert!(root.join(".gitattributes").exists());
        assert!(root.join(".env.example").exists());
        assert!(root.join("docs").join("dev-environment.md").exists());
        assert!(root.join("docs").join("agent-runtime.md").exists());
        assert!(root.join("scripts").join("elon.ps1").exists());
        assert!(root.join("scripts").join("elon-agent.ps1").exists());
        assert!(root.join("scripts").join("elon-dev-check.ps1").exists());
        assert!(root.join("scripts").join("elon-new-task.ps1").exists());
        assert!(root.join(".elon").join("project.json").exists());
        assert!(root.join("local.properties.example").exists());
        assert!(fs::read_to_string(root.join(".elon").join("project.json"))
            .unwrap()
            .contains("\"managed_by\": \"elon_pc_dev_runtime\""));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scaffold_does_not_overwrite_user_files() {
        let root = temp_dir("scaffold_does_not_overwrite_user_files");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("README.md"), "custom").unwrap();

        ensure_project_scaffold(&root, &request()).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("README.md")).unwrap(),
            "custom"
        );
        let _ = fs::remove_dir_all(root);
    }

    fn request() -> ProjectScaffoldRequest<'static> {
        ProjectScaffoldRequest {
            project_id: "project-1",
            user_id: "user-1",
            name: "Demo App",
            template: "android",
            repo_url: Some("https://example.com/repo.git"),
            branch: Some("main"),
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-pc-dev-runtime-{label}-{nanos}"))
    }
}
