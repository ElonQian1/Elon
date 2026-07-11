use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;

const MAX_STYLE_JSON_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_TOKEN_FILE: &str = ".elon/ui-standards/tokens.json";

#[derive(Debug, Clone)]
pub(crate) struct JsonSourceBinding {
    pub(crate) file: PathBuf,
    pub(crate) relative_file: String,
    pub(crate) pointer: String,
    pub(crate) source_key: String,
    pub(crate) old_value: Value,
    pub(crate) kind: String,
}

pub(crate) fn resolve_json_binding(
    root: &Path,
    binding: &Value,
) -> Result<Option<JsonSourceBinding>> {
    let kind = binding
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_uppercase();
    let (relative_file, pointer, source_key) = match kind.as_str() {
        "STYLE_JSON" => {
            let relative_file = required_string(binding, "relativeFile")?;
            let pointer = required_string(binding, "jsonPointer")?;
            if !pointer.starts_with('/') {
                bail!("Style JSON jsonPointer 必须以 / 开头");
            }
            (relative_file, pointer.clone(), format!("json:{pointer}"))
        }
        "TOKEN" => {
            let token_path = required_string(binding, "path")?;
            let pointer = token_pointer(&token_path)?;
            (
                DEFAULT_TOKEN_FILE.to_string(),
                pointer,
                format!("token:{token_path}"),
            )
        }
        _ => return Ok(None),
    };
    let file = safe_project_file(root, &relative_file)?;
    let content = read_json(&file)?;
    let document: Value = serde_json::from_str(&content)
        .with_context(|| format!("解析样式 JSON 失败: {}", file.display()))?;
    let old_value = document
        .pointer(&pointer)
        .cloned()
        .ok_or_else(|| anyhow!("样式 JSON 中不存在路径 {pointer}"))?;
    Ok(Some(JsonSourceBinding {
        file,
        relative_file: normalize_relative(&relative_file),
        pointer,
        source_key,
        old_value,
        kind,
    }))
}

pub(crate) fn replace_json_pointer(content: &str, pointer: &str, value: &Value) -> Result<String> {
    let mut document: Value = serde_json::from_str(content).context("解析样式 JSON 失败")?;
    let target = document
        .pointer_mut(pointer)
        .ok_or_else(|| anyhow!("样式 JSON 中不存在路径 {pointer}"))?;
    *target = value.clone();
    let mut output = serde_json::to_string_pretty(&document).context("序列化样式 JSON 失败")?;
    output.push('\n');
    Ok(output)
}

pub(crate) fn read_json(path: &Path) -> Result<String> {
    let metadata =
        fs::metadata(path).with_context(|| format!("样式 JSON 不存在: {}", path.display()))?;
    if metadata.len() > MAX_STYLE_JSON_BYTES {
        bail!("样式 JSON 超过 2 MiB: {}", path.display());
    }
    fs::read_to_string(path).with_context(|| format!("读取样式 JSON 失败: {}", path.display()))
}

pub(crate) fn write_json(root: &Path, path: &Path, content: &str) -> Result<()> {
    let canonical_parent = path
        .parent()
        .ok_or_else(|| anyhow!("样式 JSON 路径没有父目录"))?
        .canonicalize()
        .with_context(|| format!("样式 JSON 父目录不存在: {}", path.display()))?;
    if !canonical_parent.starts_with(root) {
        bail!("拒绝写入项目目录之外的文件: {}", path.display());
    }
    fs::write(path, content).with_context(|| format!("写入样式 JSON 失败: {}", path.display()))
}

fn safe_project_file(root: &Path, relative_file: &str) -> Result<PathBuf> {
    let relative = Path::new(relative_file);
    if relative.as_os_str().is_empty()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("样式 JSON 必须是项目内相对路径: {relative_file}");
    }
    let file = root.join(relative);
    let canonical = file
        .canonicalize()
        .with_context(|| format!("样式 JSON 不存在: {}", file.display()))?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        bail!("拒绝读取项目目录之外的样式 JSON: {}", file.display());
    }
    Ok(canonical)
}

fn token_pointer(path: &str) -> Result<String> {
    let segments = path
        .split('.')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        bail!("Token path 不能为空");
    }
    Ok(format!(
        "/{}",
        segments
            .into_iter()
            .map(|segment| segment.replace('~', "~0").replace('/', "~1"))
            .collect::<Vec<_>>()
            .join("/")
    ))
}

fn required_string(binding: &Value, key: &str) -> Result<String> {
    binding
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("样式绑定缺少 {key}"))
}

fn normalize_relative(value: &str) -> String {
    value.replace('\\', "/")
}
