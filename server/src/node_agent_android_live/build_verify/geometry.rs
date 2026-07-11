use anyhow::{bail, Result};

use super::super::broker::LiveCommitSnapshot;
use super::super::protocol::{LiveStylePatch, LiveUiNode};
use super::super::visual_diff::PixelRect;

pub(super) fn patched_bounds(
    snapshot: &LiveCommitSnapshot,
    allow_runtime_id: bool,
) -> Option<PixelRect> {
    patched_bounds_for_nodes(&snapshot.nodes, &snapshot.patches, allow_runtime_id)
}

pub(super) fn verification_bounds(
    nodes: &[LiveUiNode],
    definition_id: Option<&str>,
    instance_key: Option<&str>,
) -> Result<Option<PixelRect>> {
    let Some(definition_id) = definition_id else {
        return Ok(None);
    };
    let candidates = nodes
        .iter()
        .filter_map(|node| {
            if node.definition_id != definition_id
                || instance_key.is_some_and(|key| node.instance_key.as_deref() != Some(key))
                || !node.geometry.visible
            {
                return None;
            }
            let bounds = &node.geometry.bounds_in_display_px;
            (bounds.right > bounds.left && bounds.bottom > bounds.top).then_some(PixelRect {
                left: bounds.left,
                top: bounds.top,
                right: bounds.right,
                bottom: bounds.bottom,
            })
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [bounds] => Ok(Some(*bounds)),
        [] => bail!(
            "构建验收找不到目标节点 definitionId={definition_id} instanceKey={:?}",
            instance_key
        ),
        _ => bail!("构建验收目标节点不唯一 definitionId={definition_id}；请提供稳定 instanceKey"),
    }
}

pub(super) fn patched_bounds_for_nodes(
    nodes: &[LiveUiNode],
    patches: &[LiveStylePatch],
    allow_runtime_id: bool,
) -> Option<PixelRect> {
    let mut result: Option<PixelRect> = None;
    for patch in patches {
        let node = allow_runtime_id
            .then(|| patch.target.runtime_node_id.as_deref())
            .flatten()
            .and_then(|id| nodes.iter().find(|node| node.runtime_node_id == id))
            .or_else(|| {
                patch
                    .target
                    .definition_id
                    .as_deref()
                    .and_then(|definition| {
                        nodes.iter().find(|node| node.definition_id == definition)
                    })
            });
        let Some(node) = node.filter(|node| node.geometry.visible) else {
            continue;
        };
        let bounds = &node.geometry.bounds_in_display_px;
        if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
            continue;
        }
        result = Some(match result {
            Some(current) => PixelRect {
                left: current.left.min(bounds.left),
                top: current.top.min(bounds.top),
                right: current.right.max(bounds.right),
                bottom: current.bottom.max(bounds.bottom),
            },
            None => PixelRect {
                left: bounds.left,
                top: bounds.top,
                right: bounds.right,
                bottom: bounds.bottom,
            },
        });
    }
    result
}
