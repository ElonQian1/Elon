use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RegisteredProjectIdentity {
    pub(crate) kind: String,
    pub(crate) issuer: String,
    pub(crate) project_id: String,
    pub(crate) node_id: String,
    pub(crate) owner_id: String,
    pub(crate) registered_workspace: String,
    pub(crate) git_common_dir: String,
    pub(crate) git_origin: String,
}

impl RegisteredProjectIdentity {
    pub(crate) fn scope_id(&self) -> Result<String> {
        Ok(format!(
            "registered-project-workspace:{}",
            hex::encode(Sha256::digest(serde_json::to_vec(self)?))
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RegisteredRuntime {
    pub(crate) identity: RegisteredProjectIdentity,
    pub(crate) build_id: String,
    pub(crate) generation: u64,
}

pub(crate) fn bind_runtime(
    identity: RegisteredProjectIdentity,
    revision: &str,
    view: &Value,
    previous: Option<&RegisteredRuntime>,
) -> Result<RegisteredRuntime> {
    if view["connected"] != true
        || view["historyCount"] != 0
        || view["redoCount"] != 0
        || view["nodeCount"].as_u64().unwrap_or(0) == 0
    {
        bail!("RUNTIME_BINDING_REGISTRY_RUNTIME_UNVERIFIED: real connected patch-free runtime required");
    }
    let build = view["runtimeBuildId"]
        .as_str()
        .filter(|v| !v.is_empty())
        .context("RUNTIME_BINDING_REGISTRY_RUNTIME_UNVERIFIED: missing build identity")?;
    let proof = &view["sourceProof"];
    let generation = if proof.is_null() {
        previous.filter(|p| p.identity == identity && p.build_id == build && p.generation > 0)
            .map(|p| p.generation).context("RUNTIME_BINDING_REGISTRY_RUNTIME_UNVERIFIED: no matching prior verified source/build binding")?
    } else {
        if proof["runtimeBuildId"] != build
            || proof["originWorkspaceRevision"] != revision
            || !proof["sourceParityLoss"]
                .as_f64()
                .is_some_and(|v| v.is_finite() && v >= 0.0 && v <= 0.035)
        {
            bail!("RUNTIME_BINDING_REGISTRY_RUNTIME_UNVERIFIED: source/build proof differs");
        }
        proof["generation"]
            .as_u64()
            .filter(|v| *v > 0)
            .context("RUNTIME_BINDING_REGISTRY_RUNTIME_UNVERIFIED: missing installed generation")?
    };
    Ok(RegisteredRuntime {
        identity,
        build_id: build.into(),
        generation,
    })
}

pub(super) fn select(
    root: &Path,
    node: &str,
    owner: &str,
    issuer: &str,
    payload: &Value,
) -> Result<RegisteredProjectIdentity> {
    let root = root.canonicalize()?;
    let common = common_dir(&root)?;
    let origin = normalize_remote(&git(&root, &["remote", "get-url", "origin"])?)?;
    let projects = payload["projects"]
        .as_array()
        .filter(|items| items.len() <= 10_000)
        .context("RUNTIME_BINDING_REGISTRY_INVALID: missing/bounded projects array")?;
    let mut matches = Vec::new();
    for project in projects {
        if project["node_id"].as_str() != Some(node) || project["status"] == "deleted" {
            continue;
        }
        let Some(workspace) = project["workspace_path"]
            .as_str()
            .filter(|v| !v.trim().is_empty())
        else {
            continue;
        };
        let Ok(bound) = Path::new(workspace).canonicalize() else {
            continue;
        };
        // A linked worktree is accepted only when the server-bound checkout has
        // exactly the same real Git common directory. Same remote is insufficient.
        if common_dir(&bound).ok().as_ref() != Some(&common) {
            continue;
        }
        let project_id = project["id"]
            .as_str()
            .filter(|v| valid_id(v))
            .context("RUNTIME_BINDING_REGISTRY_INVALID: invalid registered project id")?;
        let registered_origin = project["repo_url"]
            .as_str()
            .context("RUNTIME_BINDING_REGISTRY_INVALID: project has no repo_url")?;
        if normalize_remote(registered_origin)? != origin
            || normalize_remote(&git(&bound, &["remote", "get-url", "origin"])?)? != origin
        {
            bail!(
                "RUNTIME_BINDING_REGISTRY_ORIGIN_CHANGED: registered repository identity differs"
            );
        }
        matches.push(RegisteredProjectIdentity {
            kind: "REGISTERED_PROJECT_WORKSPACE".into(),
            issuer: issuer.into(),
            project_id: project_id.into(),
            node_id: node.into(),
            owner_id: owner.into(),
            registered_workspace: path_identity(&bound),
            git_common_dir: path_identity(&common),
            git_origin: origin.clone(),
        });
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => bail!("RUNTIME_BINDING_MISSING_REGISTRATION: use the formal PC project workbench to bind this repository to the current account/node"),
        _ => bail!("RUNTIME_BINDING_AMBIGUOUS_ROOT: multiple registered projects claim this Git workspace"),
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}

fn common_dir(root: &Path) -> Result<PathBuf> {
    let top = PathBuf::from(git(root, &["rev-parse", "--show-toplevel"])?).canonicalize()?;
    if top != root {
        bail!("RUNTIME_BINDING_REGISTRY_INVALID: workspace must be an exact Git root");
    }
    PathBuf::from(git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?)
    .canonicalize()
    .context("Git common directory unavailable")
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let mut command = Command::new("git");
    command.current_dir(root).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command
        .output()
        .context("RUNTIME_BINDING_REGISTRY_INVALID: Git identity inspection failed")?;
    if !output.status.success() || output.stdout.len() > 16 * 1024 {
        bail!("RUNTIME_BINDING_REGISTRY_INVALID: Git identity unavailable");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn path_identity(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn normalize_remote(value: &str) -> Result<String> {
    let value = value.trim().trim_end_matches('/').trim_end_matches(".git");
    if let Some(rest) = value.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            if !host.is_empty() && !path.is_empty() && !path.contains(['?', '#', '\\']) {
                return Ok(format!(
                    "{}/{}",
                    host.to_ascii_lowercase(),
                    path.trim_matches('/')
                ));
            }
        }
    }
    let url = reqwest::Url::parse(value)
        .context("RUNTIME_BINDING_REGISTRY_INVALID: unsupported Git remote")?;
    if !matches!(url.scheme(), "http" | "https" | "ssh")
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || (!url.username().is_empty() && !(url.scheme() == "ssh" && url.username() == "git"))
    {
        bail!("RUNTIME_BINDING_REGISTRY_INVALID: unsupported Git remote identity");
    }
    let host = url.host_str().context("Git remote has no host")?;
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    Ok(format!("{host}{port}/{}", url.path().trim_matches('/')))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
