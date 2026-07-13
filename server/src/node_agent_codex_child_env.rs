use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FrozenCodexHome {
    canonical_path: String,
    requires_cloud_control: bool,
    managed_marker: Option<crate::node_agent_codex_vault_active::ManagedCodexHomeMarkerIdentity>,
}

impl FrozenCodexHome {
    /// Select CODEX_HOME once for a task. The returned path is canonical and
    /// must be passed explicitly to the child; task execution must never read
    /// the process-wide environment again.
    pub(crate) fn capture_for_task() -> Result<Self> {
        let selected = selected_codex_home_for_task()?;
        freeze_selected_codex_home(&selected)
    }

    pub(crate) fn capture_unmanaged_for_local_task() -> Result<Self> {
        let frozen = Self::capture_for_task()?;
        if frozen.requires_cloud_control {
            bail!("当前 Codex 凭据由云端保险箱管理；本机离线任务必须切回自己的本地 Codex 登录。");
        }
        Ok(frozen)
    }

    pub(crate) fn path(&self) -> &str {
        &self.canonical_path
    }

    pub(crate) fn requires_cloud_control(&self) -> bool {
        self.requires_cloud_control
    }

    pub(crate) fn managed_lease_expires_at(&self) -> Option<&str> {
        self.managed_marker
            .as_ref()
            .and_then(|marker| marker.lease_expires_at())
    }

    pub(crate) fn validate_cloud_binding(
        &self,
        binding: &homecli_proto::CliCodexCredentialBinding,
    ) -> Result<()> {
        if binding.managed != self.requires_cloud_control {
            bail!("云端授权的 Codex 凭据类型与本机冻结的 CODEX_HOME 不一致。")
        }
        if !binding.managed {
            if binding
                .lease_id
                .as_deref()
                .is_some_and(|lease_id| !lease_id.trim().is_empty())
            {
                bail!("云端 unmanaged Codex 授权不能携带 lease_id。")
            }
            return Ok(());
        }
        let expected_lease_id = binding
            .lease_id
            .as_deref()
            .filter(|lease_id| !lease_id.is_empty() && *lease_id == lease_id.trim())
            .ok_or_else(|| anyhow!("云端 managed Codex 授权缺少有效 lease_id。"))?;
        let frozen_lease_id = self
            .managed_marker
            .as_ref()
            .and_then(|marker| marker.lease_id())
            .ok_or_else(|| anyhow!("本机托管 CODEX_HOME marker 缺少 lease_id。"))?;
        if frozen_lease_id != expected_lease_id {
            bail!("云端授权的 Codex lease_id 与本机冻结 marker 不一致。")
        }
        Ok(())
    }

    /// Revalidate the frozen filesystem identity immediately before launch.
    /// This catches deleted paths and symlink/junction retargeting.
    pub(crate) fn validate_for_task(
        &self,
        local_offline: bool,
        cloud_connected: bool,
    ) -> Result<()> {
        let frozen_path = PathBuf::from(&self.canonical_path);
        let current_path = std::fs::canonicalize(&frozen_path).with_context(|| {
            format!("任务冻结的 CODEX_HOME 已不可用：{}", frozen_path.display())
        })?;
        if !current_path.is_dir() {
            bail!("任务冻结的 CODEX_HOME 不是目录：{}", current_path.display());
        }
        if current_path != frozen_path {
            bail!(
                "任务冻结的 CODEX_HOME 文件系统身份已变化，拒绝启动：{}",
                frozen_path.display()
            );
        }
        let managed =
            crate::node_agent_codex_vault_active::codex_home_path_is_managed(&current_path);
        if managed != self.requires_cloud_control {
            bail!("任务冻结的 CODEX_HOME 云控属性已变化，拒绝启动。")
        }
        let current_marker = if managed {
            Some(
                crate::node_agent_codex_vault_active::managed_codex_home_marker_identity(
                    &current_path,
                )
                .context("任务冻结的托管 CODEX_HOME marker 无效")?,
            )
        } else {
            None
        };
        if current_marker != self.managed_marker {
            bail!("任务冻结的托管 CODEX_HOME marker 身份已变化，拒绝启动。")
        }
        if local_offline && managed {
            bail!("本机离线任务不能使用云端保险箱或朋友共享 Codex 凭据。")
        }
        if managed && !cloud_connected {
            bail!("云端连接已断开，云端保险箱或朋友共享 Codex 任务不能启动。")
        }
        Ok(())
    }
}

fn freeze_selected_codex_home(path: &Path) -> Result<FrozenCodexHome> {
    if !path.is_dir() {
        bail!("CODEX_HOME 不存在或不是目录：{}", path.display());
    }
    let canonical = std::fs::canonicalize(path)
        .with_context(|| format!("无法规范化 CODEX_HOME：{}", path.display()))?;
    let requires_cloud_control =
        crate::node_agent_codex_vault_active::codex_home_path_is_managed(&canonical);
    let managed_marker = if requires_cloud_control {
        Some(
            crate::node_agent_codex_vault_active::managed_codex_home_marker_identity(&canonical)
                .context("托管 CODEX_HOME marker 校验失败")?,
        )
    } else {
        None
    };
    Ok(FrozenCodexHome {
        canonical_path: canonical.to_string_lossy().into_owned(),
        requires_cloud_control,
        managed_marker,
    })
}

fn selected_codex_home_for_task() -> Result<PathBuf> {
    if let Some(active) = std::env::var("CODEX_HOME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        // A path under the managed root is authoritative even when its
        // directory/marker is missing: falling back to the owner's default
        // home would silently cross the task's credential boundary.
        if active.exists()
            || crate::node_agent_codex_vault_active::codex_home_path_is_managed(&active)
        {
            return Ok(active);
        }
    }
    default_codex_home_env()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("没有找到可用的本机 CODEX_HOME，请先登录 Codex。"))
}

#[cfg(test)]
fn codex_child_home_env() -> Option<String> {
    crate::node_agent_codex_vault_active::current_valid_codex_home_env()
        .or_else(default_codex_home_env)
}

#[cfg(test)]
fn codex_child_home_env_assignment() -> Option<(&'static str, String)> {
    codex_child_home_env().map(|home| ("CODEX_HOME", home))
}

fn default_codex_home_env() -> Option<String> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(|home| Path::new(&home).join(".codex"))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::{codex_child_home_env, codex_child_home_env_assignment, FrozenCodexHome};
    use std::path::Path;
    use std::process::Command;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn codex_child_home_prefers_active_shared_vault_home() {
        let _guard = env_lock();
        let temp = std::env::temp_dir().join(format!(
            "elon-codex-child-env-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let shared_home = temp
            .join("Elon")
            .join("codex-vault")
            .join("slots")
            .join("shared-provider-slot")
            .join("codex-home");
        std::fs::create_dir_all(&shared_home).unwrap();
        std::fs::write(
            shared_home.join("elon-codex-vault-slot.json"),
            r#"{"slot_id":"shared-provider-slot","lease_id":"lease-1","lease_expires_at":"2999-01-01T00:00:00+00:00"}"#,
        )
        .unwrap();

        let old_local = std::env::var("LOCALAPPDATA").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        let old_codex = std::env::var("CODEX_HOME").ok();
        std::env::set_var("LOCALAPPDATA", temp.as_os_str());
        std::env::set_var("XDG_DATA_HOME", temp.as_os_str());
        std::env::set_var("CODEX_HOME", shared_home.as_os_str());
        let selected = codex_child_home_env();
        restore_env("LOCALAPPDATA", old_local);
        restore_env("XDG_DATA_HOME", old_xdg);
        restore_env("CODEX_HOME", old_codex);
        let _ = std::fs::remove_dir_all(&temp);

        assert_path_eq(selected.as_deref(), &shared_home);
    }

    #[test]
    fn fake_codex_child_receives_shared_vault_home_without_real_cli() {
        let _guard = env_lock();
        let temp = std::env::temp_dir().join(format!(
            "elon-codex-child-fake-cli-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let shared_home = temp
            .join("Elon")
            .join("codex-vault")
            .join("slots")
            .join("shared-provider-slot")
            .join("codex-home");
        std::fs::create_dir_all(&shared_home).unwrap();
        std::fs::write(
            shared_home.join("elon-codex-vault-slot.json"),
            r#"{"slot_id":"shared-provider-slot","lease_id":"lease-1","lease_expires_at":"2999-01-01T00:00:00+00:00"}"#,
        )
        .unwrap();

        let old_local = std::env::var("LOCALAPPDATA").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        let old_codex = std::env::var("CODEX_HOME").ok();
        std::env::set_var("LOCALAPPDATA", temp.as_os_str());
        std::env::set_var("XDG_DATA_HOME", temp.as_os_str());
        std::env::set_var("CODEX_HOME", shared_home.as_os_str());
        let assignment = codex_child_home_env_assignment();
        restore_env("LOCALAPPDATA", old_local);
        restore_env("XDG_DATA_HOME", old_xdg);
        restore_env("CODEX_HOME", old_codex);

        let (name, home) = assignment.expect("child CODEX_HOME assignment");
        let mut child = fake_codex_child_command();
        child.env(name, &home).env("EXPECTED_CODEX_HOME", &home);
        let output = child.output().expect("fake child should run");
        let _ = std::fs::remove_dir_all(&temp);

        assert!(
            output.status.success(),
            "fake child failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_path_eq(Some(&home), &shared_home);
    }

    #[test]
    fn codex_child_home_ignores_expired_shared_vault_home() {
        let _guard = env_lock();
        let temp = std::env::temp_dir().join(format!(
            "elon-codex-child-env-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let shared_home = temp
            .join("Elon")
            .join("codex-vault")
            .join("slots")
            .join("shared-provider-slot")
            .join("codex-home");
        let profile = temp.join("profile");
        let default_home = profile.join(".codex");
        std::fs::create_dir_all(&shared_home).unwrap();
        std::fs::create_dir_all(&default_home).unwrap();
        std::fs::write(
            shared_home.join("elon-codex-vault-slot.json"),
            r#"{"slot_id":"shared-provider-slot","lease_id":"lease-1","lease_expires_at":"2000-01-01T00:00:00+00:00"}"#,
        )
        .unwrap();

        let old_local = std::env::var("LOCALAPPDATA").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        let old_codex = std::env::var("CODEX_HOME").ok();
        let old_profile = std::env::var("USERPROFILE").ok();
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("LOCALAPPDATA", temp.as_os_str());
        std::env::set_var("XDG_DATA_HOME", temp.as_os_str());
        std::env::set_var("CODEX_HOME", shared_home.as_os_str());
        std::env::set_var("USERPROFILE", profile.as_os_str());
        std::env::remove_var("HOME");
        let selected = codex_child_home_env();
        restore_env("LOCALAPPDATA", old_local);
        restore_env("XDG_DATA_HOME", old_xdg);
        restore_env("CODEX_HOME", old_codex);
        restore_env("USERPROFILE", old_profile);
        restore_env("HOME", old_home);
        let _ = std::fs::remove_dir_all(&temp);

        assert_path_eq(selected.as_deref(), &default_home);
    }

    #[test]
    fn local_task_freezes_unmanaged_home_and_ignores_later_global_switch() {
        let _guard = env_lock();
        let temp = std::env::temp_dir().join(format!(
            "elon-codex-frozen-local-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let owner_home = temp.join("owner").join(".codex");
        let managed_home = temp
            .join("Elon")
            .join("codex-vault")
            .join("slots")
            .join("shared-provider-slot")
            .join("codex-home");
        std::fs::create_dir_all(&owner_home).unwrap();
        std::fs::create_dir_all(&managed_home).unwrap();
        let canonical_owner_home = std::fs::canonicalize(&owner_home).unwrap();
        std::fs::write(
            managed_home.join("elon-codex-vault-slot.json"),
            r#"{"slot_id":"shared-provider-slot","lease_id":"lease-1","lease_expires_at":"2999-01-01T00:00:00+00:00"}"#,
        )
        .unwrap();

        let old_local = std::env::var("LOCALAPPDATA").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        let old_codex = std::env::var("CODEX_HOME").ok();
        std::env::set_var("LOCALAPPDATA", temp.as_os_str());
        std::env::set_var("XDG_DATA_HOME", temp.as_os_str());
        std::env::set_var("CODEX_HOME", owner_home.as_os_str());
        let frozen = FrozenCodexHome::capture_unmanaged_for_local_task().unwrap();

        // Simulate a concurrent cloud vault restore after local task creation.
        std::env::set_var("CODEX_HOME", managed_home.as_os_str());
        frozen.validate_for_task(true, false).unwrap();

        let mut child = fake_codex_child_command();
        child
            .env("CODEX_HOME", frozen.path())
            .env("EXPECTED_CODEX_HOME", frozen.path());
        let output = child.output().expect("fake child should use frozen home");

        restore_env("LOCALAPPDATA", old_local);
        restore_env("XDG_DATA_HOME", old_xdg);
        restore_env("CODEX_HOME", old_codex);
        let _ = std::fs::remove_dir_all(&temp);

        assert!(output.status.success());
        assert_path_eq(Some(frozen.path()), &canonical_owner_home);
        assert!(!frozen.requires_cloud_control());
        assert!(frozen
            .validate_cloud_binding(&homecli_proto::CliCodexCredentialBinding {
                managed: false,
                lease_id: None,
            })
            .is_ok());
        assert!(frozen
            .validate_cloud_binding(&homecli_proto::CliCodexCredentialBinding {
                managed: true,
                lease_id: Some("lease-1".to_string()),
            })
            .is_err());
    }

    #[test]
    fn managed_frozen_home_requires_live_cloud_control() {
        let _guard = env_lock();
        let temp = std::env::temp_dir().join(format!(
            "elon-codex-frozen-managed-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let managed_home = temp
            .join("Elon")
            .join("codex-vault")
            .join("slots")
            .join("shared-provider-slot")
            .join("codex-home");
        std::fs::create_dir_all(&managed_home).unwrap();
        std::fs::write(
            managed_home.join("elon-codex-vault-slot.json"),
            r#"{"slot_id":"shared-provider-slot","lease_id":"lease-1","lease_expires_at":"2999-01-01T00:00:00+00:00"}"#,
        )
        .unwrap();

        let old_local = std::env::var("LOCALAPPDATA").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        let old_codex = std::env::var("CODEX_HOME").ok();
        std::env::set_var("LOCALAPPDATA", temp.as_os_str());
        std::env::set_var("XDG_DATA_HOME", temp.as_os_str());
        std::env::set_var("CODEX_HOME", managed_home.as_os_str());
        let frozen = FrozenCodexHome::capture_for_task().unwrap();

        assert!(frozen.requires_cloud_control());
        assert!(frozen.validate_for_task(false, true).is_ok());
        assert!(frozen.validate_for_task(false, false).is_err());
        assert!(frozen.validate_for_task(true, true).is_err());
        assert!(FrozenCodexHome::capture_unmanaged_for_local_task().is_err());
        assert!(frozen
            .validate_cloud_binding(&homecli_proto::CliCodexCredentialBinding {
                managed: true,
                lease_id: Some("lease-1".to_string()),
            })
            .is_ok());
        assert!(frozen
            .validate_cloud_binding(&homecli_proto::CliCodexCredentialBinding {
                managed: true,
                lease_id: Some("lease-other".to_string()),
            })
            .is_err());
        assert!(frozen
            .validate_cloud_binding(&homecli_proto::CliCodexCredentialBinding {
                managed: false,
                lease_id: None,
            })
            .is_err());

        restore_env("LOCALAPPDATA", old_local);
        restore_env("XDG_DATA_HOME", old_xdg);
        restore_env("CODEX_HOME", old_codex);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn managed_home_marker_is_fail_closed_but_legacy_marker_is_allowed_online() {
        let _guard = env_lock();
        let temp = std::env::temp_dir().join(format!(
            "elon-codex-marker-validation-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let managed_home = temp
            .join("Elon")
            .join("codex-vault")
            .join("slots")
            .join("legacy-slot")
            .join("codex-home");
        std::fs::create_dir_all(&managed_home).unwrap();

        let old_local = std::env::var("LOCALAPPDATA").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        let old_codex = std::env::var("CODEX_HOME").ok();
        std::env::set_var("LOCALAPPDATA", temp.as_os_str());
        std::env::set_var("XDG_DATA_HOME", temp.as_os_str());
        std::env::set_var("CODEX_HOME", managed_home.as_os_str());
        let marker = managed_home.join("elon-codex-vault-slot.json");

        assert!(FrozenCodexHome::capture_for_task().is_err());
        std::fs::write(&marker, "{").unwrap();
        assert!(FrozenCodexHome::capture_for_task().is_err());
        std::fs::write(&marker, r#"{"slot_id":""}"#).unwrap();
        assert!(FrozenCodexHome::capture_for_task().is_err());
        std::fs::write(&marker, r#"{"slot_id":"shared-provider"}"#).unwrap();
        assert!(FrozenCodexHome::capture_for_task().is_err());
        std::fs::write(
            &marker,
            r#"{"slot_id":"shared-provider","lease_id":"lease-1"}"#,
        )
        .unwrap();
        assert!(FrozenCodexHome::capture_for_task().is_err());

        std::fs::write(
            &marker,
            r#"{"slot_id":"legacy-slot","lease_id":null,"lease_expires_at":null}"#,
        )
        .unwrap();
        let legacy = FrozenCodexHome::capture_for_task().expect("valid legacy managed marker");
        assert!(legacy.requires_cloud_control());
        assert!(legacy.validate_for_task(false, true).is_ok());
        assert!(legacy.validate_for_task(false, false).is_err());

        restore_env("LOCALAPPDATA", old_local);
        restore_env("XDG_DATA_HOME", old_xdg);
        restore_env("CODEX_HOME", old_codex);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn frozen_managed_home_rejects_marker_rebinding() {
        let _guard = env_lock();
        let temp = std::env::temp_dir().join(format!(
            "elon-codex-marker-rebind-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let managed_home = temp
            .join("Elon")
            .join("codex-vault")
            .join("slots")
            .join("shared-provider")
            .join("codex-home");
        std::fs::create_dir_all(&managed_home).unwrap();
        let marker = managed_home.join("elon-codex-vault-slot.json");
        std::fs::write(
            &marker,
            r#"{"slot_id":"shared-provider","lease_id":"lease-1","lease_expires_at":"2999-01-01T00:00:00+00:00","account_hint_hash":"account-a"}"#,
        )
        .unwrap();

        let old_local = std::env::var("LOCALAPPDATA").ok();
        let old_xdg = std::env::var("XDG_DATA_HOME").ok();
        let old_codex = std::env::var("CODEX_HOME").ok();
        std::env::set_var("LOCALAPPDATA", temp.as_os_str());
        std::env::set_var("XDG_DATA_HOME", temp.as_os_str());
        std::env::set_var("CODEX_HOME", managed_home.as_os_str());
        let frozen = FrozenCodexHome::capture_for_task().expect("freeze shared home");

        std::fs::write(
            &marker,
            r#"{"slot_id":"shared-provider","lease_id":"lease-1","lease_expires_at":"2999-01-01T00:00:00+00:00","account_hint_hash":"account-b"}"#,
        )
        .unwrap();
        let error = frozen
            .validate_for_task(false, true)
            .expect_err("marker content swap must invalidate frozen home");
        assert!(error.to_string().contains("marker 身份已变化"));

        restore_env("LOCALAPPDATA", old_local);
        restore_env("XDG_DATA_HOME", old_xdg);
        restore_env("CODEX_HOME", old_codex);
        let _ = std::fs::remove_dir_all(&temp);
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn assert_path_eq(actual: Option<&str>, expected: &Path) {
        let actual = actual.expect("CODEX_HOME should be selected");
        assert_eq!(Path::new(actual), expected);
    }

    fn restore_env(name: &str, value: Option<String>) {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }

    fn fake_codex_child_command() -> Command {
        if cfg!(windows) {
            let mut cmd = Command::new("cmd");
            cmd.args([
                "/C",
                "if \"%CODEX_HOME%\"==\"%EXPECTED_CODEX_HOME%\" (exit /b 0) else (echo CODEX_HOME=%CODEX_HOME% expected=%EXPECTED_CODEX_HOME% & exit /b 1)",
            ]);
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.args([
                "-c",
                "test \"$CODEX_HOME\" = \"$EXPECTED_CODEX_HOME\" || { echo \"CODEX_HOME=$CODEX_HOME expected=$EXPECTED_CODEX_HOME\"; exit 1; }",
            ]);
            cmd
        }
    }
}
