use anyhow::{bail, Context, Result};
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(super) struct DurableNodeStatePaths {
    pub(super) node_config: PathBuf,
    pub(super) full_access_grants: PathBuf,
    pub(super) local_tasks: PathBuf,
    pub(super) task_journal: PathBuf,
}

impl DurableNodeStatePaths {
    fn from_state_file(state_file: &Path) -> Result<Self> {
        let state_dir = state_file
            .parent()
            .context("node state file has no parent directory")?;
        Ok(Self {
            node_config: state_file.to_path_buf(),
            full_access_grants: state_dir.join("full-access-grants.json"),
            local_tasks: state_dir.join("local-tasks.sqlite3"),
            task_journal: state_dir.join("task-journal"),
        })
    }

    fn all(&self) -> [&Path; 4] {
        [
            &self.node_config,
            &self.full_access_grants,
            &self.local_tasks,
            &self.task_journal,
        ]
    }
}

/// Package and single-exe updates may replace the install tree, but identity,
/// credentials, grants, supervised task bindings, and journal data are durable
/// APPDATA state. Fail the update closed if an unusual environment would place
/// any of that state inside the replaceable install tree.
pub(super) fn ensure_durable_state_outside_install(
    install_dir: &Path,
    state_file: &Path,
) -> Result<DurableNodeStatePaths> {
    let install_dir = comparable_absolute_path(install_dir)?;
    let paths = DurableNodeStatePaths::from_state_file(state_file)?;
    for durable_path in paths.all() {
        let durable_path = comparable_absolute_path(durable_path)?;
        if path_is_within(&durable_path, &install_dir) {
            bail!(
                "refusing node client replacement because durable state {} is inside install tree {}",
                durable_path.display(),
                install_dir.display()
            );
        }
    }
    Ok(paths)
}

fn comparable_absolute_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("resolve current directory for update compatibility check")?
            .join(path)
    };
    Ok(std::fs::canonicalize(&absolute).unwrap_or(absolute))
}

fn path_is_within(candidate: &Path, root: &Path) -> bool {
    let candidate = normalized_components(candidate);
    let root = normalized_components(root);
    candidate.len() >= root.len() && candidate[..root.len()] == root
}

fn normalized_components(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| match component {
            Component::Prefix(prefix) => normalize_component(&prefix.as_os_str().to_string_lossy()),
            Component::RootDir => String::from("/"),
            Component::CurDir => String::from("."),
            Component::ParentDir => String::from(".."),
            Component::Normal(value) => normalize_component(&value.to_string_lossy()),
        })
        .collect()
}

fn normalize_component(value: &str) -> String {
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        node_agent_local_task_store::{LocalTaskStart, LocalTaskStore},
        node_agent_task_journal::{TaskJournal, TaskJournalStart},
    };
    use std::fs;

    #[test]
    fn rejects_durable_state_nested_in_replaced_install_tree() {
        let install = Path::new(r"C:\Program Files\YilongNode");
        let unsafe_state = install.join("_internal").join("node.json");

        let error = ensure_durable_state_outside_install(install, &unsafe_state)
            .expect_err("state nested in install must block replacement");

        assert!(error.to_string().contains("durable state"));
    }

    #[test]
    fn replacement_preserves_grants_identity_credentials_journal_and_project_binding() {
        let root =
            std::env::temp_dir().join(format!("elon-node-update-compat-{}", uuid::Uuid::new_v4()));
        let install = root.join("installed-client");
        let internal = install.join("_internal");
        let state_dir = root.join("appdata").join("elon-node-agent");
        fs::create_dir_all(&internal).expect("create install fixture");
        fs::create_dir_all(&state_dir).expect("create durable state fixture");

        let state_file = state_dir.join("node.json");
        let node_config = br#"{
          "install_id":"ins_preserved",
          "agent_id":"agent_preserved",
          "agent_secret":"secret_preserved",
          "owner_user_id":"owner_preserved",
          "user_token":"token_preserved"
        }"#;
        fs::write(&state_file, node_config).expect("write node identity fixture");

        let grants_file = state_dir.join("full-access-grants.json");
        let grants = r#"{"grants":[{
          "owner_user_id":"owner_preserved",
          "agent_id":"agent_preserved",
          "install_id":"ins_preserved",
          "project_id":"project_chinese",
          "workspace_path":"D:\\项目\\中文工作区",
          "granted_at_ms":1
        }]}"#
            .as_bytes();
        fs::write(&grants_file, grants).expect("write full access fixture");

        let journal = TaskJournal::new(state_dir.join("task-journal"));
        journal
            .record_started(TaskJournalStart {
                req_id: "local-preserved",
                cli_name: "codex",
                route: Some("A"),
                run_handle_id: Some("run-preserved"),
                cwd: Some(r"D:\项目\中文工作区"),
                runtime_permission: Some("full_access"),
            })
            .expect("write task journal fixture");

        let local_tasks = LocalTaskStore::new(state_dir.join("local-tasks.sqlite3"));
        local_tasks
            .create(LocalTaskStart {
                task_id: "local-preserved",
                owner_user_id: "owner_preserved",
                agent_id: "agent_preserved",
                install_id: "ins_preserved",
                project_id: "project_chinese",
                channel_id: None,
                conversation_id: "desktop-supervised-preserved",
                workspace_path: r"D:\项目\中文工作区",
                prompt: "保留中文项目绑定",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .expect("write local project binding fixture");

        let durable = ensure_durable_state_outside_install(&install, &state_file)
            .expect("APPDATA state must be outside replacement tree");
        assert_eq!(durable.node_config, state_file);
        assert_eq!(durable.full_access_grants, grants_file);

        let packaged_internal = root.join("package").join("_internal");
        fs::create_dir_all(packaged_internal.join("pc-next-dist"))
            .expect("create package internal fixture");
        fs::write(packaged_internal.join("README.txt"), b"new package readme")
            .expect("write package file");
        fs::write(
            packaged_internal.join("pc-next-dist").join("index.html"),
            b"new pc frontend",
        )
        .expect("write package directory");
        super::super::super::installer::copy_internal_files(&packaged_internal, &internal)
            .expect("replace package-owned internal artifacts");
        super::super::super::installer::cleanup_legacy_files(&install)
            .expect("cleanup package-owned legacy artifacts");

        let client = install.join("一龙开发平台.exe");
        let uninstall = install.join("卸载一龙开发平台.exe");
        let version_file = internal.join("node-agent-version.json");
        fs::write(&client, b"old client").expect("write old client");
        fs::write(&uninstall, b"old uninstall").expect("write old uninstall");
        fs::write(&version_file, b"old version").expect("write old version");
        let tmp_exe = internal.join("一龙开发平台.exe.new");
        let tmp_version = internal.join("node-agent-version.json.new");
        fs::write(&tmp_exe, b"new client").expect("write replacement client");
        fs::write(&tmp_version, b"new version").expect("write replacement version");
        super::super::updater_impl::replace_client_files(
            &tmp_exe,
            &client,
            &uninstall,
            &tmp_version,
            &version_file,
        )
        .expect("replace client artifacts");

        assert_eq!(fs::read(&state_file).expect("read node state"), node_config);
        assert_eq!(fs::read(&grants_file).expect("read grants"), grants);
        let snapshot = journal
            .snapshot("local-preserved", 0, 20)
            .expect("read preserved task journal");
        assert_eq!(
            snapshot.record.and_then(|record| record.cwd),
            Some(r"D:\项目\中文工作区".to_string())
        );
        let binding = local_tasks
            .get_for_owner("owner_preserved", "local-preserved")
            .expect("read preserved local task binding")
            .expect("local task binding exists");
        assert_eq!(binding.install_id, "ins_preserved");
        assert_eq!(binding.project_id, "project_chinese");
        assert_eq!(binding.workspace_path, r"D:\项目\中文工作区");
        assert_eq!(binding.runtime_permission, "full_access");
        assert_eq!(fs::read(&client).expect("read client"), b"new client");
        assert_eq!(
            fs::read(&version_file).expect("read version"),
            b"new version"
        );

        fs::remove_dir_all(root).expect("remove update compatibility fixture");
    }
}
