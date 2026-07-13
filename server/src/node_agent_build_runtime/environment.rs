use super::paths::BuildRunPaths;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BuildEnvironment {
    entries: Vec<(String, String)>,
}

impl BuildEnvironment {
    pub(crate) fn for_run(paths: &BuildRunPaths, project_id: &str, task_id: &str) -> Self {
        let path = |value: &std::path::Path| value.to_string_lossy().to_string();
        Self {
            entries: vec![
                ("ELON_NODE_DATA_ROOT".into(), path(&paths.root)),
                ("ELON_PROJECT_ID".into(), project_id.to_string()),
                ("ELON_BUILD_TASK_ID".into(), task_id.to_string()),
                ("ELON_RUST_TOOLCHAIN_KEY".into(), paths.toolchain_key.clone()),
                ("CARGO_TARGET_DIR".into(), path(&paths.cargo_target)),
                ("CARGO_HOME".into(), path(&paths.cargo_home)),
                ("GRADLE_USER_HOME".into(), path(&paths.gradle_home)),
                ("NPM_CONFIG_CACHE".into(), path(&paths.npm_cache)),
                ("npm_config_cache".into(), path(&paths.npm_cache)),
                ("PNPM_STORE_DIR".into(), path(&paths.pnpm_store)),
                ("npm_config_store_dir".into(), path(&paths.pnpm_store)),
                ("COREPACK_HOME".into(), path(&paths.corepack_home)),
                ("TEMP".into(), path(&paths.task_temp)),
                ("TMP".into(), path(&paths.task_temp)),
                ("TMPDIR".into(), path(&paths.task_temp)),
            ],
        }
    }

    pub(crate) fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    pub(crate) fn apply_tokio(&self, command: &mut tokio::process::Command) {
        for (key, value) in &self.entries {
            command.env(key, value);
        }
    }

    pub(crate) fn merge_into(&self, target: &mut Vec<(String, String)>) {
        for (key, value) in &self.entries {
            if let Some((_, existing)) = target
                .iter_mut()
                .find(|(candidate, _)| env_key_eq(candidate, key))
            {
                *existing = value.clone();
            } else {
                target.push((key.clone(), value.clone()));
            }
        }
    }
}

fn env_key_eq(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}
