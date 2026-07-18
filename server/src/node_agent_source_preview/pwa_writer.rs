use super::{
    pwa_json::edit_token_json,
    pwa_style_syntax::{edit_css_rule, edit_style_object, valid_source_property},
    types::{
        CommitPwaStyleRequest, CommitPwaStyleResponse, PwaExplicitStyleBinding, PwaStyleBindingKind,
    },
};
use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;
const STYLE_PROPERTIES: &[&str] = &[
    "width",
    "height",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "marginTop",
    "marginRight",
    "marginBottom",
    "marginLeft",
    "borderRadius",
    "fontSize",
    "fontWeight",
    "lineHeight",
    "color",
    "backgroundColor",
    "opacity",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PwaCommitErrorKind {
    Invalid,
    Conflict,
    Internal,
}

#[derive(Debug)]
pub(crate) struct PwaCommitError {
    kind: PwaCommitErrorKind,
    source: anyhow::Error,
}

impl PwaCommitError {
    pub(crate) fn kind(&self) -> PwaCommitErrorKind {
        self.kind
    }

    pub(crate) fn into_anyhow(self) -> anyhow::Error {
        self.source
    }
}

pub(super) fn invalid(message: impl Into<String>) -> PwaCommitError {
    PwaCommitError {
        kind: PwaCommitErrorKind::Invalid,
        source: anyhow!(message.into()),
    }
}

pub(super) fn conflict(message: impl Into<String>) -> PwaCommitError {
    PwaCommitError {
        kind: PwaCommitErrorKind::Conflict,
        source: anyhow!(message.into()),
    }
}

fn internal(error: anyhow::Error) -> PwaCommitError {
    PwaCommitError {
        kind: PwaCommitErrorKind::Internal,
        source: error,
    }
}

pub(crate) fn commit_pwa_style(
    request: &CommitPwaStyleRequest,
) -> Result<CommitPwaStyleResponse, PwaCommitError> {
    validate_request(request)?;
    let (source_file, path) = resolve_source_path(request)?;
    let content = fs::read_to_string(&path)
        .with_context(|| format!("读取 PWA 源码失败: {}", path.display()))
        .map_err(internal)?;
    validate_revision(request, &content)?;
    validate_range(&content, &request.binding)?;

    let replacement = match request.binding.kind {
        PwaStyleBindingKind::CssRule => edit_css_rule(&content, &request.binding, &request.changes),
        PwaStyleBindingKind::StyleObject => {
            edit_style_object(&content, &request.binding, &request.changes)
        }
        PwaStyleBindingKind::TokenJson => {
            edit_token_json(&content, &request.binding, &request.changes)
        }
    }?;
    let mut updated = content.clone();
    updated.replace_range(
        request.binding.range.start..request.binding.range.end,
        &replacement,
    );
    if matches!(request.binding.kind, PwaStyleBindingKind::TokenJson) {
        serde_json::from_str::<serde_json::Value>(&updated)
            .map_err(|error| invalid(format!("token-json 写回会产生无效 JSON: {error}")))?;
    }

    let current = fs::read_to_string(&path)
        .with_context(|| format!("写回前复核 PWA 源码失败: {}", path.display()))
        .map_err(internal)?;
    if current != content {
        return Err(conflict("源码在写回验证期间发生变化，请重新加载后再保存"));
    }
    crate::node_agent_atomic_file::write(&path, updated.as_bytes())
        .with_context(|| format!("原子写入 PWA 源码失败: {}", path.display()))
        .map_err(internal)?;
    Ok(CommitPwaStyleResponse {
        ok: true,
        source_revision: sha256(&updated),
        changed_files: vec![source_file],
    })
}

fn validate_request(request: &CommitPwaStyleRequest) -> Result<(), PwaCommitError> {
    let binding = &request.binding;
    if binding.version != 1 {
        return Err(invalid("只支持 version=1 的显式 PWA 样式绑定"));
    }
    if !is_sha256(&request.source_revision) || !is_sha256(&binding.source_revision) {
        return Err(invalid(
            "sourceRevision 必须是 64 位 SHA-256 十六进制字符串",
        ));
    }
    if !request
        .source_revision
        .eq_ignore_ascii_case(&binding.source_revision)
    {
        return Err(conflict("顶层与 binding sourceRevision 不一致"));
    }
    if binding.target.trim() != binding.target
        || binding.target.is_empty()
        || binding.target.chars().count() > 240
        || binding.target.chars().any(char::is_control)
    {
        return Err(invalid(
            "binding target 不能为空、含控制字符或超过 240 个字符",
        ));
    }
    if binding.property_map.is_empty() || binding.property_map.len() > 32 {
        return Err(invalid("propertyMap 必须包含 1 到 32 个属性"));
    }
    for (property, source_property) in &binding.property_map {
        if !STYLE_PROPERTIES.contains(&property.as_str()) {
            return Err(invalid(format!(
                "propertyMap 包含不支持的样式属性: {property}"
            )));
        }
        if !valid_source_property(source_property) {
            return Err(invalid(format!(
                "propertyMap 源属性名无效: {source_property}"
            )));
        }
    }
    if request.changes.is_empty() || request.changes.len() > 32 {
        return Err(invalid("一次写回必须包含 1 到 32 个变更"));
    }
    let allowed = binding.property_map.values().collect::<Vec<_>>();
    for (property, value) in &request.changes {
        if !valid_source_property(property) || !allowed.iter().any(|item| *item == property) {
            return Err(invalid(format!(
                "变更属性未在 propertyMap 中显式授权: {property}"
            )));
        }
        if value.chars().count() > 1000 || value.contains('\0') {
            return Err(invalid(format!(
                "属性 {property} 的值超过 1000 字符或包含 NUL"
            )));
        }
    }
    Ok(())
}

fn resolve_source_path(
    request: &CommitPwaStyleRequest,
) -> Result<(String, PathBuf), PwaCommitError> {
    let raw_root = request.project_root.trim();
    let root_path = PathBuf::from(raw_root);
    if raw_root.is_empty() || !root_path.is_dir() {
        return Err(invalid("请选择有效的本机 PWA 项目目录"));
    }
    let root = root_path
        .canonicalize()
        .context("无法 canonicalize PWA workspace 根目录")
        .map_err(internal)?;
    let relative = request.binding.source_file.trim();
    if relative != request.binding.source_file
        || relative.is_empty()
        || relative.len() > 500
        || relative.contains('\\')
        || relative.contains('\0')
    {
        return Err(invalid("sourceFile 必须是 500 字符内的规范项目相对路径"));
    }
    let relative_path = PathBuf::from(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(invalid("sourceFile 禁止绝对路径、. 或 .. 路径段"));
    }
    validate_extension(&request.binding.kind, &relative_path)?;
    let candidate = root
        .join(&relative_path)
        .canonicalize()
        .context("PWA 绑定文件不存在或无法解析")
        .map_err(|error| invalid(format!("{error:#}")))?;
    if !candidate.starts_with(&root) {
        return Err(invalid("sourceFile 通过符号链接越出 workspace 根目录"));
    }
    let metadata = candidate
        .metadata()
        .context("无法读取 PWA 绑定文件元数据")
        .map_err(internal)?;
    if !metadata.is_file() || metadata.len() > MAX_SOURCE_BYTES {
        return Err(invalid("PWA 绑定必须是 2MB 以内的普通文件"));
    }
    Ok((relative.to_string(), candidate))
}

fn validate_extension(
    kind: &PwaStyleBindingKind,
    path: &std::path::Path,
) -> Result<(), PwaCommitError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let allowed = match kind {
        PwaStyleBindingKind::CssRule => {
            matches!(extension.as_str(), "css" | "scss" | "sass" | "less")
        }
        PwaStyleBindingKind::StyleObject => matches!(
            extension.as_str(),
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs"
        ),
        PwaStyleBindingKind::TokenJson => extension == "json",
    };
    if allowed {
        Ok(())
    } else {
        Err(invalid("binding kind 与 sourceFile 扩展名不匹配"))
    }
}

fn validate_revision(request: &CommitPwaStyleRequest, content: &str) -> Result<(), PwaCommitError> {
    if !sha256(content).eq_ignore_ascii_case(&request.source_revision) {
        return Err(conflict("sourceRevision 冲突，源码已变化"));
    }
    Ok(())
}

fn validate_range(content: &str, binding: &PwaExplicitStyleBinding) -> Result<(), PwaCommitError> {
    let range = &binding.range;
    if range.start >= range.end || range.end > content.len() {
        return Err(invalid("binding range 越出 UTF-8 源码范围"));
    }
    if !content.is_char_boundary(range.start) || !content.is_char_boundary(range.end) {
        return Err(invalid("binding range 必须位于 UTF-8 字符边界"));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}
