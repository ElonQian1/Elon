use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;

use super::protocol::LiveUiNode;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StableSelector {
    definition_id: String,
    #[serde(default)]
    instance_key: Option<String>,
    #[serde(default)]
    screen_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectorArguments {
    #[serde(default)]
    runtime_node_id: Option<String>,
    #[serde(default)]
    selector: Option<StableSelector>,
    #[serde(default)]
    definition_id: Option<String>,
    #[serde(default)]
    instance_key: Option<String>,
    #[serde(default)]
    screen_id: Option<String>,
}

pub(super) fn resolve<'a>(nodes: &'a [LiveUiNode], arguments: &Value) -> Result<&'a LiveUiNode> {
    let arguments: SelectorArguments =
        serde_json::from_value(arguments.clone()).context("UI 节点选择器格式无效")?;
    let stable = arguments.selector.or_else(|| {
        arguments.definition_id.map(|definition_id| StableSelector {
            definition_id,
            instance_key: arguments.instance_key,
            screen_id: arguments.screen_id,
        })
    });
    if arguments.runtime_node_id.is_some() && stable.is_some() {
        bail!("runtimeNodeId 与 stable selector 不能同时提供")
    }
    if let Some(runtime_node_id) = arguments.runtime_node_id {
        return nodes
            .iter()
            .find(|node| node.runtime_node_id == runtime_node_id)
            .ok_or_else(|| anyhow!("找不到 runtimeNodeId={runtime_node_id} 的 UI 节点"));
    }
    let stable = stable.ok_or_else(|| {
        anyhow!("必须提供 runtimeNodeId，或 selector.definitionId/instanceKey/screenId")
    })?;
    if stable.definition_id.trim().is_empty() {
        bail!("selector.definitionId 不能为空")
    }
    let candidates = nodes
        .iter()
        .filter(|node| node.definition_id == stable.definition_id)
        .filter(|node| {
            stable
                .instance_key
                .as_deref()
                .is_none_or(|value| node.instance_key.as_deref() == Some(value))
        })
        .filter(|node| {
            stable
                .screen_id
                .as_deref()
                .is_none_or(|value| node.screen_id == value)
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [node] => Ok(*node),
        [] => bail!(
            "stable selector 未匹配 UI 节点: definitionId={}",
            stable.definition_id
        ),
        many => {
            let identities = many
                .iter()
                .take(8)
                .map(|node| {
                    format!(
                        "runtimeNodeId={},instanceKey={},screenId={}",
                        node.runtime_node_id,
                        node.instance_key.as_deref().unwrap_or("none"),
                        node.screen_id
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!(
                "stable selector 匹配到 {} 个节点，拒绝选择首项；请补充 instanceKey/screenId。candidates=[{}]",
                many.len(), identities
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(runtime: &str, instance: &str) -> LiveUiNode {
        serde_json::from_value(json!({
            "runtimeNodeId":runtime,"definitionId":"screen.row","instanceKey":instance,
            "parentRuntimeNodeId":null,"screenId":"screen","kind":"row","className":"Row",
            "geometry":{"boundsInDisplayPx":{"left":0,"top":0,"right":10,"bottom":10,"width":10,"height":10},"density":1.0,"fontScale":1.0,"rotation":0,"visible":true},
            "properties":{},"capabilities":{}
        })).unwrap()
    }

    #[test]
    fn ambiguous_definition_never_silently_selects_first() {
        let nodes = vec![node("one", "a"), node("two", "b")];
        let error = resolve(&nodes, &json!({"selector":{"definitionId":"screen.row"}}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("拒绝选择首项"));
        assert_eq!(
            resolve(&nodes, &json!({"selector":{"definitionId":"screen.row","instanceKey":"b","screenId":"screen"}})).unwrap().runtime_node_id,
            "two"
        );
    }
}
