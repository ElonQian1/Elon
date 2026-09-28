//! Versioned, Git-tracked acceptance scope. Capture calls cannot shrink it.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Contract {
    pub schema_version: u32,
    pub task_id: String,
    pub scope: String,
    pub design_system: DesignSystem,
    pub requirement_spec: SourceSpec,
    pub states: Vec<State>,
    pub platform_differences: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DesignSystem {
    pub id: String,
    pub source_file: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SourceSpec {
    pub source_file: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    pub id: String,
    pub android_screen_id: String,
    pub android_font_scale: f64,
    pub web_capture: Value,
    pub capabilities: Vec<Capability>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Capability {
    pub id: String,
    pub state: String,
    pub android: AndroidMapping,
    pub web: WebMapping,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AndroidMapping {
    #[serde(default)]
    pub definition_id: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub instance_key: Option<String>,
    pub assertions: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WebMapping {
    pub selector: String,
    pub assertions: BTreeMap<String, Value>,
}

pub(crate) fn load(root: &Path, relative: &str, task_id: &str) -> Result<(Contract, String)> {
    let bytes = tracked_bytes(root, relative, 512 * 1024)?;
    let contract: Contract = serde_json::from_slice(&bytes).context("SEMANTIC_CONTRACT_INVALID")?;
    validate(&contract, task_id)?;
    let design = tracked_bytes(root, &contract.design_system.source_file, 2 * 1024 * 1024)?;
    if digest(&design) != contract.design_system.sha256 {
        bail!("SEMANTIC_DESIGN_VERSION_CHANGED");
    }
    let requirement = tracked_bytes(
        root,
        &contract.requirement_spec.source_file,
        2 * 1024 * 1024,
    )?;
    if digest(&requirement) != contract.requirement_spec.sha256 {
        bail!("SEMANTIC_REQUIREMENTS_CHANGED");
    }
    Ok((contract, digest(&bytes)))
}

pub(crate) fn validate(contract: &Contract, task_id: &str) -> Result<()> {
    if contract.schema_version != 1
        || contract.task_id != task_id
        || contract.scope != "OBSERVED_CAPABILITY_STATE"
        || contract.design_system.id != "mobile-design-v2"
        || !valid_hash(&contract.design_system.sha256)
        || !valid_hash(&contract.requirement_spec.sha256)
        || contract.states.is_empty()
        || contract.states.len() > 32
        || contract.platform_differences.is_empty()
        || contract.platform_differences.len() > 32
    {
        bail!("SEMANTIC_CONTRACT_INVALID: task/version/scope/design/states/differences mismatch");
    }
    let mut ids = BTreeSet::new();
    for state in &contract.states {
        if !safe_id(&state.id)
            || !ids.insert(&state.id)
            || !text(&state.android_screen_id, 200)
            || !state.android_font_scale.is_finite()
            || !(0.5..=4.0).contains(&state.android_font_scale)
            || state.capabilities.is_empty()
            || state.capabilities.len() > 100
        {
            bail!("SEMANTIC_CONTRACT_INVALID: empty/duplicate/unbounded state or capabilities");
        }
        let capture = state
            .web_capture
            .as_object()
            .context("SEMANTIC_CONTRACT_INVALID: webCapture must be object")?;
        if capture.keys().any(|key| {
            ![
                "url",
                "viewport",
                "waitFor",
                "capture",
                "authProfile",
                "fixtureProfile",
                "expectedPage",
                "steps",
            ]
            .contains(&key.as_str())
        }) {
            bail!("SEMANTIC_CONTRACT_INVALID: webCapture contains unknown fields or caller source evidence");
        }
        if capture.get("steps").is_some_and(|steps| {
            !steps.as_array().is_some_and(|steps| {
                steps.iter().all(|step| {
                    matches!(
                        step["action"].as_str(),
                        Some("waitFor" | "assertText" | "scrollIntoView")
                    )
                })
            })
        }) {
            bail!("SEMANTIC_CONTRACT_INVALID: observation-only capture steps required");
        }
        let mut capabilities = BTreeSet::new();
        let mut android = BTreeSet::new();
        let mut web = BTreeSet::new();
        for capability in &state.capabilities {
            if !safe_id(&capability.id)
                || !capabilities.insert(&capability.id)
                || !text(&capability.state, 200)
                || (!capability
                    .android
                    .definition_id
                    .as_deref()
                    .is_some_and(|v| text(v, 500))
                    && !capability
                        .android
                        .resource_id
                        .as_deref()
                        .is_some_and(|v| text(v, 500)))
                || !text(&capability.web.selector, 1000)
                || !android.insert((
                    &capability.android.definition_id,
                    &capability.android.resource_id,
                    &capability.android.instance_key,
                ))
                || !web.insert(&capability.web.selector)
            {
                bail!("SEMANTIC_CONTRACT_INVALID: missing or duplicate capability mapping");
            }
            validate_assertions(&capability.android.assertions, true)?;
            validate_assertions(&capability.web.assertions, false)?;
        }
    }
    if contract.platform_differences.iter().any(|v| !text(v, 1000)) {
        bail!("SEMANTIC_CONTRACT_INVALID: platform difference reasons required");
    }
    Ok(())
}

fn validate_assertions(assertions: &BTreeMap<String, Value>, android: bool) -> Result<()> {
    let type_present = if android {
        assertions.contains_key("/kind") || assertions.contains_key("/className")
    } else {
        assertions.contains_key("/role")
    };
    if assertions.len() < 2 || assertions.len() > 20 || !type_present {
        bail!("SEMANTIC_CONTRACT_INVALID: role/type and observable state assertions required");
    }
    for (pointer, expected) in assertions {
        let allowed = if android {
            matches!(pointer.as_str(), "/kind" | "/className" | "/text")
                || pointer.starts_with("/properties/")
                || pointer.starts_with("/capabilities/")
        } else {
            matches!(
                pointer.as_str(),
                "/role"
                    | "/label"
                    | "/disabled"
                    | "/checked"
                    | "/selected"
                    | "/inputType"
                    | "/interactive"
                    | "/style/color"
                    | "/style/backgroundColor"
                    | "/style/fontSize"
            )
        };
        if !allowed
            || pointer.len() > 200
            || expected.is_null()
            || expected.is_array()
            || expected.is_object()
            || expected.as_str().is_some_and(|s| !text(s, 500))
        {
            bail!("SEMANTIC_CONTRACT_INVALID: invalid state assertion");
        }
    }
    if android
        && assertions
            .keys()
            .all(|key| matches!(key.as_str(), "/kind" | "/className"))
    {
        bail!("SEMANTIC_CONTRACT_INVALID: type-only mapping does not prove capability state");
    }
    Ok(())
}

pub(crate) fn evaluate(state: &State, android_nodes: &[Value], web_tree: &Value) -> Result<Value> {
    if web_tree["schema"] != "elon.web.semantic-tree.v1" || web_tree["truncated"] != false {
        bail!("SEMANTIC_WEB_TREE_INVALID: complete real semantic tree required");
    }
    let web_nodes = web_tree["nodes"]
        .as_array()
        .context("SEMANTIC_WEB_TREE_INVALID: missing nodes")?;
    let mut outcomes = Vec::new();
    let mut observed_android_nodes = std::collections::BTreeSet::new();
    for capability in &state.capabilities {
        let android = unique(
            android_nodes.iter().filter(|node| {
                capability
                    .android
                    .definition_id
                    .as_deref()
                    .is_none_or(|id| node["definitionId"] == id)
                    && capability
                        .android
                        .resource_id
                        .as_deref()
                        .is_none_or(|id| node["resourceId"] == id)
                    && capability
                        .android
                        .instance_key
                        .as_deref()
                        .is_none_or(|key| node["instanceKey"] == key)
            }),
            &capability.id,
            "android",
        )?;
        let runtime_node_id = android["runtimeNodeId"]
            .as_str()
            .filter(|id| !id.trim().is_empty())
            .context("SEMANTIC_NODE_INVALID: native runtime identity required")?;
        if !observed_android_nodes.insert(runtime_node_id) {
            bail!("SEMANTIC_NODE_REUSED: distinct capabilities must map to distinct native nodes");
        }
        if android["screenId"] != state.android_screen_id
            || android.pointer("/geometry/visible") != Some(&Value::Bool(true))
            || android
                .pointer("/geometry/boundsInDisplayPx/width")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                <= 0.0
            || android
                .pointer("/geometry/boundsInDisplayPx/height")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                <= 0.0
            || !android
                .pointer("/geometry/fontScale")
                .and_then(Value::as_f64)
                .is_some_and(|v| (v - state.android_font_scale).abs() < 0.001)
        {
            bail!(
                "SEMANTIC_STATE_MISMATCH: {} native screen/visibility",
                capability.id
            );
        }
        let web = unique(
            web_nodes
                .iter()
                .filter(|node| node["selector"] == capability.web.selector),
            &capability.id,
            "web",
        )?;
        if web
            .pointer("/bounds/width")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            <= 0.0
            || web
                .pointer("/bounds/height")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                <= 0.0
        {
            bail!("SEMANTIC_STATE_MISMATCH: {} web visibility", capability.id);
        }
        assert_node(
            android,
            &capability.android.assertions,
            &capability.id,
            "android",
        )?;
        assert_node(web, &capability.web.assertions, &capability.id, "web")?;
        outcomes.push(serde_json::json!({"capabilityId":capability.id,"state":capability.state,"status":"PASSED","androidDefinitionId":capability.android.definition_id,"webSelector":capability.web.selector}));
    }
    Ok(
        serde_json::json!({"stateId":state.id,"scope":"OBSERVED_CAPABILITY_STATE","capabilities":outcomes}),
    )
}

fn unique<'a>(
    mut values: impl Iterator<Item = &'a Value>,
    id: &str,
    platform: &str,
) -> Result<&'a Value> {
    let node = values
        .next()
        .with_context(|| format!("SEMANTIC_NODE_MISSING: {id}/{platform}"))?;
    if values.next().is_some() {
        bail!("SEMANTIC_NODE_AMBIGUOUS: {id}/{platform}");
    }
    Ok(node)
}
fn assert_node(
    node: &Value,
    assertions: &BTreeMap<String, Value>,
    id: &str,
    platform: &str,
) -> Result<()> {
    for (pointer, expected) in assertions {
        if node.pointer(pointer) != Some(expected) {
            bail!("SEMANTIC_ASSERTION_FAILED: {id}/{platform}{pointer}");
        }
    }
    Ok(())
}
pub(crate) fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"_-".contains(&v))
}
fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub(crate) fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|v| v.is_ascii_hexdigit())
}

pub(crate) fn tracked_bytes(root: &Path, relative: &str, limit: usize) -> Result<Vec<u8>> {
    let path = Path::new(relative);
    if relative.contains(['\\', ':'])
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || relative.is_empty()
    {
        bail!("SEMANTIC_SOURCE_PATH_INVALID");
    }
    let root = root.canonicalize()?;
    let resolved = root.join(path).canonicalize()?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
        bail!("SEMANTIC_SOURCE_PATH_ESCAPE");
    }
    let bytes = std::fs::read(&resolved)?;
    if bytes.len() > limit {
        bail!("SEMANTIC_SOURCE_TOO_LARGE");
    }
    let mut command = elon_pc_dev_runtime::git_command();
    command
        .current_dir(&root)
        .args(["show", &format!("HEAD:{relative}")]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command.output()?;
    if !output.status.success() || output.stdout != bytes {
        bail!("SEMANTIC_SOURCE_NOT_COMMITTED: contract/design specification must match HEAD");
    }
    Ok(bytes)
}
