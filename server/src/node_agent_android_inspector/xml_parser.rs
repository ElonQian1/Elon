use anyhow::Result;
use quick_xml::{events::BytesStart, events::Event, Reader};

use super::types::{BoundsRect, RuntimeUiNode};

pub(crate) fn validate_ui_xml(xml: &str) -> Result<()> {
    let trimmed = xml.trim_start();
    if trimmed.is_empty() {
        anyhow::bail!("XML 内容为空");
    }
    if !trimmed.starts_with("<?xml") {
        if trimmed.contains("could not get idle state") {
            anyhow::bail!("应用暂未进入 idle 状态，UiAutomator 无法读取");
        }
        if trimmed.contains("null root node") {
            anyhow::bail!("UiAutomator 返回空根节点");
        }
        anyhow::bail!(
            "XML 格式无效: {}",
            trimmed.chars().take(120).collect::<String>()
        );
    }
    if !trimmed.contains("<hierarchy") {
        anyhow::bail!("XML 缺少 hierarchy 节点");
    }
    Ok(())
}

pub(crate) fn parse_runtime_nodes(xml: &str) -> Result<Vec<RuntimeUiNode>> {
    validate_ui_xml(xml)?;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut nodes = Vec::new();
    let mut index_path_stack: Vec<u32> = Vec::new();
    let mut sibling_count_stack: Vec<u32> = vec![0];
    let mut next_id = 0usize;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref event)) if event.name().as_ref() == b"node" => {
                push_node(
                    event,
                    &mut nodes,
                    &mut next_id,
                    &mut index_path_stack,
                    &mut sibling_count_stack,
                    true,
                )?;
            }
            Ok(Event::Empty(ref event)) if event.name().as_ref() == b"node" => {
                push_node(
                    event,
                    &mut nodes,
                    &mut next_id,
                    &mut index_path_stack,
                    &mut sibling_count_stack,
                    false,
                )?;
            }
            Ok(Event::End(ref event)) if event.name().as_ref() == b"node" => {
                index_path_stack.pop();
                if sibling_count_stack.len() > 1 {
                    sibling_count_stack.pop();
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => anyhow::bail!("XML 解析失败: {error}"),
            _ => {}
        }
        buf.clear();
    }
    Ok(nodes)
}

fn push_node(
    event: &BytesStart<'_>,
    nodes: &mut Vec<RuntimeUiNode>,
    next_id: &mut usize,
    index_path_stack: &mut Vec<u32>,
    sibling_count_stack: &mut Vec<u32>,
    has_children: bool,
) -> Result<()> {
    let current_index = sibling_count_stack.last().copied().unwrap_or(0);
    if let Some(count) = sibling_count_stack.last_mut() {
        *count += 1;
    }
    let mut index_path = index_path_stack.clone();
    index_path.push(current_index);
    *next_id += 1;
    let node = node_from_attributes(event, *next_id, index_path.clone())?;
    nodes.push(node);
    if has_children {
        index_path_stack.push(current_index);
        sibling_count_stack.push(0);
    }
    Ok(())
}

fn node_from_attributes(
    event: &BytesStart<'_>,
    index: usize,
    index_path: Vec<u32>,
) -> Result<RuntimeUiNode> {
    let mut text = String::new();
    let mut content_desc = String::new();
    let mut resource_id = None;
    let mut package_name = None;
    let mut class_name = None;
    let mut bounds = None;
    let mut clickable = false;
    let mut enabled = true;
    let mut focusable = false;
    let mut focused = false;
    let mut scrollable = false;
    let mut checkable = false;
    let mut checked = false;
    let mut selected = false;
    let mut password = false;
    let mut visible = true;

    for attr in event.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or_default();
        let value = unescape_xml_attr(std::str::from_utf8(attr.value.as_ref()).unwrap_or_default());
        match key {
            "text" => text = value,
            "content-desc" => content_desc = value,
            "resource-id" => resource_id = non_empty(value),
            "package" => package_name = non_empty(value),
            "class" => class_name = non_empty(value),
            "bounds" => bounds = parse_bounds(&value),
            "clickable" => clickable = value == "true",
            "enabled" => enabled = value != "false",
            "focusable" => focusable = value == "true",
            "focused" => focused = value == "true",
            "scrollable" => scrollable = value == "true",
            "checkable" => checkable = value == "true",
            "checked" => checked = value == "true",
            "selected" => selected = value == "true",
            "password" => password = value == "true",
            "displayed" => visible = value != "false",
            _ => {}
        }
    }

    let bounds = bounds.unwrap_or(BoundsRect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
        width: 0,
        height: 0,
    });
    Ok(RuntimeUiNode {
        id: format!("runtime-node-{index}"),
        depth: index_path.len(),
        xpath: xpath_from_index_path(&index_path),
        index_path,
        text,
        content_desc,
        resource_id,
        package_name,
        class_name,
        bounds,
        clickable,
        enabled,
        focusable,
        focused,
        scrollable,
        checkable,
        checked,
        selected,
        password,
        visible,
        source: None,
        source_candidates: Vec::new(),
    })
}

pub(crate) fn parse_bounds(value: &str) -> Option<BoundsRect> {
    let cleaned = value.replace("][", ",").replace(['[', ']'], "");
    let mut parts = cleaned
        .split(',')
        .filter_map(|part| part.parse::<i32>().ok());
    let left = parts.next()?;
    let top = parts.next()?;
    let right = parts.next()?;
    let bottom = parts.next()?;
    Some(BoundsRect {
        left,
        top,
        right,
        bottom,
        width: (right - left).max(0),
        height: (bottom - top).max(0),
    })
}

fn xpath_from_index_path(index_path: &[u32]) -> String {
    if index_path.is_empty() {
        return "/hierarchy".to_string();
    }
    let mut value = String::from("/hierarchy");
    for index in index_path {
        value.push_str(&format!("/node[{index}]"));
    }
    value
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn unescape_xml_attr(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bounds() {
        assert_eq!(
            parse_bounds("[1,2][101,202]").unwrap(),
            BoundsRect {
                left: 1,
                top: 2,
                right: 101,
                bottom: 202,
                width: 100,
                height: 200,
            }
        );
    }

    #[test]
    fn parses_runtime_nodes_from_uiautomator_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hierarchy rotation="0"><node index="0" text="" resource-id="com.elon.app:id/root" class="android.widget.FrameLayout" package="com.elon.app" bounds="[0,0][1080,2400]"><node index="0" text="首页" resource-id="com.elon.app:id/topTitleText" class="android.widget.TextView" package="com.elon.app" clickable="false" enabled="true" bounds="[36,60][360,132]" /></node></hierarchy>"#;
        let nodes = parse_runtime_nodes(xml).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(
            nodes[1].resource_id.as_deref(),
            Some("com.elon.app:id/topTitleText")
        );
        assert_eq!(nodes[1].index_path, vec![0, 0]);
    }
}
