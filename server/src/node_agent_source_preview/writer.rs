use super::parser::{canonical_project_root, safe_join};
use super::types::CommitPreviewRequest;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;

pub(crate) fn commit_changes(request: &CommitPreviewRequest) -> Result<String> {
    if request.changes.is_empty() || request.changes.len() > 32 {
        bail!("一次写回必须包含 1 到 32 个受支持属性");
    }
    if request.changes.values().any(|value| value.len() > 1000) {
        bail!("单个属性值不能超过 1000 个字符");
    }
    let root = canonical_project_root(&request.project_root)?;
    let path = safe_join(&root, &request.layout_file)?;
    let mut content = fs::read_to_string(&path)?;
    let revision = hex::encode(Sha256::digest(content.as_bytes()));
    if revision != request.source_revision {
        bail!("源码已在设计会话期间变化，请重新加载后再保存");
    }
    if request.start_tag_start >= request.start_tag_end || request.start_tag_end > content.len() {
        bail!("节点源码位置已失效");
    }
    let current = &content[request.start_tag_start..request.start_tag_end];
    if !current.trim_start().starts_with('<') {
        bail!("节点源码锚点无效");
    }
    let mut updated = current.to_string();
    let mut changes = request.changes.clone();
    if let Some(radius) = changes.remove("borderRadius") {
        let background = changes
            .remove("background")
            .or_else(|| current_attribute(current, "android:background"))
            .unwrap_or_else(|| "#FFFFFFFF".to_string());
        let resource_name = generated_drawable_name(&request.node_key);
        write_shape_drawable(&path, &resource_name, &background, &radius)?;
        updated = replace_attribute(
            &updated,
            "android:background",
            &format!("@drawable/{resource_name}"),
        );
    }
    for (property, value) in &changes {
        let attribute = property_attribute(property).context("属性不支持确定性写回")?;
        updated = replace_attribute(&updated, attribute, value);
    }
    content.replace_range(request.start_tag_start..request.start_tag_end, &updated);
    fs::write(&path, &content).with_context(|| format!("写入布局失败: {}", path.display()))?;
    Ok(hex::encode(Sha256::digest(content.as_bytes())))
}

fn generated_drawable_name(node_key: &str) -> String {
    let safe = node_key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("yilong_ui_{}", safe.trim_matches('_'))
}

fn write_shape_drawable(
    layout_path: &std::path::Path,
    name: &str,
    background: &str,
    radius: &str,
) -> Result<()> {
    let res_dir = layout_path
        .parent()
        .and_then(std::path::Path::parent)
        .context("无法定位 res 目录")?;
    let drawable_dir = res_dir.join("drawable");
    fs::create_dir_all(&drawable_dir)?;
    let xml = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<shape xmlns:android=\"http://schemas.android.com/apk/res/android\" android:shape=\"rectangle\">\n    <solid android:color=\"{}\" />\n    <corners android:radius=\"{}\" />\n</shape>\n", escape_xml(background), escape_xml(radius));
    fs::write(drawable_dir.join(format!("{name}.xml")), xml)?;
    Ok(())
}

fn current_attribute(tag: &str, attribute: &str) -> Option<String> {
    let needle = format!("{attribute}=");
    let index = tag.find(&needle)? + needle.len();
    let quote = *tag.as_bytes().get(index)? as char;
    let start = index + 1;
    let end = start + tag[start..].find(quote)?;
    Some(tag[start..end].to_string())
}

fn property_attribute(property: &str) -> Option<&'static str> {
    match property {
        "text" => Some("android:text"),
        "textColor" => Some("android:textColor"),
        "background" => Some("android:background"),
        "width" => Some("android:layout_width"),
        "height" => Some("android:layout_height"),
        "fontSize" => Some("android:textSize"),
        "opacity" => Some("android:alpha"),
        "gravity" => Some("android:gravity"),
        "paddingStart" => Some("android:paddingStart"),
        "paddingTop" => Some("android:paddingTop"),
        "paddingEnd" => Some("android:paddingEnd"),
        "paddingBottom" => Some("android:paddingBottom"),
        "marginStart" => Some("android:layout_marginStart"),
        "marginTop" => Some("android:layout_marginTop"),
        "marginEnd" => Some("android:layout_marginEnd"),
        "marginBottom" => Some("android:layout_marginBottom"),
        _ => None,
    }
}

fn replace_attribute(tag: &str, attribute: &str, value: &str) -> String {
    let needle = format!("{attribute}=");
    if let Some(index) = tag.find(&needle) {
        let quote_index = index + needle.len();
        let quote = tag.as_bytes().get(quote_index).copied().unwrap_or(b'"') as char;
        if (quote == '"' || quote == '\'') && tag[quote_index + 1..].find(quote).is_some() {
            let value_start = quote_index + 1;
            let value_end = value_start + tag[value_start..].find(quote).unwrap_or(0);
            let mut next = tag.to_string();
            next.replace_range(value_start..value_end, &escape_xml(value));
            return next;
        }
    }
    let close = tag
        .rfind("/>")
        .or_else(|| tag.rfind('>'))
        .unwrap_or(tag.len());
    let mut insert = close;
    while insert > 0 && tag.as_bytes()[insert - 1].is_ascii_whitespace() {
        insert -= 1;
    }
    let separator = if tag.contains('\n') {
        let indentation = tag
            .lines()
            .rev()
            .find(|line| line.trim_start().starts_with("android:"))
            .map(|line| &line[..line.len() - line.trim_start().len()])
            .unwrap_or("    ");
        format!("\n{indentation}")
    } else {
        " ".to_string()
    };
    let mut next = tag.to_string();
    next.insert_str(
        insert,
        &format!("{separator}{attribute}=\"{}\"", escape_xml(value)),
    );
    next
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_source_preview::parser::load_document;
    use std::{collections::BTreeMap, path::Path};

    #[test]
    fn commits_text_padding_color_and_radius_to_xml_sources() {
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/node_agent_source_preview/testdata");
        let target =
            std::env::temp_dir().join(format!("elon-source-preview-writer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&target);
        copy_tree(&source, &target).unwrap();
        let document = load_document(target.to_str().unwrap(), None).unwrap();
        let action = &document.root.children[1];
        let changes = BTreeMap::from([
            ("text".into(), "确定性写回".into()),
            ("background".into(), "#16A34A".into()),
            ("borderRadius".into(), "22dp".into()),
            ("paddingStart".into(), "18dp".into()),
        ]);
        commit_changes(&CommitPreviewRequest {
            project_root: target.to_string_lossy().to_string(),
            layout_file: document.selected_layout,
            source_revision: document.source_revision,
            node_key: action.key.clone(),
            start_tag_start: action.source.start_tag_start,
            start_tag_end: action.source.start_tag_end,
            changes,
        })
        .unwrap();
        let layout =
            fs::read_to_string(target.join("app/src/main/res/layout/activity_main.xml")).unwrap();
        assert!(layout.contains("android:text=\"确定性写回\""));
        assert!(layout.contains("android:paddingStart=\"18dp\""));
        assert!(layout.contains("@drawable/yilong_ui_id_action"));
        let drawable =
            fs::read_to_string(target.join("app/src/main/res/drawable/yilong_ui_id_action.xml"))
                .unwrap();
        assert!(drawable.contains("#16A34A"));
        assert!(drawable.contains("22dp"));
        fs::remove_dir_all(target).unwrap();
    }

    fn copy_tree(source: &Path, target: &Path) -> Result<()> {
        fs::create_dir_all(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let destination = target.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_tree(&entry.path(), &destination)?;
            } else {
                fs::copy(entry.path(), destination)?;
            }
        }
        Ok(())
    }
}
