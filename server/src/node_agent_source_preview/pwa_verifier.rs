use anyhow::{anyhow, bail, Context, Result};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant, SystemTime},
};
use tokio::{process::Command, time::timeout};

const MAX_CHANGED_FILES: usize = 64;
const MAX_EXPECTED_VALUES: usize = 64;
const MAX_RESOURCE_FILES: usize = 5_000;
const MAX_RESOURCE_BYTES: u64 = 32 * 1024 * 1024;
const BUILD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DEFAULT_OUTPUT_DIRS: &[&str] = &["dist", "build", "out", ".next", "public/build"];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PwaSourceRevision {
    pub(crate) source_file: String,
    pub(crate) source_revision: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VerifyPwaSourceRequest {
    pub(crate) project_root: String,
    pub(crate) changed_files: Vec<PwaSourceRevision>,
    pub(crate) expected_values: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PwaSourceBuildVerification {
    pub(crate) ok: bool,
    pub(crate) status: &'static str,
    pub(crate) message: String,
    pub(crate) source_revisions: BTreeMap<String, String>,
    pub(crate) changed_files: Vec<String>,
    pub(crate) build_command: Option<String>,
    pub(crate) build_duration_ms: u128,
    pub(crate) resource_files: Vec<String>,
    pub(crate) resource_values_verified: usize,
    pub(crate) output_tail: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewConfig {
    build: Option<PreviewBuildConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewBuildConfig {
    module: Option<String>,
    script: Option<String>,
    output_dirs: Option<Vec<String>>,
}

struct VerifiedSources {
    root: PathBuf,
    files: Vec<(String, PathBuf)>,
    revisions: BTreeMap<String, String>,
}

struct BuildPlan {
    module: PathBuf,
    manager: &'static str,
    script: String,
    output_dirs: Vec<String>,
}

pub(crate) async fn verify_pwa_source(
    request: VerifyPwaSourceRequest,
) -> Result<PwaSourceBuildVerification> {
    let sources = match verify_sources(&request) {
        Ok(value) => value,
        Err(error) => return Ok(failed(&request, error, None, 0, String::new())),
    };
    let plan = match build_plan(&sources) {
        Ok(value) => value,
        Err(error) => return Ok(failed(&request, error, None, 0, String::new())),
    };
    let command_label = format!("{} run {}", plan.manager, plan.script);
    let started_at = SystemTime::now();
    let started = Instant::now();
    let output = match run_build(&plan).await {
        Ok(value) => value,
        Err(error) => {
            return Ok(failed(
                &request,
                error,
                Some(command_label),
                started.elapsed().as_millis(),
                String::new(),
            ));
        }
    };
    let duration = started.elapsed().as_millis();
    let output_tail = bounded_output(&output.stdout, &output.stderr);
    if !output.status.success() {
        return Ok(failed(
            &request,
            anyhow!("PWA 前端构建失败"),
            Some(command_label),
            duration,
            output_tail,
        ));
    }
    let (resource_files, verified_values) =
        match verify_resources(&sources.root, &plan, &request.expected_values, started_at) {
            Ok(value) => value,
            Err(error) => {
                return Ok(failed(
                    &request,
                    error,
                    Some(command_label),
                    duration,
                    output_tail,
                ));
            }
        };
    Ok(PwaSourceBuildVerification {
        ok: true,
        status: "BUILD_VERIFIED",
        message: format!(
            "源码哈希、前端构建和 {} 个目标资源值均已验证",
            verified_values
        ),
        source_revisions: sources.revisions,
        changed_files: sources.files.into_iter().map(|(file, _)| file).collect(),
        build_command: Some(command_label),
        build_duration_ms: duration,
        resource_files,
        resource_values_verified: verified_values,
        output_tail,
    })
}

fn verify_sources(request: &VerifyPwaSourceRequest) -> Result<VerifiedSources> {
    let raw_root = request.project_root.trim();
    if raw_root.is_empty() || !Path::new(raw_root).is_dir() {
        bail!("请选择有效的本机 PWA 项目目录");
    }
    if request.changed_files.is_empty() || request.changed_files.len() > MAX_CHANGED_FILES {
        bail!("changedFiles 必须包含 1 到 {MAX_CHANGED_FILES} 个源码文件");
    }
    if request.expected_values.is_empty() || request.expected_values.len() > MAX_EXPECTED_VALUES {
        bail!("expectedValues 必须包含 1 到 {MAX_EXPECTED_VALUES} 个目标值");
    }
    if request
        .expected_values
        .iter()
        .any(|value| value.is_empty() || value.len() > 1_000 || value.contains('\0'))
    {
        bail!("expectedValues 包含空值、超长值或 NUL");
    }
    let root = Path::new(raw_root)
        .canonicalize()
        .context("无法解析 PWA workspace 根目录")?;
    let mut files = Vec::new();
    let mut revisions = BTreeMap::new();
    for entry in &request.changed_files {
        if !is_sha256(&entry.source_revision) {
            bail!("{} 的 sourceRevision 不是 SHA-256", entry.source_file);
        }
        let path = canonical_child(&root, &entry.source_file)?;
        let content =
            fs::read(&path).with_context(|| format!("读取 PWA 源码失败: {}", entry.source_file))?;
        let actual = hex::encode(Sha256::digest(&content));
        if !actual.eq_ignore_ascii_case(&entry.source_revision) {
            bail!("{} 的 sourceRevision 已变化", entry.source_file);
        }
        if revisions
            .insert(entry.source_file.clone(), actual)
            .is_some()
        {
            bail!("changedFiles 不得重复: {}", entry.source_file);
        }
        files.push((entry.source_file.clone(), path));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(VerifiedSources {
        root,
        files,
        revisions,
    })
}

fn build_plan(sources: &VerifiedSources) -> Result<BuildPlan> {
    let config = read_config(&sources.root);
    let configured = config.build.unwrap_or_default();
    let module = if let Some(module) = configured.module.as_deref() {
        let module = safe_relative(module)?;
        let candidate = sources.root.join(module);
        if !candidate.join("package.json").is_file() {
            bail!("配置的 PWA build module 不包含 package.json");
        }
        candidate
            .canonicalize()
            .context("无法解析 PWA build module")?
    } else {
        common_package_module(&sources.root, &sources.files)?
    };
    let package = fs::read_to_string(module.join("package.json"))
        .context("无法读取 PWA build package.json")?;
    let package: serde_json::Value =
        serde_json::from_str(&package).context("PWA build package.json 无效")?;
    let scripts = package
        .get("scripts")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow!("PWA build package.json 没有 scripts"))?;
    let script = configured
        .script
        .filter(|value| valid_script(value))
        .or_else(|| {
            ["build", "compile", "dist"]
                .into_iter()
                .find(|name| scripts.contains_key(*name))
                .map(str::to_string)
        })
        .ok_or_else(|| anyhow!("PWA 项目没有可验证的 build/compile/dist 脚本"))?;
    if !scripts.contains_key(&script) {
        bail!("配置的 PWA build script 不存在: {script}");
    }
    let output_dirs = configured
        .output_dirs
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| {
            DEFAULT_OUTPUT_DIRS
                .iter()
                .map(|value| value.to_string())
                .collect()
        });
    if output_dirs.len() > 12
        || output_dirs
            .iter()
            .any(|value| safe_relative(value).is_err())
    {
        bail!("PWA outputDirs 必须是 1 到 12 个安全相对目录");
    }
    Ok(BuildPlan {
        manager: package_manager(&module),
        module,
        script,
        output_dirs,
    })
}

async fn run_build(plan: &BuildPlan) -> Result<std::process::Output> {
    let program = if cfg!(windows) {
        format!("{}.cmd", plan.manager)
    } else {
        plan.manager.to_string()
    };
    let mut command = Command::new(program);
    command
        .current_dir(&plan.module)
        .args(["run", plan.script.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    timeout(BUILD_TIMEOUT, command.output())
        .await
        .map_err(|_| anyhow!("PWA 前端构建超时（10 分钟）"))?
        .context("无法启动 PWA 前端构建")
}

fn verify_resources(
    root: &Path,
    plan: &BuildPlan,
    expected_values: &[String],
    started_at: SystemTime,
) -> Result<(Vec<String>, usize)> {
    let threshold = started_at
        .checked_sub(Duration::from_secs(2))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut artifacts = Vec::new();
    let mut matched = vec![false; expected_values.len()];
    let compact_expected = expected_values
        .iter()
        .map(|value| compact(value))
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    let mut bytes = 0u64;
    for relative in &plan.output_dirs {
        let directory = plan.module.join(safe_relative(relative)?);
        if !directory.is_dir() {
            continue;
        }
        for entry in WalkBuilder::new(&directory)
            .hidden(false)
            .git_ignore(false)
            .build()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(value) if value.is_file() => value,
                _ => continue,
            };
            if !resource_extension(path)
                || metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH) < threshold
            {
                continue;
            }
            visited += 1;
            bytes = bytes.saturating_add(metadata.len());
            if visited > MAX_RESOURCE_FILES || bytes > MAX_RESOURCE_BYTES {
                bail!("PWA 构建资源超过验证上限");
            }
            let content = match fs::read_to_string(path) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let compact_content = compact(&content);
            for (index, value) in expected_values.iter().enumerate() {
                if !matched[index]
                    && (content.contains(value)
                        || compact_content.contains(&compact_expected[index]))
                {
                    matched[index] = true;
                }
            }
            if let Ok(relative) = path.strip_prefix(root) {
                artifacts.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    if artifacts.is_empty() {
        bail!("构建成功，但没有找到本轮生成的 PWA 文本资源");
    }
    let missing = matched.iter().filter(|value| !**value).count();
    if missing > 0 {
        bail!("构建资源缺少 {missing} 个目标样式值");
    }
    artifacts.sort();
    artifacts.dedup();
    artifacts.truncate(80);
    Ok((artifacts, matched.len()))
}

fn common_package_module(root: &Path, files: &[(String, PathBuf)]) -> Result<PathBuf> {
    let mut modules = BTreeSet::new();
    for (_, file) in files {
        let mut current = file.parent();
        let mut found = None;
        while let Some(directory) = current {
            if !directory.starts_with(root) {
                break;
            }
            if directory.join("package.json").is_file() {
                found = Some(directory.to_path_buf());
                break;
            }
            if directory == root {
                break;
            }
            current = directory.parent();
        }
        modules
            .insert(found.ok_or_else(|| anyhow!("变更源码不属于可构建的 PWA package.json 模块"))?);
    }
    if modules.len() != 1 {
        bail!("一次 PWA 验证只允许一个前端构建模块");
    }
    Ok(modules.into_iter().next().expect("one module"))
}

fn read_config(root: &Path) -> PreviewConfig {
    fs::read_to_string(root.join(".elon/ui-pwa-preview.json"))
        .ok()
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

fn canonical_child(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative_path = safe_relative(relative)?;
    let path = root
        .join(relative_path)
        .canonicalize()
        .with_context(|| format!("源码文件不存在或无法解析: {relative}"))?;
    if !path.starts_with(root) || !path.is_file() {
        bail!("源码文件通过符号链接越出 workspace 或不是普通文件: {relative}");
    }
    Ok(path)
}

fn safe_relative(value: &str) -> Result<PathBuf> {
    if value.is_empty() || value.len() > 500 || value.contains(['\\', '\0']) {
        bail!("路径必须是 500 字符内的规范项目相对路径");
    }
    let path = PathBuf::from(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("路径禁止绝对路径、. 或 .. 路径段");
    }
    Ok(path)
}

fn package_manager(module: &Path) -> &'static str {
    if module.join("bun.lockb").is_file() || module.join("bun.lock").is_file() {
        "bun"
    } else if module.join("pnpm-lock.yaml").is_file() {
        "pnpm"
    } else if module.join("yarn.lock").is_file() {
        "yarn"
    } else {
        "npm"
    }
}

fn valid_script(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
}

fn resource_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "css" | "js" | "mjs" | "cjs" | "html" | "json"
    )
}

fn compact(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn bounded_output(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    let mut characters = combined.chars().collect::<Vec<_>>();
    if characters.len() > 4_000 {
        characters = characters.split_off(characters.len() - 4_000);
    }
    characters
        .into_iter()
        .collect::<String>()
        .trim()
        .to_string()
}

fn failed(
    request: &VerifyPwaSourceRequest,
    error: anyhow::Error,
    command: Option<String>,
    duration: u128,
    output_tail: String,
) -> PwaSourceBuildVerification {
    PwaSourceBuildVerification {
        ok: false,
        status: "VERIFY_FAILED",
        message: format!("{error:#}"),
        source_revisions: BTreeMap::new(),
        changed_files: request
            .changed_files
            .iter()
            .map(|entry| entry.source_file.clone())
            .collect(),
        build_command: command,
        build_duration_ms: duration,
        resource_files: Vec::new(),
        resource_values_verified: 0,
        output_tail,
    }
}

#[cfg(test)]
pub(super) fn verify_sources_for_test(request: &VerifyPwaSourceRequest) -> Result<()> {
    verify_sources(request).map(|_| ())
}

#[cfg(test)]
pub(super) fn verify_resources_for_test(
    root: &Path,
    module: &Path,
    output_dirs: Vec<String>,
    expected: &[String],
) -> Result<(Vec<String>, usize)> {
    let plan = BuildPlan {
        module: module.to_path_buf(),
        manager: "npm",
        script: "build".to_string(),
        output_dirs,
    };
    verify_resources(root, &plan, expected, SystemTime::UNIX_EPOCH)
}
