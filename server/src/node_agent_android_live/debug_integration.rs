use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use super::debug_integration_contract::{
    git, inspect_repository, is_ancestor, normalized_candidate, now, path_arg, plan_from_status,
    repository_identity, slot_id, validate_slot_identity,
};
pub(crate) use super::debug_integration_contract::{
    DebugArtifactStatus, DebugContribution, DebugIntegrationPlan, DebugIntegrationStatus,
    DebugMergeCandidateRequest,
};
use anyhow::{anyhow, bail, Context, Result};

pub(crate) struct DebugIntegrationCoordinator {
    root: PathBuf,
    node_fingerprint: String,
    state_lock: Mutex<()>,
}

impl Default for DebugIntegrationCoordinator {
    fn default() -> Self {
        Self::new(
            std::env::temp_dir().join("elon-android-debug-integration-tests"),
            "unscoped".into(),
        )
    }
}

impl super::broker::LiveUiBroker {
    pub(crate) fn node_install_id(&self) -> Option<&str> {
        self.debug_deployments.node_install_id()
    }

    pub(crate) fn fixed_debug_label(&self) -> Option<String> {
        self.node_install_id()
            .and_then(|install_id| super::node_debug_fingerprint(install_id).ok())
            .map(|fingerprint| format!("一龙调试 {fingerprint}"))
    }
}

impl DebugIntegrationCoordinator {
    pub(crate) fn new(root: PathBuf, node_fingerprint: String) -> Self {
        Self {
            root,
            node_fingerprint,
            state_lock: Mutex::new(()),
        }
    }

    pub(crate) fn register_candidate(
        &self,
        project_root: &str,
        project_id: &str,
        device_identity: &str,
        package_name: &str,
        candidate: Option<&DebugMergeCandidateRequest>,
        compatibility_source: &str,
        lkg_enabled: Option<bool>,
    ) -> Result<DebugIntegrationPlan> {
        let repo = inspect_repository(project_root)?;
        let candidate = normalized_candidate(candidate, compatibility_source, &repo)?;
        let slot_id = slot_id(
            &repo.identity,
            project_id,
            device_identity,
            &self.node_fingerprint,
        );
        let _guard = self.state_lock.lock().expect("debug integration lock");
        let mut status = self
            .load(&slot_id)?
            .unwrap_or_else(|| DebugIntegrationStatus {
                schema: "elon.android_debug_integration.v1".into(),
                slot_id: slot_id.clone(),
                node_fingerprint: self.node_fingerprint.clone(),
                project_id: project_id.trim().to_string(),
                device_identity: device_identity.trim().to_string(),
                package_name: package_name.trim().to_string(),
                repository_identity: repo.identity.clone(),
                base_sha: candidate.base_sha.clone(),
                desired_generation: 0,
                installed_generation: None,
                status: "EMPTY".into(),
                lkg_enabled: lkg_enabled.unwrap_or(false),
                integration_worktree: None,
                contributions: Vec::new(),
                conflicts: Vec::new(),
                legacy_packages: Vec::new(),
                preview_owner: None,
                last_error: None,
                last_usable: None,
                updated_at: now(),
            });
        validate_slot_identity(
            &status,
            &repo.identity,
            project_id,
            device_identity,
            package_name,
        )?;
        let policy_changed = lkg_enabled.is_some_and(|enabled| {
            if status.lkg_enabled == enabled {
                false
            } else {
                status.lkg_enabled = enabled;
                true
            }
        });
        if !is_ancestor(&repo.root, &status.base_sha, &repo.head)? {
            bail!(
                "DEBUG_CANDIDATE_BASE_DIVERGED: 候选提交不是固定集成基础 {} 的后继，拒绝猜测合并",
                status.base_sha
            );
        }
        let existing = status
            .contributions
            .iter()
            .map(|item| item.commit_sha.as_str())
            .collect::<Vec<_>>();
        let additions = candidate
            .commits
            .iter()
            .filter(|commit| !existing.contains(&commit.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if additions.is_empty() {
            if policy_changed {
                status.updated_at = now();
                self.save(&status)?;
            }
            return Ok(plan_from_status(&self.root, &repo.root, &status));
        }
        for commit in additions {
            status.contributions.push(DebugContribution {
                commit_sha: commit,
                source_task_id: candidate.source_task_id.clone(),
                source_session_id: candidate.source_session_id.clone(),
                accepted_at: now(),
            });
        }
        status.desired_generation = status.desired_generation.saturating_add(1);
        status.status = "QUEUED".into();
        status.conflicts.clear();
        status.last_error = None;
        status.preview_owner = candidate.preview_owner.clone().or_else(|| {
            candidate
                .source_session_id
                .clone()
                .or(candidate.source_task_id.clone())
        });
        status.updated_at = now();
        self.save(&status)?;
        Ok(plan_from_status(&self.root, &repo.root, &status))
    }

    pub(crate) fn materialize(&self, plan: &DebugIntegrationPlan) -> Result<PathBuf> {
        let current = self.assert_current(plan)?;
        let generation_root = &plan.worktree;
        if generation_root.exists() {
            if current.integration_worktree.as_deref()
                == Some(generation_root.to_string_lossy().as_ref())
            {
                return Ok(generation_root.clone());
            }
            bail!("DEBUG_INTEGRATION_WORKTREE_EXISTS: 代次工作区已存在但不属于当前已验证状态，拒绝覆盖: {}", generation_root.display());
        }
        fs::create_dir_all(generation_root.parent().context("代次工作区缺少父目录")?)?;
        git(
            &plan.source_root,
            &[
                "worktree",
                "add",
                "--detach",
                path_arg(generation_root)?,
                &plan.base_sha,
            ],
        )
        .context("创建节点托管的临时集成 worktree 失败")?;
        for commit in &plan.contributions {
            if commit == &plan.base_sha {
                continue;
            }
            if let Err(error) = git(generation_root, &["cherry-pick", commit]) {
                let _ = git(generation_root, &["cherry-pick", "--abort"]);
                self.record_failure(
                    plan,
                    "MERGE_CONFLICT",
                    format!("commit={commit}; {error:#}"),
                    vec![commit.clone()],
                )?;
                let retention = if plan.lkg_enabled {
                    "已保留显式启用的最近成功版本"
                } else {
                    "最近成功版本未启用；手机当前版本未改动"
                };
                return Err(anyhow!("DEBUG_MERGE_CONFLICT: 提交 {commit} 与当前合并序列冲突；{retention}，未猜测解决冲突。{error:#}"));
            }
        }
        self.update(plan, |status| {
            status.status = "MERGED".into();
            status.integration_worktree = Some(generation_root.display().to_string());
            status.conflicts.clear();
            status.last_error = None;
        })?;
        Ok(generation_root.clone())
    }

    pub(crate) fn mark_building(&self, plan: &DebugIntegrationPlan) -> Result<()> {
        self.update(plan, |status| status.status = "BUILDING".into())
    }

    pub(crate) fn authorize_install(&self, plan: &DebugIntegrationPlan) -> Result<()> {
        let status = self.assert_current(plan)?;
        if status.status != "BUILD_READY" {
            bail!(
                "DEBUG_DEPLOY_NOT_READY: 代次 {} 尚未完成 APK 校验",
                plan.generation
            );
        }
        Ok(())
    }

    pub(crate) fn record_artifact(
        &self,
        plan: &DebugIntegrationPlan,
        artifact: DebugArtifactStatus,
    ) -> Result<()> {
        let current = self.assert_current(plan)?;
        if plan.lkg_enabled {
            if let Some(previous) = current.last_usable.as_ref() {
                if previous.signer_sha256 != artifact.signer_sha256 {
                    self.record_failure(
                        plan,
                        "SIGNATURE_MISMATCH",
                        format!(
                            "expectedSigner={} actualSigner={}; 未安装且不会自动卸载手机应用",
                            previous.signer_sha256, artifact.signer_sha256
                        ),
                        Vec::new(),
                    )?;
                    bail!("DEBUG_APK_SIGNATURE_MISMATCH: 新 APK 签名与固定槽已锁定签名不一致；已保留最后可用 APK 和手机当前版本，不会自动卸载应用");
                }
            }
        }
        self.update(plan, |status| {
            status.status = "BUILD_READY".into();
            if plan.lkg_enabled {
                status.last_usable = Some(artifact);
            }
            status.last_error = None;
        })
    }

    pub(crate) fn artifact_root(&self, plan: &DebugIntegrationPlan) -> PathBuf {
        self.root.join(&plan.slot_id).join("artifacts")
    }

    pub(crate) fn record_deployed(&self, plan: &DebugIntegrationPlan) -> Result<()> {
        self.update(plan, |status| {
            status.status = "DEPLOYED".into();
            status.installed_generation = Some(plan.generation);
            status.last_error = None;
        })
    }

    pub(crate) fn record_legacy_packages(
        &self,
        plan: &DebugIntegrationPlan,
        packages: Vec<String>,
    ) -> Result<()> {
        self.update(plan, |status| status.legacy_packages = packages)
    }

    pub(crate) fn record_failure(
        &self,
        plan: &DebugIntegrationPlan,
        phase: &str,
        error: String,
        conflicts: Vec<String>,
    ) -> Result<()> {
        self.update(plan, |status| {
            status.status = phase.to_string();
            status.last_error = Some(error);
            status.conflicts = conflicts;
        })
    }

    pub(crate) fn record_runtime_failure(
        &self,
        plan: &DebugIntegrationPlan,
        error: String,
    ) -> Result<()> {
        if self.status(&plan.slot_id)?.is_some_and(|status| {
            status.desired_generation == plan.generation
                && matches!(
                    status.status.as_str(),
                    "MERGE_CONFLICT" | "SIGNATURE_MISMATCH"
                )
        }) {
            return Ok(());
        }
        self.record_failure(plan, "FAILED", error, Vec::new())
    }

    pub(crate) fn status(&self, slot_id: &str) -> Result<Option<DebugIntegrationStatus>> {
        let _guard = self.state_lock.lock().expect("debug integration lock");
        self.load(slot_id)
    }

    pub(crate) fn status_for(
        &self,
        project_root: &str,
        project_id: &str,
        device_identity: &str,
    ) -> Result<Option<DebugIntegrationStatus>> {
        let repository = repository_identity(project_root)?;
        let slot_id = slot_id(
            &repository,
            project_id,
            device_identity,
            &self.node_fingerprint,
        );
        self.status(&slot_id)
    }

    fn assert_current(&self, plan: &DebugIntegrationPlan) -> Result<DebugIntegrationStatus> {
        let _guard = self.state_lock.lock().expect("debug integration lock");
        let status = self.load(&plan.slot_id)?.context("固定调试槽状态不存在")?;
        if status.desired_generation != plan.generation {
            bail!(
                "DEBUG_GENERATION_SUPERSEDED: 代次 {} 已被新代次 {} 淘汰，旧构建禁止安装",
                plan.generation,
                status.desired_generation
            );
        }
        Ok(status)
    }

    fn update(
        &self,
        plan: &DebugIntegrationPlan,
        apply: impl FnOnce(&mut DebugIntegrationStatus),
    ) -> Result<()> {
        let _guard = self.state_lock.lock().expect("debug integration lock");
        let mut status = self.load(&plan.slot_id)?.context("固定调试槽状态不存在")?;
        if status.desired_generation != plan.generation {
            bail!(
                "DEBUG_GENERATION_SUPERSEDED: 代次 {} 已被新代次 {} 淘汰，旧构建禁止继续",
                plan.generation,
                status.desired_generation
            );
        }
        status.lkg_enabled = plan.lkg_enabled;
        apply(&mut status);
        status.updated_at = now();
        self.save(&status)
    }

    fn load(&self, slot_id: &str) -> Result<Option<DebugIntegrationStatus>> {
        let path = self.manifest_path(slot_id);
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .with_context(|| format!("固定调试槽状态损坏: {}", path.display()))
                .map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn save(&self, status: &DebugIntegrationStatus) -> Result<()> {
        let path = self.manifest_path(&status.slot_id);
        let bytes = serde_json::to_vec_pretty(status)?;
        crate::node_agent_atomic_file::write(&path, &bytes)
            .with_context(|| format!("无法原子写入固定调试槽状态: {}", path.display()))
    }

    fn manifest_path(&self, slot_id: &str) -> PathBuf {
        self.root.join(slot_id).join("status.json")
    }
}
