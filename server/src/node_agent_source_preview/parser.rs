use super::resources::{scalar, AndroidResources};
use super::types::*;
use anyhow::{bail, Context, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

struct OpenNode {
    node: PreviewNode,
}

pub(crate) fn load_document(
    project_root: &str,
    requested: Option<&str>,
) -> Result<SourcePreviewDocument> {
    let root = canonical_project_root(project_root)?;
    let layouts = discover_layouts(&root)?;
    if layouts.is_empty() {
        bail!("项目中没有找到 src/main/res/layout/*.xml");
    }
    let selected = requested
        .filter(|item| layouts.iter().any(|path| path == item))
        .map(str::to_string)
        .or_else(|| {
            layouts
                .iter()
                .find(|path| path.ends_with("activity_main.xml"))
                .cloned()
        })
        .unwrap_or_else(|| layouts[0].clone());
    let path = safe_join(&root, &selected)?;
    if fs::metadata(&path)?.len() > 2 * 1024 * 1024 {
        bail!("布局文件超过 2MB，拒绝在交互式预览中加载");
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("读取布局失败: {}", path.display()))?;
    let resources = AndroidResources::load(&path)?;
    let root_node = parse_layout(&content, &selected, &resources)?;
    let revision = hex::encode(Sha256::digest(content.as_bytes()));
    Ok(SourcePreviewDocument {
        ok: true,
        ir_kind: "elon.source_ui_ir".into(),
        ir_version: 1,
        project_root: root.to_string_lossy().to_string(),
        layout_files: layouts,
        selected_layout: selected,
        source_revision: revision,
        rendering: PreviewRendering {
            backend: "react_twin".into(),
            authoritative: false,
            source_of_truth: "android_source".into(),
            calibration_required: true,
        },
        canvas: PreviewCanvas {
            width: 393.0,
            height: 852.0,
            background: "#f7f8fa".into(),
        },
        root: root_node,
    })
}

pub(crate) fn canonical_project_root(raw: &str) -> Result<PathBuf> {
    let path = PathBuf::from(raw.trim());
    if raw.trim().is_empty() || !path.is_dir() {
        bail!("请选择有效的本机 Android 项目目录");
    }
    path.canonicalize().context("无法解析项目目录")
}

pub(crate) fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let normalized = relative.replace('\\', "/");
    if !normalized.contains("/src/main/res/layout/") || !normalized.ends_with(".xml") {
        bail!("只允许访问项目 src/main/res/layout 下的 XML 布局");
    }
    let candidate = root
        .join(relative)
        .canonicalize()
        .context("布局文件不存在")?;
    if !candidate.starts_with(root) {
        bail!("布局路径越出项目目录");
    }
    Ok(candidate)
}

fn discover_layouts(root: &Path) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for entry in ignore::WalkBuilder::new(root)
        .hidden(false)
        .max_depth(Some(8))
        .build()
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|v| v.to_str()) != Some("xml") {
            continue;
        }
        let normalized = path.to_string_lossy().replace('\\', "/");
        if !normalized.contains("/src/main/res/layout/") {
            continue;
        }
        result.push(
            path.strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/"),
        );
        if result.len() >= 500 {
            break;
        }
    }
    result.sort();
    Ok(result)
}

fn parse_layout(
    content: &str,
    layout_file: &str,
    resources: &AndroidResources,
) -> Result<PreviewNode> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<OpenNode> = Vec::new();
    let mut root = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event_into(&mut buf)?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(tag) => stack.push(OpenNode {
                node: build_node(&tag, layout_file, start, end, resources, &stack),
            }),
            Event::Empty(tag) => {
                let node = build_node(&tag, layout_file, start, end, resources, &stack);
                attach_node(node, &mut stack, &mut root);
            }
            Event::End(_) => {
                if let Some(open) = stack.pop() {
                    attach_node(open.node, &mut stack, &mut root);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    root.context("布局文件没有根节点")
}

fn attach_node(node: PreviewNode, stack: &mut [OpenNode], root: &mut Option<PreviewNode>) {
    if let Some(parent) = stack.last_mut() {
        parent.node.children.push(node);
    } else if root.is_none() {
        *root = Some(node);
    }
}

fn build_node(
    tag: &BytesStart<'_>,
    layout_file: &str,
    start: usize,
    end: usize,
    resources: &AndroidResources,
    stack: &[OpenNode],
) -> PreviewNode {
    let tag_name = String::from_utf8_lossy(tag.name().as_ref()).to_string();
    let attributes = tag
        .attributes()
        .flatten()
        .map(|attribute| {
            (
                String::from_utf8_lossy(attribute.key.as_ref()).to_string(),
                String::from_utf8_lossy(&attribute.value).to_string(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let attr = |name: &str| attributes.get(name).map(String::as_str).unwrap_or("");
    let resource_id = attr("android:id").split('/').nth(1).map(str::to_string);
    let key = resource_id
        .clone()
        .map(|id| format!("id/{id}"))
        .unwrap_or_else(|| {
            stack
                .last()
                .map(|parent| format!("{}/{}", parent.node.key, parent.node.children.len()))
                .unwrap_or_else(|| "path/root".into())
        });
    let simple = tag_name.rsplit('.').next().unwrap_or(&tag_name).to_string();
    let text = resources.resolve(attr("android:text"));
    let kind = classify(&simple);
    let raw_background = resources.resolve(attr("android:background"));
    let drawable = resources.drawable(&raw_background);
    let padding_all = scalar(&resources.resolve(attr("android:padding")));
    let margin_all = scalar(&resources.resolve(attr("android:layout_margin")));
    PreviewNode {
        key,
        resource_id: resource_id.clone(),
        tag: tag_name,
        name: resource_id.unwrap_or_else(|| {
            if text.is_empty() {
                simple.clone()
            } else {
                text.clone()
            }
        }),
        kind: kind.into(),
        source: PreviewNodeSource {
            layout_file: layout_file.into(),
            start_tag_start: start,
            start_tag_end: end,
            attributes: attributes.clone(),
        },
        layout: PreviewLayout {
            mode: if simple == "LinearLayout" {
                "flow"
            } else if matches!(
                simple.as_str(),
                "FrameLayout" | "ConstraintLayout" | "CoordinatorLayout"
            ) {
                "stack"
            } else {
                "leaf"
            }
            .into(),
            orientation: if attr("android:orientation") == "horizontal" {
                "row"
            } else {
                "column"
            }
            .into(),
            width: resources.resolve(attr("android:layout_width")),
            height: resources.resolve(attr("android:layout_height")),
            weight: attr("android:layout_weight").parse().unwrap_or(0.0),
            gravity: attr("android:gravity").into(),
            margin: edges(&attributes, "android:layout_margin", margin_all, resources),
            padding: edges(&attributes, "android:padding", padding_all, resources),
            gap: 0.0,
        },
        style: PreviewStyle {
            text,
            text_color: normalize_color(&resources.resolve(attr("android:textColor")), "#1c2430"),
            background: normalize_color(
                drawable
                    .as_ref()
                    .map(|item| item.0.as_str())
                    .unwrap_or(&raw_background),
                if kind == "button" {
                    "#2563eb"
                } else {
                    "transparent"
                },
            ),
            font_size: scalar(&resources.resolve(attr("android:textSize"))).max(
                if kind == "text" || kind == "button" {
                    14.0
                } else {
                    0.0
                },
            ),
            font_weight: if attr("android:textStyle").contains("bold") {
                700
            } else {
                400
            },
            border_radius: drawable.map(|item| item.1).unwrap_or(0.0),
            opacity: attr("android:alpha").parse().unwrap_or(1.0),
            visible: attr("android:visibility") != "gone",
            content_description: resources.resolve(attr("android:contentDescription")),
        },
        editable: vec![
            "text",
            "textColor",
            "background",
            "width",
            "height",
            "padding",
            "margin",
            "fontSize",
            "opacity",
            "gravity",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        children: Vec::new(),
    }
}

fn classify(tag: &str) -> &'static str {
    if tag.contains("Button") {
        "button"
    } else if tag.contains("EditText") {
        "input"
    } else if tag.contains("Text") {
        "text"
    } else if tag.contains("Image") {
        "image"
    } else if tag.contains("Recycler") || tag.contains("List") {
        "list"
    } else if tag.contains("Space") {
        "spacer"
    } else {
        "group"
    }
}

fn edges(
    attrs: &BTreeMap<String, String>,
    prefix: &str,
    all: f32,
    resources: &AndroidResources,
) -> EdgeValues {
    let value = |suffix: &str| {
        attrs
            .get(&format!("{prefix}{suffix}"))
            .map(|v| scalar(&resources.resolve(v)))
            .unwrap_or(all)
    };
    EdgeValues {
        start: value("Start"),
        top: value("Top"),
        end: value("End"),
        bottom: value("Bottom"),
    }
}

fn normalize_color(value: &str, fallback: &str) -> String {
    if value.len() == 9 && value.starts_with('#') {
        format!("#{}{}", &value[3..], &value[1..3])
    } else if value.starts_with('#') {
        value.into()
    } else {
        fallback.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_android_argb_to_css_rgba() {
        assert_eq!(normalize_color("#FF112233", "transparent"), "#112233FF");
        assert_eq!(normalize_color("#2AFFFFFF", "transparent"), "#FFFFFF2A");
    }

    #[test]
    fn loads_nested_source_preview_fixture() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/node_agent_source_preview/testdata");
        let document = load_document(root.to_str().unwrap(), None).unwrap();
        assert_eq!(document.root.layout.orientation, "column");
        assert_eq!(document.root.children.len(), 2);
        assert_eq!(document.root.children[0].style.text, "动态设计标题");
        assert_eq!(document.root.children[1].style.border_radius, 14.0);
    }
}
