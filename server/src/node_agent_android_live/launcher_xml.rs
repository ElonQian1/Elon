use anyhow::{bail, Result};
use quick_xml::{events::Event, Reader};

use super::visual_diff::PixelRect;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LauncherNode {
    pub(super) text: String,
    pub(super) content_desc: String,
    pub(super) class_name: String,
    pub(super) clickable: bool,
    pub(super) rect: PixelRect,
}

pub(super) fn launcher_candidates(xml: &str, label: &str) -> Result<Vec<LauncherNode>> {
    Ok(parse_nodes(xml)?
        .into_iter()
        .filter(|node| node.clickable && node.class_name.ends_with("TextView"))
        .filter(|node| node.text == label || node.content_desc == label)
        .filter(|node| node.rect.right > node.rect.left && node.rect.bottom > node.rect.top)
        .collect())
}

pub(super) fn parse_nodes(xml: &str) -> Result<Vec<LauncherNode>> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut result = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event) | Event::Empty(event)) if event.name().as_ref() == b"node" => {
                let mut node = LauncherNode {
                    text: String::new(),
                    content_desc: String::new(),
                    class_name: String::new(),
                    clickable: false,
                    rect: PixelRect {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    },
                };
                let mut bounds = String::new();
                for attribute in event.attributes().flatten() {
                    let value = attribute.unescape_value()?.into_owned();
                    match attribute.key.as_ref() {
                        b"text" => node.text = value,
                        b"content-desc" => node.content_desc = value,
                        b"class" => node.class_name = value,
                        b"clickable" => node.clickable = value == "true",
                        b"bounds" => bounds = value,
                        _ => {}
                    }
                }
                if let Some(rect) = parse_bounds(&bounds) {
                    node.rect = rect;
                    result.push(node);
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => bail!("Launcher XML 解析失败: {error}"),
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

pub(super) fn page_identity(xml: &str) -> String {
    parse_nodes(xml)
        .ok()
        .and_then(|nodes| {
            nodes.into_iter().find(|node| {
                node.content_desc.contains("页共") || node.content_desc.contains("page of")
            })
        })
        .map(|node| node.content_desc)
        .unwrap_or_else(|| format!("xml-bytes:{}", xml.len()))
}

fn parse_bounds(value: &str) -> Option<PixelRect> {
    let values = value
        .split(|ch: char| !ch.is_ascii_digit() && ch != '-')
        .filter(|part| !part.is_empty())
        .map(str::parse::<i32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    (values.len() == 4).then(|| PixelRect {
        left: values[0],
        top: values[1],
        right: values[2],
        bottom: values[3],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_matching_clickable_launcher_icons() {
        let xml = r#"<?xml version='1.0'?><hierarchy><node text="一龙" content-desc="一龙" class="android.widget.TextView" clickable="true" bounds="[540,151][794,426]"/><node text="一龙" content-desc="一龙" class="android.widget.TextView" clickable="false" bounds="[0,0][1,1]"/></hierarchy>"#;
        let nodes = launcher_candidates(xml, "一龙").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].rect.left, 540);
    }

    #[test]
    fn parses_miui_page_identity() {
        let xml = r#"<?xml version='1.0'?><hierarchy><node text="" content-desc="第 21 页共 21 页" class="android.widget.SeekBar" clickable="false" bounds="[0,0][1,1]"/></hierarchy>"#;
        assert_eq!(page_identity(xml), "第 21 页共 21 页");
    }
}
