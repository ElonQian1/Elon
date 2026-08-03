use super::{cdp::CdpClient, CaptureDiagnostic, PreviewStylePatch};
use serde_json::{json, Value};
use std::time::Instant;

const ALLOWED_PROPERTIES: &[&str] = &[
    "aligncontent",
    "alignitems",
    "alignself",
    "background",
    "backgroundcolor",
    "border",
    "bordercolor",
    "borderradius",
    "borderstyle",
    "borderwidth",
    "bottom",
    "boxshadow",
    "color",
    "columncount",
    "columngap",
    "display",
    "flex",
    "flexbasis",
    "flexdirection",
    "flexgrow",
    "flexshrink",
    "flexwrap",
    "fontfamily",
    "fontsize",
    "fontstyle",
    "fontweight",
    "gap",
    "gridautocolumns",
    "gridautorows",
    "gridcolumn",
    "gridrow",
    "gridtemplatecolumns",
    "gridtemplaterows",
    "height",
    "inset",
    "justifycontent",
    "justifyitems",
    "justifyself",
    "left",
    "letterspacing",
    "lineheight",
    "margin",
    "marginbottom",
    "marginleft",
    "marginright",
    "margintop",
    "maxheight",
    "maxwidth",
    "minheight",
    "minwidth",
    "opacity",
    "order",
    "overflow",
    "overflowx",
    "overflowy",
    "padding",
    "paddingbottom",
    "paddingleft",
    "paddingright",
    "paddingtop",
    "position",
    "right",
    "rowgap",
    "textalign",
    "textdecoration",
    "textoverflow",
    "texttransform",
    "top",
    "transform",
    "transformorigin",
    "transition",
    "visibility",
    "whitespace",
    "width",
    "wordbreak",
    "zindex",
];

pub(super) fn validate_patches(patches: &[PreviewStylePatch]) -> Result<(), CaptureDiagnostic> {
    if patches.is_empty() || patches.len() > 32 {
        return Err(invalid("previewStyle patches 必须包含 1..32 项"));
    }
    for patch in patches {
        let property = normalized_property(&patch.property);
        if !ALLOWED_PROPERTIES.contains(&property.as_str()) {
            return Err(invalid("previewStyle property 不在视觉样式白名单中"));
        }
        let value = patch.value.trim();
        let lower = value.to_ascii_lowercase();
        if value.chars().count() > 300
            || value
                .chars()
                .any(|ch| ch == '\0' || (ch.is_control() && ch != '\t'))
            || value.contains(';')
            || [
                "url(",
                "image-set(",
                "@import",
                "expression(",
                "javascript:",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            return Err(invalid("previewStyle value 过长或包含外部资源/脚本型语法"));
        }
    }
    Ok(())
}

pub(super) async fn preview(
    cdp: &mut CdpClient,
    session: &str,
    selector: &str,
    patches: &[PreviewStylePatch],
    deadline: Instant,
) -> Result<Value, CaptureDiagnostic> {
    let selector = serde_json::to_string(selector).map_err(|_| invalid("无法序列化 selector"))?;
    let patches = serde_json::to_string(patches).map_err(|_| invalid("无法序列化样式补丁"))?;
    let expression = format!(
        r#"(() => {{ try {{
            const element = document.querySelector({selector});
            if (!element) return "missing";
            const key = Symbol.for("elon.design.stylePreview.v1");
            const store = window[key] || (window[key] = new WeakMap());
            let originals = store.get(element);
            if (!originals) {{ originals = new Map(); store.set(element, originals); }}
            for (const patch of {patches}) {{
                const property = patch.property.replace(/[A-Z]/g, value => "-" + value.toLowerCase());
                if (!originals.has(property)) originals.set(property, [element.style.getPropertyValue(property), element.style.getPropertyPriority(property)]);
                element.style.setProperty(property, patch.value, "");
            }}
            return "previewed";
        }} catch (_) {{ return "invalid"; }} }})()"#
    );
    evaluate(cdp, session, &expression, deadline).await
}

pub(super) async fn restore(
    cdp: &mut CdpClient,
    session: &str,
    selector: &str,
    deadline: Instant,
) -> Result<Value, CaptureDiagnostic> {
    let selector = serde_json::to_string(selector).map_err(|_| invalid("无法序列化 selector"))?;
    let expression = format!(
        r#"(() => {{ try {{
            const element = document.querySelector({selector});
            if (!element) return "missing";
            const store = window[Symbol.for("elon.design.stylePreview.v1")];
            const originals = store && store.get(element);
            if (!originals) return "restored";
            for (const [property, original] of originals.entries()) {{
                if (original[0]) element.style.setProperty(property, original[0], original[1] || "");
                else element.style.removeProperty(property);
            }}
            store.delete(element);
            return "restored";
        }} catch (_) {{ return "invalid"; }} }})()"#
    );
    evaluate(cdp, session, &expression, deadline).await
}

async fn evaluate(
    cdp: &mut CdpClient,
    session: &str,
    expression: &str,
    deadline: Instant,
) -> Result<Value, CaptureDiagnostic> {
    cdp.command(
        "Runtime.evaluate",
        json!({"expression":expression,"returnByValue":true}),
        Some(session),
        deadline,
    )
    .await?
    .pointer("/result/value")
    .cloned()
    .ok_or_else(|| invalid("样式预览没有返回可验证结果"))
}

fn normalized_property(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '-' && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn invalid(message: &str) -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "STYLE_PREVIEW_INVALID",
        message,
        false,
        "只使用白名单视觉属性和值；外部资源、脚本和任意 CSS 声明均被拒绝",
    )
}
