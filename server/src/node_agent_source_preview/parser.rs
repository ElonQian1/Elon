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
    let fidelity = preview_fidelity(&root_node);
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
        fidelity,
        canvas: PreviewCanvas {
            width: 393.0,
            height: 852.0,
            background: "#f7f8fa".into(),
        },
        root: root_node,
    })
}

#[derive(Default)]
struct FidelityStats {
    total: u32,
    unsupported: u32,
    dynamic: u32,
    complex_stacks: u32,
    partial_resource_semantics: u32,
}

fn preview_fidelity(root: &PreviewNode) -> PreviewFidelity {
    let mut stats = FidelityStats::default();
    collect_fidelity_stats(root, &mut stats);

    let mut score = 100_i32;
    score -= (stats.unsupported.min(12) * 4) as i32;
    score -= (stats.dynamic.min(6) * 8) as i32;
    score -= (stats.complex_stacks.min(5) * 5) as i32;
    score -= (stats.partial_resource_semantics.min(5) * 3) as i32;
    if stats.total > 100 {
        score -= 20;
    } else if stats.total > 50 {
        score -= 12;
    } else if stats.total > 30 {
        score -= 6;
    }
    let score = score.clamp(0, 100) as u8;
    let safe_for_default_preview =
        score >= 75 && stats.dynamic == 0 && stats.unsupported <= 1 && stats.complex_stacks <= 1;
    let level = if safe_for_default_preview {
        "high"
    } else if score >= 55 {
        "medium"
    } else {
        "low"
    };

    let mut issues = Vec::new();
    if stats.dynamic > 0 {
        issues.push(format!(
            "检测到 {} 个列表、include 或自定义运行时节点，静态 XML 不包含其真实内容",
            stats.dynamic
        ));
    }
    if stats.unsupported > 0 {
        issues.push(format!(
            "检测到 {} 个 React 草稿无法完整模拟的 Android 控件",
            stats.unsupported
        ));
    }
    if stats.complex_stacks > 0 {
        issues.push(format!(
            "检测到 {} 个复杂叠放容器，缺少 Android 测量、约束和层级语义",
            stats.complex_stacks
        ));
    }
    if stats.partial_resource_semantics > 0 {
        issues.push("页面使用 Theme、Style、约束或状态资源，浏览器只能读取其中一部分".into());
    }
    if stats.total > 50 {
        issues.push(format!(
            "布局包含 {} 个节点，疑似承载多个页面状态，不适合直接当成单页设计稿",
            stats.total
        ));
    }
    if issues.is_empty() {
        issues.push("仅使用基础 XML 控件，可作为本地草稿；最终仍需 Android 真帧校准".into());
    }

    PreviewFidelity {
        score,
        level: level.into(),
        safe_for_default_preview,
        total_nodes: stats.total,
        unsupported_nodes: stats.unsupported,
        dynamic_nodes: stats.dynamic,
        issues,
    }
}

fn collect_fidelity_stats(node: &PreviewNode, stats: &mut FidelityStats) {
    stats.total += 1;
    let simple = node.tag.rsplit('.').next().unwrap_or(&node.tag);
    let is_dynamic = matches!(simple, "include" | "merge" | "fragment")
        || simple.contains("Recycler")
        || simple.contains("ListView")
        || simple.contains("GridView")
        || simple.contains("ViewPager")
        || simple.contains("WebView")
        || simple.contains("ComposeView")
        || simple.contains("SwipeRefresh");
    let supported = matches!(
        simple,
        "LinearLayout"
            | "FrameLayout"
            | "ScrollView"
            | "HorizontalScrollView"
            | "TextView"
            | "Button"
            | "ImageButton"
            | "ImageView"
            | "EditText"
            | "Space"
            | "View"
    );
    let custom_view = node.tag.contains('.')
        && !node.tag.starts_with("android.widget.")
        && !node.tag.starts_with("android.view.");
    if is_dynamic {
        stats.dynamic += 1;
    }
    if !supported || custom_view {
        stats.unsupported += 1;
    }
    if node.layout.mode == "stack" && node.children.len() > 2 {
        stats.complex_stacks += 1;
    }
    if node.source.attributes.iter().any(|(key, value)| {
        key == "style"
            || key.contains("layout_constraint")
            || key.contains("srcCompat")
            || value.starts_with("?attr/")
            || value.starts_with("@style/")
    }) {
        stats.partial_resource_semantics += 1;
    }
    for child in &node.children {
        collect_fidelity_stats(child, stats);
    }
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
        assert!(document.fidelity.safe_for_default_preview);
        assert_eq!(document.fidelity.level, "high");
    }

    #[test]
    fn blocks_dynamic_android_shell_from_default_react_preview() {
        let xml = r#"
            <FrameLayout xmlns:android="http://schemas.android.com/apk/res/android">
                <androidx.recyclerview.widget.RecyclerView android:layout_width="match_parent" android:layout_height="match_parent" />
                <include layout="@layout/page_tabs" />
                <com.example.CustomStatusView android:layout_width="match_parent" android:layout_height="48dp" />
                <LinearLayout android:layout_width="match_parent" android:layout_height="wrap_content">
                    <TextView android:text="标题" />
                    <TextView android:text="说明" />
                    <TextView android:text="状态" />
                </LinearLayout>
            </FrameLayout>
        "#;
        let root = parse_layout(
            xml,
            "app/src/main/res/layout/activity_main.xml",
            &AndroidResources::default(),
        )
        .unwrap();
        let fidelity = preview_fidelity(&root);
        assert!(!fidelity.safe_for_default_preview);
        assert!(fidelity.dynamic_nodes >= 2);
        assert!(fidelity.unsupported_nodes >= 3);
        assert!(!fidelity.issues.is_empty());
    }
}
