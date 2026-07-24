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
        let loaded = self.load(&slot_id)?;
        let is_new = loaded.is_none();
        let mut status = loaded.unwrap_or_else(|| DebugIntegrationStatus {
            schema: "elon.android_debug_integration.v1".into(),
            slot_id: slot_id.clone(),
            node_fingerprint: self.node_fingerprint.clone(),
            project_id: project_id.trim().to_string(),
            device_identity: device_identity.trim().to_string(),
            package_name: package_name.trim().to_string(),
            repository_identity: repo.identity.clone(),
            base_sha: candidate.base_sha.clone(),
            source_revision: Some(candidate.source_revision.clone()),
            integration_revision: None,
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
        let base_changed = if status.base_sha == candidate.base_sha {
            false
        } else if is_ancestor(&repo.root, &status.base_sha, &candidate.base_sha)? {
            status.base_sha = candidate.base_sha.clone();
            true
        } else if is_ancestor(&repo.root, &candidate.base_sha, &status.base_sha)? {
            false
        } else {
            bail!(
                "DEBUG_CANDIDATE_BASE_DIVERGED: 候选 base {} 与固定集成基础 {} 不在同一历史，拒绝猜测合并",
                candidate.base_sha,
                status.base_sha,
            );
        };
        let prior_contribution_count = status.contributions.len();
        let mut retained = Vec::with_capacity(prior_contribution_count);
        for contribution in std::mem::take(&mut status.contributions) {
            if !is_ancestor(&repo.root, &contribution.commit_sha, &status.base_sha)? {
                retained.push(contribution);
            }
        }
        status.contributions = retained;
        let pruned_by_base = status.contributions.len() != prior_contribution_count;
        let cleared_explicit_sequence =
            candidate.commits_explicitly_empty && !status.contributions.is_empty();
        if candidate.commits_explicitly_empty {
            status.contributions.clear();
        }
        let existing = status
            .contributions
            .iter()
            .map(|item| item.commit_sha.as_str())
            .collect::<Vec<_>>();
        let additions = candidate
            .commits
            .iter()
            .filter_map(
                |commit| match is_ancestor(&repo.root, commit, &status.base_sha) {
                    Ok(true) => None,
                    Ok(false) => Some(Ok(commit.clone())),
                    Err(error) => Some(Err(error)),
                },
            )
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|commit| !existing.contains(&commit.as_str()))
            .collect::<Vec<_>>();
        let sequence_changed = is_new
            || base_changed
            || pruned_by_base
            || cleared_explicit_sequence
            || !additions.is_empty();
        let source_revision_changed =
            status.source_revision.as_deref() != Some(candidate.source_revision.as_str());
        status.source_revision = Some(candidate.source_revision.clone());
        if !sequence_changed {
            if policy_changed || source_revision_changed {
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
        status.desired_generation =
            self.next_available_generation(&status.slot_id, status.desired_generation)?;
        status.status = "QUEUED".into();
        status.integration_worktree = None;
        status.integration_revision = None;
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

    pub(crate) fn restart_failed_generation(
        &self,
        plan: &DebugIntegrationPlan,
    ) -> Result<DebugIntegrationPlan> {
        let _guard = self.state_lock.lock().expect("debug integration lock");
        let mut status = self.load(&plan.slot_id)?.context("固定调试槽状态不存在")?;
        if status.desired_generation != plan.generation {
            return Ok(plan_from_status(&self.root, &plan.source_root, &status));
        }
        if !matches!(
            status.status.as_str(),
            "FAILED" | "MERGE_CONFLICT" | "SIGNATURE_MISMATCH"
        ) {
            bail!(
                "DEBUG_RESTART_NOT_FAILED: 代次 {} 当前状态 {}，只有 FAILED 代次可以重启",
                plan.generation,
                status.status
            );
        }
        // A runtime/build/install failure does not change the candidate source.
        // Keep the verified generation worktree so Gradle can reuse its partial
        // or completed outputs. Allocating a new generation here previously
        // converted every timeout into a cold 125-task rebuild.
        if status.status == "FAILED" && generation_is_clean_and_owned(&status, &plan.worktree)? {
            status.status = "MERGED".into();
            status.conflicts.clear();
            status.last_error = None;
            status.updated_at = now();
            self.save(&status)?;
            return Ok(plan_from_status(&self.root, &plan.source_root, &status));
        }
        status.desired_generation =
            self.next_available_generation(&status.slot_id, status.desired_generation)?;
        status.status = "QUEUED".into();
        status.integration_worktree = None;
        status.integration_revision = None;
        status.conflicts.clear();
        status.last_error = None;
        status.updated_at = now();
        self.save(&status)?;
        Ok(plan_from_status(&self.root, &plan.source_root, &status))
    }

    pub(crate) fn materialize(&self, plan: &DebugIntegrationPlan) -> Result<PathBuf> {
        let current = self.assert_current(plan)?;
        let generation_root = &plan.worktree;
        if generation_root.exists() {
            if current.integration_worktree.as_deref()
                == Some(generation_root.to_string_lossy().as_ref())
            {
                if current.integration_revision.is_none() {
                    let integration_revision = git(generation_root, &["rev-parse", "HEAD"])?
                        .trim()
                        .to_string();
                    self.update(plan, |status| {
                        status.integration_revision = Some(integration_revision)
                    })?;
                }
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
            if is_ancestor(generation_root, commit, "HEAD")? {
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
        let integration_revision = git(generation_root, &["rev-parse", "HEAD"])?
            .trim()
            .to_string();
        self.update(plan, |status| {
            status.status = "MERGED".into();
            status.integration_worktree = Some(generation_root.display().to_string());
            status.integration_revision = Some(integration_revision);
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

    fn next_available_generation(&self, slot_id: &str, current: u64) -> Result<u64> {
        let mut generation = current
            .checked_add(1)
            .context("DEBUG_GENERATION_EXHAUSTED: 调试集成 generation 已耗尽")?;
        while self
            .root
            .join(slot_id)
            .join("generations")
            .join(format!("generation-{generation}"))
            .exists()
        {
            generation = generation
                .checked_add(1)
                .context("DEBUG_GENERATION_EXHAUSTED: 调试集成 generation 已耗尽")?;
        }
        Ok(generation)
    }
}

fn generation_is_clean_and_owned(
    status: &DebugIntegrationStatus,
    expected_worktree: &std::path::Path,
) -> Result<bool> {
    let Some(recorded_worktree) = status.integration_worktree.as_deref() else {
        return Ok(false);
    };
    let recorded_worktree = PathBuf::from(recorded_worktree);
    if !recorded_worktree.exists()
        || recorded_worktree.canonicalize()? != expected_worktree.canonicalize()?
    {
        return Ok(false);
    }
    let Some(expected_revision) = status.integration_revision.as_deref() else {
        return Ok(false);
    };
    let actual_revision = git(&recorded_worktree, &["rev-parse", "HEAD"])?;
    if actual_revision.trim() != expected_revision {
        return Ok(false);
    }
    Ok(git(
        &recorded_worktree,
        &["status", "--porcelain", "--untracked-files=no"],
    )?
    .trim()
    .is_empty())
}
