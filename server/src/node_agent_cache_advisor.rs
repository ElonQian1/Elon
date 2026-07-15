//! Read-only inventory for the cache layouts that already exist on a PC.
//!
//! The advisor records paths and produces suggestions. It never claims,
//! moves, rewrites, or deletes a discovered external cache.

use elon_pc_dev_runtime::NodeDataPaths;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const ADVISOR_SCHEMA_VERSION: u32 = 1;
const MAX_OBSERVED_WORKSPACES: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct CacheArchitectureAdvisor {
    state: Arc<Mutex<AdvisorState>>,
    persistence_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AdvisorState {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    observed_workspaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CacheArchitectureReport {
    schema_version: u32,
    mode: &'static str,
    summary: &'static str,
    candidates: Vec<CacheCandidate>,
    suggestions: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CacheCandidate {
    kind: &'static str,
    label: &'static str,
    path: String,
    source: String,
    scope: &'static str,
    exists: bool,
    managed_by_platform: bool,
    automatic_action: &'static str,
    estimated_bytes: Option<u64>,
    recommendation: &'static str,
}

#[derive(Debug, Clone)]
struct CandidateSeed {
    kind: &'static str,
    label: &'static str,
    path: PathBuf,
    source: String,
    scope: &'static str,
    managed_by_platform: bool,
    recommendation: &'static str,
}

impl CacheArchitectureAdvisor {
    pub(crate) fn load_default() -> Self {
        let state = std::fs::read_to_string(state_path())
            .ok()
            .and_then(|payload| serde_json::from_str::<AdvisorState>(&payload).ok())
            .filter(|state| state.schema_version == ADVISOR_SCHEMA_VERSION)
            .unwrap_or_else(|| AdvisorState {
                schema_version: ADVISOR_SCHEMA_VERSION,
                observed_workspaces: Vec::new(),
            });
        Self {
            state: Arc::new(Mutex::new(state)),
            persistence_path: Some(state_path()),
        }
    }

    pub(crate) fn observe_workspace(&self, workspace: &Path) {
        let canonical =
            std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
        if !canonical.is_absolute() {
            return;
        }
        let text = canonical.to_string_lossy().to_string();
        let changed = self
            .state
            .lock()
            .ok()
            .map(|mut state| {
                if state
                    .observed_workspaces
                    .iter()
                    .any(|item| path_text_eq(item, &text))
                {
                    return false;
                }
                state.observed_workspaces.push(text);
                if state.observed_workspaces.len() > MAX_OBSERVED_WORKSPACES {
                    let remove = state.observed_workspaces.len() - MAX_OBSERVED_WORKSPACES;
                    state.observed_workspaces.drain(0..remove);
                }
                if let Some(path) = self.persistence_path.as_deref() {
                    persist(path, &state);
                }
                true
            })
            .unwrap_or(false);
        if changed {
            tracing::info!(workspace = %canonical.display(), "项目数据架构体检器已登记工作区；不会移动或清理原目录");
        }
    }

    pub(crate) fn report(
        &self,
        data_paths: Option<&NodeDataPaths>,
        include_sizes: bool,
    ) -> CacheArchitectureReport {
        let observed = self
            .state
            .lock()
            .ok()
            .map(|state| state.observed_workspaces.clone())
            .unwrap_or_default();
        let mut seeds = Vec::new();
        if let Some(paths) = data_paths {
            seeds.push(CandidateSeed {
                kind: "managed_data_root",
                label: "一龙推荐数据根缓存",
                path: paths.cache(),
                source: "node_data_root".to_string(),
                scope: "platform_managed",
                managed_by_platform: true,
                recommendation:
                    "新建托管项目可继续使用；平台只管理该数据根内自己创建的缓存和临时文件。",
            });
        }
        add_environment_candidates(&mut seeds);
        add_default_candidates(&mut seeds);
        for workspace in observed {
            add_workspace_candidates(&mut seeds, Path::new(&workspace));
        }

        let mut seen = HashSet::new();
        let candidates = seeds
            .into_iter()
            .filter(|seed| seed.path.is_absolute())
            .filter(|seed| seen.insert(normalized_key(&seed.path)))
            .map(|seed| {
                let exists = seed.path.is_dir();
                let estimated_bytes = (include_sizes && exists)
                    .then(|| crate::node_agent_build_runtime::directory_size(&seed.path));
                CacheCandidate {
                    kind: seed.kind,
                    label: seed.label,
                    path: seed.path.to_string_lossy().to_string(),
                    source: seed.source,
                    scope: seed.scope,
                    exists,
                    managed_by_platform: seed.managed_by_platform,
                    automatic_action: "none",
                    estimated_bytes,
                    recommendation: seed.recommendation,
                }
            })
            .filter(|candidate| candidate.exists || candidate.managed_by_platform)
            .collect();

        CacheArchitectureReport {
            schema_version: ADVISOR_SCHEMA_VERSION,
            mode: "advisory",
            summary: "只读识别并给出渐进整理建议；不会因升级移动、接管或清理已有项目和外部缓存。",
            candidates,
            suggestions: vec![
                "先复用已经验证成功的项目路径和共享缓存，再根据工具链、target triple、profile 与 features 判断是否值得迁移。",
                "开发检查、Win 节点发布、服务器发布使用不同缓存作用域，不强行合并成一个 target。",
                "迁移必须先预览、可回滚并由用户或 AI 明确执行；外部缓存永远不进入平台自动清理范围。",
            ],
        }
    }
}

impl Default for CacheArchitectureAdvisor {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(AdvisorState {
                schema_version: ADVISOR_SCHEMA_VERSION,
                observed_workspaces: Vec::new(),
            })),
            persistence_path: None,
        }
    }
}

fn add_environment_candidates(seeds: &mut Vec<CandidateSeed>) {
    for (key, kind, label, scope, recommendation) in candidate_variables() {
        let Some(path) = std::env::var(key)
            .ok()
            .map(|value| PathBuf::from(value.trim()))
            .filter(|path| path.is_absolute())
        else {
            continue;
        };
        seeds.push(CandidateSeed {
            kind,
            label,
            path,
            source: format!("environment:{key}"),
            scope,
            managed_by_platform: false,
            recommendation,
        });
    }
}

fn add_default_candidates(seeds: &mut Vec<CandidateSeed>) {
    let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return;
    };
    let root = local.join("Elon").join("build-target");
    seeds.push(seed(
        "development_shared",
        "当前开发检查、测试共享缓存",
        root.join("elon-dev-cargo"),
        "windows_default",
        "product_family",
        "保持跨 worktree 复用；由 cargo-dev 锁保护，不迁入用户项目级缓存。",
    ));
    seeds.push(seed(
        "windows_node_release",
        "Win 节点发布缓存",
        root.join("elon-node-agent"),
        "windows_default",
        "release",
        "继续作为 Win 节点发布专用缓存；不要与开发或服务器发布 target 混用。",
    ));
}

fn add_workspace_candidates(seeds: &mut Vec<CandidateSeed>, workspace: &Path) {
    for directory in workspace.ancestors().take(8) {
        add_dotenv_candidates(seeds, &directory.join(".env.local"));
        let shared = directory.join("shared");
        seeds.push(seed(
            "historical_shared_rust",
            "历史跨子项目共享 Rust 缓存",
            shared.join("target"),
            "workspace_convention",
            "machine_shared",
            "登记为可信外部共享缓存并原地复用；平台不自动清理，兼容性变化时只重建受影响部分。",
        ));
        seeds.push(seed(
            "server_release_shared",
            "服务器发布共享缓存",
            shared.join("server-musl-target"),
            "workspace_convention",
            "release",
            "继续由服务器发布脚本复用；与 Windows target 和开发 profile 分开。",
        ));
    }
    seeds.push(seed(
        "repository_legacy",
        "仓库旧缓存",
        workspace.join("target"),
        "workspace",
        "repository",
        "先确认最后使用的构建入口；可继续复用，不自动移动或删除。",
    ));
    seeds.push(seed(
        "repository_legacy",
        "仓库旧缓存",
        workspace.join("server").join("target"),
        "workspace",
        "repository",
        "先确认最后使用的构建入口；可继续复用，不自动移动或删除。",
    ));
}

fn add_dotenv_candidates(seeds: &mut Vec<CandidateSeed>, path: &Path) {
    let Ok(payload) = std::fs::read_to_string(path) else {
        return;
    };
    for line in payload.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches(['\'', '"']);
        let Some((_, kind, label, scope, recommendation)) = candidate_variables()
            .into_iter()
            .find(|(candidate, ..)| candidate.eq_ignore_ascii_case(key))
        else {
            continue;
        };
        let candidate = PathBuf::from(value);
        let candidate = if candidate.is_absolute() {
            candidate
        } else {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(candidate)
        };
        seeds.push(CandidateSeed {
            kind,
            label,
            path: candidate,
            source: format!("dotenv:{}:{key}", path.display()),
            scope,
            managed_by_platform: false,
            recommendation,
        });
    }
}

fn candidate_variables() -> [(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
); 5] {
    [
        (
            "CARGO_TARGET_DIR",
            "historical_shared_rust",
            "历史跨子项目共享 Rust 缓存",
            "machine_shared",
            "登记为可信外部共享缓存并原地复用；平台不自动清理，兼容性变化时只重建受影响部分。",
        ),
        (
            "ELON_DEV_CARGO_TARGET_DIR",
            "development_shared",
            "当前开发检查、测试共享缓存",
            "product_family",
            "保持跨 worktree 复用；由 cargo-dev 锁保护，不迁入用户项目级缓存。",
        ),
        (
            "ELON_NODE_AGENT_TARGET_DIR",
            "windows_node_release",
            "Win 节点发布缓存",
            "release",
            "继续作为 Win 节点发布专用缓存；不要与开发或服务器发布 target 混用。",
        ),
        (
            "RUST_SERVER_MUSL_TARGET_DIR",
            "server_release_shared",
            "服务器发布共享缓存",
            "release",
            "继续由服务器发布脚本复用；与 Windows target 和开发 profile 分开。",
        ),
        (
            "ELON_BUILD_TARGET_DIR",
            "server_release_shared",
            "服务器发布共享缓存",
            "release",
            "继续由服务器发布脚本复用；与 Windows target 和开发 profile 分开。",
        ),
    ]
}

fn seed(
    kind: &'static str,
    label: &'static str,
    path: PathBuf,
    source: &'static str,
    scope: &'static str,
    recommendation: &'static str,
) -> CandidateSeed {
    CandidateSeed {
        kind,
        label,
        path,
        source: source.to_string(),
        scope,
        managed_by_platform: false,
        recommendation,
    }
}

fn state_path() -> PathBuf {
    crate::node_agent_config::state_path().with_file_name("cache-advisor.json")
}

fn persist(path: &Path, state: &AdvisorState) {
    let Ok(payload) = serde_json::to_vec_pretty(state) else {
        return;
    };
    if let Err(error) = crate::node_agent_atomic_file::write(path, &payload) {
        tracing::warn!(error = %error, "无法持久化项目数据架构体检记录");
    }
}

fn path_text_eq(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn normalized_key(path: &Path) -> String {
    let text = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn workspace_conventions_discover_shared_and_repository_caches_without_claiming_them() {
        let root = unique_root("inventory");
        let workspace = root.join("projects").join("app");
        let shared = root.join("shared").join("target");
        let repository = workspace.join("server").join("target");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::create_dir_all(&repository).unwrap();
        std::fs::write(shared.join("artifact"), b"shared").unwrap();
        std::fs::write(repository.join("artifact"), b"repo").unwrap();

        let advisor = CacheArchitectureAdvisor::default();
        advisor.observe_workspace(&workspace);
        let report = advisor.report(None, true);
        assert!(report.candidates.iter().any(|item| {
            item.kind == "historical_shared_rust"
                && item.exists
                && !item.managed_by_platform
                && item.estimated_bytes.unwrap_or_default() > 0
        }));
        assert!(report.candidates.iter().any(|item| {
            item.kind == "repository_legacy" && item.exists && !item.managed_by_platform
        }));
        assert!(report
            .candidates
            .iter()
            .all(|item| item.automatic_action == "none"));
        let _ = std::fs::remove_dir_all(root);
    }

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos();
        std::env::temp_dir().join(format!("elon-cache-advisor-{label}-{nanos}"))
    }
}
