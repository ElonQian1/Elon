use std::path::Path;

pub(crate) fn codex_child_home_env() -> Option<String> {
    crate::node_agent_codex_vault_active::current_valid_codex_home_env()
        .or_else(default_codex_home_env)
}

pub(crate) fn codex_child_home_env_assignment() -> Option<(&'static str, String)> {
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
    use super::{codex_child_home_env, codex_child_home_env_assignment};
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
            r#"{"slot_id":"shared-provider-slot","lease_expires_at":"2999-01-01T00:00:00+00:00"}"#,
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
            r#"{"slot_id":"shared-provider-slot","lease_expires_at":"2999-01-01T00:00:00+00:00"}"#,
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
            r#"{"slot_id":"shared-provider-slot","lease_expires_at":"2000-01-01T00:00:00+00:00"}"#,
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
