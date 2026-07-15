use anyhow::{bail, Context, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use std::path::{Path, PathBuf};

use super::{
    path_text, paths_overlap, validate_and_prepare, validate_no_canonical_root_overlap,
    validate_no_root_overlap, NodeDataRootSource, NodeDataRootState,
};

const AUTOMATIC_ROOT_NAME: &str = "ElonNodeData";

/// Automatically prepare a managed data root for an upgraded node.
///
/// Existing external projects stay where the user put them. The preferred
/// candidate is a sibling of that project, which keeps Git worktrees and build
/// caches on the same writable volume without adopting or moving the project.
pub(crate) fn prepare_automatic_root(
    current: &NodeDataRootState,
    workspace_hint: Option<&Path>,
    fallback_parent: Option<&Path>,
    install_id: &str,
) -> Result<NodeDataPaths> {
    if let Some(paths) = current.paths.as_ref() {
        return Ok(paths.clone());
    }
    if current.source != NodeDataRootSource::Unconfigured || current.invalid_reason.is_some() {
        bail!(
            "已有节点数据目录配置未通过安全校验，客户端不会擅自覆盖；{}",
            current
                .invalid_reason
                .as_deref()
                .unwrap_or("请修复已有配置后重试")
        );
    }

    let candidates =
        automatic_root_candidates(current, workspace_hint, fallback_parent, install_id);
    if candidates.is_empty() {
        bail!("找不到可写的项目同盘位置，无法自动准备 AI 临时工作区");
    }

    let mut failures = Vec::new();
    for candidate in candidates {
        if workspace_hint.is_some_and(|workspace| paths_overlap(&candidate, workspace)) {
            failures.push(format!("{} 与项目目录重叠", candidate.display()));
            continue;
        }
        if let Err(error) =
            validate_no_root_overlap(candidate.to_string_lossy().as_ref(), current, install_id)
        {
            failures.push(format!("{}: {error}", candidate.display()));
            continue;
        }
        let paths = match validate_and_prepare(candidate.to_string_lossy().as_ref(), install_id) {
            Ok(paths) => paths,
            Err(error) => {
                failures.push(format!("{}: {error}", candidate.display()));
                continue;
            }
        };
        if let Err(error) = validate_no_canonical_root_overlap(paths.root(), current, install_id) {
            failures.push(format!("{}: {error}", candidate.display()));
            continue;
        }
        return Ok(paths);
    }

    let detail = failures
        .last()
        .map(String::as_str)
        .unwrap_or("候选目录均不可用");
    bail!("客户端未能自动准备 AI 临时工作区，原项目没有被移动或删除。最后一次检查：{detail}")
}

fn automatic_root_candidates(
    current: &NodeDataRootState,
    workspace_hint: Option<&Path>,
    fallback_parent: Option<&Path>,
    install_id: &str,
) -> Vec<PathBuf> {
    let mut parents = Vec::new();
    push_parent(&mut parents, workspace_hint.and_then(Path::parent));
    if workspace_hint.is_none() {
        for drive in fixed_drive_roots_by_free_space() {
            push_parent(&mut parents, Some(&drive));
        }
    }
    push_parent(
        &mut parents,
        current
            .legacy_workspace_root
            .as_deref()
            .and_then(Path::parent),
    );
    push_parent(
        &mut parents,
        current
            .legacy_storage_root
            .as_deref()
            .and_then(Path::parent),
    );
    push_parent(&mut parents, fallback_parent);

    let suffix = safe_install_suffix(install_id);
    let mut candidates = Vec::new();
    for parent in parents {
        let base = parent.join(AUTOMATIC_ROOT_NAME);
        push_unique(&mut candidates, base.clone());
        push_unique(
            &mut candidates,
            parent.join(format!("{AUTOMATIC_ROOT_NAME}-{suffix}")),
        );
    }
    candidates
}

fn push_parent(parents: &mut Vec<PathBuf>, parent: Option<&Path>) {
    let Some(parent) = parent.filter(|path| path.is_absolute()) else {
        return;
    };
    push_unique(parents, parent.to_path_buf());
}

fn push_unique(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    let key = normalized_candidate_key(&candidate);
    if !paths
        .iter()
        .any(|existing| normalized_candidate_key(existing) == key)
    {
        paths.push(candidate);
    }
}

fn normalized_candidate_key(path: &Path) -> String {
    let text = path_text(path);
    if cfg!(windows) {
        text.to_ascii_lowercase()
    } else {
        text
    }
}

fn safe_install_suffix(install_id: &str) -> String {
    let suffix = install_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(10)
        .collect::<String>();
    if suffix.is_empty() {
        "local".to_string()
    } else {
        suffix
    }
}

#[cfg(windows)]
fn fixed_drive_roots_by_free_space() -> Vec<PathBuf> {
    use std::os::windows::ffi::OsStrExt;

    const DRIVE_FIXED: u32 = 3;
    let drive_mask = unsafe { GetLogicalDrives() };
    let system_drive = std::env::var("SystemDrive").ok().map(|value| {
        value
            .trim()
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase()
    });
    let mut drives = Vec::new();
    for index in 0..26_u32 {
        if drive_mask & (1_u32 << index) == 0 {
            continue;
        }
        let letter = (b'A' + index as u8) as char;
        let root = PathBuf::from(format!("{letter}:\\"));
        let wide = root
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        if unsafe { GetDriveTypeW(wide.as_ptr()) } != DRIVE_FIXED {
            continue;
        }
        let mut free = 0_u64;
        let readable = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } != 0;
        if !readable {
            continue;
        }
        let key = format!("{letter}:").to_ascii_lowercase();
        let is_system = system_drive.as_deref() == Some(key.as_str());
        drives.push((is_system, std::cmp::Reverse(free), root));
    }
    drives.sort_by(|left, right| (left.0, left.1).cmp(&(right.0, right.1)));
    drives.into_iter().map(|(_, _, root)| root).collect()
}

#[cfg(not(windows))]
fn fixed_drive_roots_by_free_space() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLogicalDrives() -> u32;
    fn GetDriveTypeW(root_path_name: *const u16) -> u32;
    fn GetDiskFreeSpaceExW(
        directory_name: *const u16,
        free_bytes_available: *mut u64,
        total_number_of_bytes: *mut u64,
        total_number_of_free_bytes: *mut u64,
    ) -> i32;
}

pub(crate) fn automatic_fallback_parent(state_path: &Path) -> Result<PathBuf> {
    state_path
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .with_context(|| format!("无法从节点状态路径推导安全目录: {}", state_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_data_root::{resolve_from_values, verify_root_marker};

    #[test]
    fn automatic_root_is_project_sibling_and_keeps_project_untouched() {
        let sandbox = std::env::temp_dir().join(format!("elon-auto-root-{}", uuid::Uuid::new_v4()));
        let project = sandbox.join("projects").join("existing-project");
        std::fs::create_dir_all(&project).expect("create project");
        std::fs::write(project.join("keep.txt"), "user data").expect("seed project");
        let state = resolve_from_values(None, None, None, None);

        let paths = prepare_automatic_root(&state, Some(&project), Some(&sandbox), "ins_auto_test")
            .expect("automatic root");

        assert_eq!(
            paths.root(),
            sandbox.join("projects").join(AUTOMATIC_ROOT_NAME)
        );
        assert_eq!(
            std::fs::read_to_string(project.join("keep.txt")).expect("project remains"),
            "user data"
        );
        verify_root_marker(&paths, "ins_auto_test").expect("owned marker");
        let _ = std::fs::remove_dir_all(sandbox);
    }

    #[test]
    fn automatic_root_uses_install_scoped_fallback_when_base_is_occupied() {
        let sandbox =
            std::env::temp_dir().join(format!("elon-auto-root-collision-{}", uuid::Uuid::new_v4()));
        let project = sandbox.join("project");
        let occupied = sandbox.join(AUTOMATIC_ROOT_NAME);
        std::fs::create_dir_all(&project).expect("create project");
        std::fs::create_dir_all(&occupied).expect("create occupied root");
        std::fs::write(occupied.join("foreign.txt"), "foreign").expect("seed occupied root");
        let state = resolve_from_values(None, None, None, None);

        let paths = prepare_automatic_root(&state, Some(&project), Some(&sandbox), "ins_collision")
            .expect("automatic collision fallback");

        assert_ne!(paths.root(), occupied);
        assert!(paths.root().ends_with("ElonNodeData-inscollisi"));
        assert_eq!(
            std::fs::read_to_string(occupied.join("foreign.txt")).expect("foreign remains"),
            "foreign"
        );
        let _ = std::fs::remove_dir_all(sandbox);
    }
}
