use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const CONFIG_FILE: &str = ".elon/ui-style-targets.json";
const MAX_CONFIG_BYTES: u64 = 128 * 1024;
const MAX_TOKEN_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StyleTargetConfig {
    #[serde(default = "version_one")]
    version: u32,
    #[serde(default = "default_token_file")]
    token_file: String,
    android_values_file: Option<String>,
    web_css_file: Option<String>,
    #[serde(default = "default_android_prefix")]
    android_name_prefix: String,
    #[serde(default = "default_web_prefix")]
    web_variable_prefix: String,
    #[serde(default = "default_web_selector")]
    web_selector: String,
}

pub(crate) fn generate_style_targets(root: &Path, tokens_changed: bool) -> Result<Vec<PathBuf>> {
    if !tokens_changed {
        return Ok(Vec::new());
    }
    let root = root.canonicalize().context("规范化 UI 样式项目目录失败")?;
    let config_path = root.join(CONFIG_FILE);
    if !config_path.is_file() {
        return Ok(Vec::new());
    }
    if fs::metadata(&config_path)?.len() > MAX_CONFIG_BYTES {
        bail!("UI 双端生成配置超过 128KiB");
    }
    let config: StyleTargetConfig = serde_json::from_str(&fs::read_to_string(&config_path)?)
        .context("解析 .elon/ui-style-targets.json 失败")?;
    if config.version != 1 {
        bail!("UI 双端生成配置仅支持 version=1");
    }
    let token_file = safe_target(&root, &config.token_file)?;
    if !token_file.is_file() || fs::metadata(&token_file)?.len() > MAX_TOKEN_BYTES {
        bail!("UI Token 文件不存在或超过 2MiB: {}", token_file.display());
    }
    let document: Value = serde_json::from_str(&fs::read_to_string(&token_file)?)
        .context("解析 UI Token JSON 失败")?;
    let mut tokens = BTreeMap::new();
    flatten_tokens("", &document, &mut tokens);
    if tokens.is_empty() {
        bail!("UI Token JSON 没有可生成的标量值");
    }
    let source_hash = hex::encode(Sha256::digest(serde_json::to_vec(&document)?));
    let mut generated = Vec::new();
    if let Some(relative) = config.android_values_file.as_deref() {
        let path = safe_target(&root, relative)?;
        write_generated(&root, &path, &android_xml(&tokens, &config, &source_hash)?)?;
        generated.push(path);
    }
    if let Some(relative) = config.web_css_file.as_deref() {
        let path = safe_target(&root, relative)?;
        write_generated(&root, &path, &web_css(&tokens, &config, &source_hash)?)?;
        generated.push(path);
    }
    if generated.is_empty() {
        bail!("UI 双端生成配置至少需要 androidValuesFile 或 webCssFile");
    }
    Ok(generated)
}

fn flatten_tokens(prefix: &str, value: &Value, output: &mut BTreeMap<String, Value>) {
    if let Value::Object(map) = value {
        for (key, child) in map {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            flatten_tokens(&path, child, output);
        }
    } else if value.is_string() || value.is_number() || value.is_boolean() {
        output.insert(prefix.to_string(), value.clone());
    }
}

fn android_xml(
    tokens: &BTreeMap<String, Value>,
    config: &StyleTargetConfig,
    hash: &str,
) -> Result<String> {
    let mut output = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!-- Generated from {} · sha256:{}; do not edit. -->\n<resources>\n", config.token_file, &hash[..16]);
    for (path, value) in tokens {
        let name = format!(
            "{}_{}",
            android_identifier(&config.android_name_prefix),
            android_identifier(path)
        );
        let (tag, rendered) = android_value(path, value)?;
        output.push_str(&format!(
            "    <{tag} name=\"{name}\">{}</{tag}>\n",
            xml_escape(&rendered)
        ));
    }
    output.push_str("</resources>\n");
    Ok(output)
}

fn web_css(
    tokens: &BTreeMap<String, Value>,
    config: &StyleTargetConfig,
    hash: &str,
) -> Result<String> {
    let selector = config.web_selector.trim();
    if selector.is_empty() || selector.contains(['{', '}']) {
        bail!("webSelector 非法");
    }
    let prefix = css_identifier(&config.web_variable_prefix);
    let mut output = format!(
        "/* Generated from {} · sha256:{}; do not edit. */\n{selector} {{\n",
        config.token_file,
        &hash[..16]
    );
    for (path, value) in tokens {
        output.push_str(&format!(
            "  --{}-{}: {};\n",
            prefix,
            css_identifier(path),
            css_value(path, value)?
        ));
    }
    output.push_str("}\n");
    Ok(output)
}

fn android_value(path: &str, value: &Value) -> Result<(&'static str, String)> {
    match value {
        Value::Bool(value) => Ok(("bool", value.to_string())),
        Value::String(value) if is_color(value) => Ok(("color", normalize_android_color(value))),
        Value::String(value) if is_dimension(value) => Ok(("dimen", value.clone())),
        Value::String(value) => Ok(("string", value.clone())),
        Value::Number(value) if integral_semantic(path) => Ok(("integer", value.to_string())),
        Value::Number(value) if scalar_semantic(path) => Ok(("string", format!("{}", value))),
        Value::Number(value) => Ok(("dimen", format!("{}dp", value))),
        _ => bail!("不支持生成的 Token 类型: {path}"),
    }
}

fn css_value(path: &str, value: &Value) -> Result<String> {
    match value {
        Value::Bool(value) => Ok(value.to_string()),
        Value::String(value) if is_color(value) => Ok(normalize_web_color(value)),
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) if scalar_semantic(path) || integral_semantic(path) => {
            Ok(value.to_string())
        }
        Value::Number(value) => Ok(format!("{}px", value)),
        _ => bail!("不支持生成的 Web Token 类型: {path}"),
    }
}

fn safe_target(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative = Path::new(relative.trim());
    if relative.as_os_str().is_empty()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("UI 样式目标必须是项目内相对路径");
    }
    Ok(root.join(relative))
}

fn write_generated(root: &Path, path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().context("UI 样式目标没有父目录")?;
    fs::create_dir_all(parent)?;
    let canonical = parent.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("拒绝写入项目目录外的 UI 样式目标");
    }
    fs::write(path, content).with_context(|| format!("写入 UI 样式目标失败: {}", path.display()))
}

fn android_identifier(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while output.contains("__") {
        output = output.replace("__", "_");
    }
    output.trim_matches('_').to_string()
}
fn css_identifier(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
fn is_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}
fn is_dimension(value: &str) -> bool {
    ["dp", "sp", "px"].iter().any(|unit| {
        value.ends_with(unit) && value[..value.len() - unit.len()].parse::<f64>().is_ok()
    })
}
fn scalar_semantic(path: &str) -> bool {
    ["opacity", "alpha", "ratio", "scale"]
        .iter()
        .any(|key| path.to_ascii_lowercase().contains(key))
}
fn integral_semantic(path: &str) -> bool {
    ["weight", "count", "lines", "index"]
        .iter()
        .any(|key| path.to_ascii_lowercase().contains(key))
}
fn normalize_android_color(value: &str) -> String {
    value.to_string()
}
fn normalize_web_color(value: &str) -> String {
    if value.len() == 9 {
        format!("#{}{}", &value[3..9], &value[1..3])
    } else {
        value.to_string()
    }
}
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn version_one() -> u32 {
    1
}
fn default_token_file() -> String {
    ".elon/ui-standards/tokens.json".to_string()
}
fn default_android_prefix() -> String {
    "elon_ui_".to_string()
}
fn default_web_prefix() -> String {
    "elon-ui-".to_string()
}
fn default_web_selector() -> String {
    ":root".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_android_and_web_from_one_token_document() {
        let root = env_root();
        fs::create_dir_all(root.join(".elon/ui-standards")).unwrap();
        fs::write(root.join(".elon/ui-standards/tokens.json"), r##"{"colors":{"primary":"#FF112233"},"spacing":{"medium":16},"opacity":{"disabled":0.4}}"##).unwrap();
        fs::write(root.join(CONFIG_FILE), r#"{"version":1,"androidValuesFile":"app/src/main/res/values/elon_ui_tokens.xml","webCssFile":"web/elon-ui-tokens.css"}"#).unwrap();
        let generated = generate_style_targets(&root, true).unwrap();
        assert_eq!(generated.len(), 2);
        let android =
            fs::read_to_string(root.join("app/src/main/res/values/elon_ui_tokens.xml")).unwrap();
        let web = fs::read_to_string(root.join("web/elon-ui-tokens.css")).unwrap();
        assert!(android.contains("<color name=\"elon_ui_colors_primary\">#FF112233</color>"));
        assert!(android.contains("<dimen name=\"elon_ui_spacing_medium\">16dp</dimen>"));
        assert!(web.contains("--elon-ui-spacing-medium: 16px;"));
        assert!(web.contains("--elon-ui-opacity-disabled: 0.4;"));
        fs::remove_dir_all(root).unwrap();
    }

    fn env_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-ui-style-codegen-{}",
            uuid::Uuid::new_v4().simple()
        ))
    }
}
