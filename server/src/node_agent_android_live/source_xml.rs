use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};

const MAX_XML_FILES: usize = 3_000;
const MAX_XML_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct LayoutElement {
    pub(crate) file: PathBuf,
    pub(crate) relative_file: String,
    pub(crate) resource_name: String,
    pub(crate) tag_name: String,
    pub(crate) tag_start: usize,
    pub(crate) tag_end: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ValueResource {
    pub(crate) file: PathBuf,
    pub(crate) relative_file: String,
    pub(crate) resource_type: String,
    pub(crate) resource_name: String,
    pub(crate) value_start: usize,
    pub(crate) value_end: usize,
}

pub(crate) fn canonical_project_root(value: Option<&str>) -> Result<PathBuf> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    let root = value.ok_or_else(|| anyhow!("Live UI 会话没有绑定项目目录"))?;
    let root = PathBuf::from(root)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {root}"))?;
    if !root.is_dir() {
        bail!("项目目录不是文件夹: {}", root.display());
    }
    Ok(root)
}

pub(crate) fn find_layout_element(root: &Path, resource_id: &str) -> Result<Option<LayoutElement>> {
    let resource_name = resource_id_name(resource_id)
        .ok_or_else(|| anyhow!("无法解析 resource-id: {resource_id}"))?;
    for file in android_xml_files(root) {
        if !is_layout_file(&file) {
            continue;
        }
        let Some(content) = read_small_xml(&file) else {
            continue;
        };
        for marker in [
            format!("@+id/{resource_name}"),
            format!("@id/{resource_name}"),
        ] {
            let Some(marker_index) = content.find(&marker) else {
                continue;
            };
            let Some(tag_start) = content[..marker_index].rfind('<') else {
                continue;
            };
            let Some(relative_end) = content[marker_index..].find('>') else {
                continue;
            };
            let tag_end = marker_index + relative_end;
            let tag_name = parse_tag_name(&content[tag_start..=tag_end]);
            if tag_name.is_empty() || tag_name.starts_with('!') {
                continue;
            }
            return Ok(Some(LayoutElement {
                relative_file: relative_path(root, &file),
                file,
                resource_name: resource_name.to_string(),
                tag_name,
                tag_start,
                tag_end,
            }));
        }
    }
    Ok(None)
}

pub(crate) fn element_attribute(
    content: &str,
    element: &LayoutElement,
    name: &str,
) -> Option<String> {
    let tag = content.get(element.tag_start..=element.tag_end)?;
    attribute_value(tag, name).map(str::to_string)
}

pub(crate) fn find_value_resource(root: &Path, reference: &str) -> Result<Option<ValueResource>> {
    let Some((resource_type, resource_name)) = parse_resource_reference(reference) else {
        return Ok(None);
    };
    for file in android_xml_files(root) {
        if !is_values_file(&file) {
            continue;
        }
        let Some(content) = read_small_xml(&file) else {
            continue;
        };
        let Some(resource) = locate_value_resource(&content, resource_type, resource_name) else {
            continue;
        };
        return Ok(Some(ValueResource {
            relative_file: relative_path(root, &file),
            file,
            resource_type: resource_type.to_string(),
            resource_name: resource_name.to_string(),
            value_start: resource.0,
            value_end: resource.1,
        }));
    }
    Ok(None)
}

pub(crate) fn replace_element_attribute(
    content: &str,
    element: &LayoutElement,
    name: &str,
    value: &str,
) -> Result<String> {
    let current_range = locate_layout_element(content, &element.resource_name)
        .ok_or_else(|| anyhow!("XML 中已找不到节点 {}", element.resource_name))?;
    let tag = content
        .get(current_range.0..=current_range.1)
        .ok_or_else(|| anyhow!("XML 元素范围失效"))?;
    let escaped = escape_xml_attribute(value);
    if let Some((start, end)) = attribute_value_range(tag, name) {
        let absolute_start = current_range.0 + start;
        let absolute_end = current_range.0 + end;
        return Ok(format!(
            "{}{}{}",
            &content[..absolute_start],
            escaped,
            &content[absolute_end..]
        ));
    }
    let insert_at = if tag.ends_with("/>") {
        current_range.1.saturating_sub(1)
    } else {
        current_range.1
    };
    let indent = line_indent(content, current_range.0);
    Ok(format!(
        "{}\n{}    {}=\"{}\"{}",
        &content[..insert_at],
        indent,
        name,
        escaped,
        &content[insert_at..]
    ))
}

pub(crate) fn replace_value_resource(
    content: &str,
    resource: &ValueResource,
    value: &str,
) -> Result<String> {
    let (value_start, value_end) =
        locate_value_resource(content, &resource.resource_type, &resource.resource_name)
            .ok_or_else(|| anyhow!("values XML 中已找不到资源 {}", resource.resource_name))?;
    Ok(format!(
        "{}{}{}",
        &content[..value_start],
        escape_xml_text(value),
        &content[value_end..]
    ))
}

pub(crate) fn read_xml(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("读取 XML 失败: {}", path.display()))
}

pub(crate) fn write_xml(root: &Path, path: &Path, content: &str) -> Result<()> {
    let canonical_parent = path
        .parent()
        .ok_or_else(|| anyhow!("XML 路径没有父目录"))?
        .canonicalize()
        .with_context(|| format!("XML 父目录不存在: {}", path.display()))?;
    if !canonical_parent.starts_with(root) {
        bail!("拒绝写入项目目录之外的文件: {}", path.display());
    }
    fs::write(path, content).with_context(|| format!("写入 XML 失败: {}", path.display()))
}

pub(crate) fn source_revision(
    root: &Path,
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<String> {
    let paths = paths.into_iter().collect::<BTreeSet<_>>();
    let mut hasher = Sha256::new();
    for path in &paths {
        let bytes =
            fs::read(path).with_context(|| format!("读取源码版本失败: {}", path.display()))?;
        hasher.update(relative_path(root, path).as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
        hasher.update([0xff]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

pub(crate) fn reference_impact_count(root: &Path, reference: &str) -> usize {
    android_xml_files(root)
        .into_iter()
        .filter_map(|path| read_small_xml(&path))
        .map(|content| content.matches(reference).count())
        .sum()
}

fn android_xml_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .build();
    for entry in walker.flatten() {
        if files.len() >= MAX_XML_FILES {
            break;
        }
        let path = entry.path();
        if !entry.file_type().is_some_and(|kind| kind.is_file())
            || path.extension().and_then(|value| value.to_str()) != Some("xml")
            || normalized(path).contains("/build/")
        {
            continue;
        }
        files.push(path.to_path_buf());
    }
    files.sort();
    files
}

fn read_small_xml(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_XML_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn is_layout_file(path: &Path) -> bool {
    let value = normalized(path);
    value.contains("/res/layout") && value.ends_with(".xml")
}

fn is_values_file(path: &Path) -> bool {
    let value = normalized(path);
    value.contains("/res/values") && value.ends_with(".xml")
}

fn normalized(path: &Path) -> String {
    format!("/{}", path.to_string_lossy().replace('\\', "/"))
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn resource_id_name(value: &str) -> Option<&str> {
    value.rsplit('/').next().filter(|value| !value.is_empty())
}

fn parse_resource_reference(value: &str) -> Option<(&str, &str)> {
    let value = value.strip_prefix('@')?;
    let (resource_type, resource_name) = value.split_once('/')?;
    if matches!(resource_type, "color" | "dimen" | "string") && !resource_name.is_empty() {
        Some((resource_type, resource_name))
    } else {
        None
    }
}

fn parse_tag_name(tag: &str) -> String {
    tag.trim_start_matches('<')
        .trim_start()
        .split(|ch: char| ch.is_whitespace() || ch == '>' || ch == '/')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn attribute_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let (start, end) = attribute_value_range(tag, name)?;
    tag.get(start..end)
}

fn attribute_value_range(tag: &str, name: &str) -> Option<(usize, usize)> {
    for quote in ['"', '\''] {
        let marker = format!("{name}={quote}");
        if let Some(marker_start) = tag.find(&marker) {
            let start = marker_start + marker.len();
            let end = tag[start..].find(quote)? + start;
            return Some((start, end));
        }
    }
    None
}

fn locate_value_resource(content: &str, kind: &str, name: &str) -> Option<(usize, usize)> {
    for quote in ['"', '\''] {
        let marker = format!("<{kind} name={quote}{name}{quote}");
        if let Some(tag_start) = content.find(&marker) {
            let value_start = content[tag_start..].find('>')? + tag_start + 1;
            let close = format!("</{kind}>");
            let value_end = content[value_start..].find(&close)? + value_start;
            return Some((value_start, value_end));
        }
    }
    None
}

fn locate_layout_element(content: &str, resource_name: &str) -> Option<(usize, usize)> {
    for marker in [
        format!("@+id/{resource_name}"),
        format!("@id/{resource_name}"),
    ] {
        if let Some(marker_index) = content.find(&marker) {
            let tag_start = content[..marker_index].rfind('<')?;
            let tag_end = content[marker_index..].find('>')? + marker_index;
            return Some((tag_start, tag_end));
        }
    }
    None
}

fn line_indent(content: &str, index: usize) -> String {
    let line_start = content[..index].rfind('\n').map_or(0, |value| value + 1);
    content[line_start..index]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect()
}

fn escape_xml_attribute(value: &str) -> String {
    escape_xml_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
