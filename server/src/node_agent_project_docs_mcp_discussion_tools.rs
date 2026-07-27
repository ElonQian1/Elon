//! MCP tools for compiling long conversations into a reusable discussion graph.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

use crate::{
    project_discussion_graph::{apply_proposal, load_graph, load_proposal, save_proposal},
    project_discussion_graph_model::{DiscussionGraphProposal, DiscussionNode},
    project_document_authorization::DocumentAutomationMode,
    project_document_response::{pagination, ProjectionRequest},
};

#[derive(Debug, Deserialize)]
struct GraphArguments {
    #[serde(default)]
    root_id: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct NodeArguments {
    node_id: String,
}

#[derive(Debug, Deserialize)]
struct SaveArguments {
    proposal: DiscussionGraphProposal,
    #[serde(default)]
    authorization_mode: DocumentAutomationMode,
    #[serde(default)]
    expected_graph_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApplyArguments {
    #[serde(default)]
    authorization_mode: DocumentAutomationMode,
    #[serde(default)]
    reviewed: bool,
    #[serde(default)]
    expected_graph_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "project_discussions_get_graph",
            "分页读取独立讨论推理图；返回显式讨论节点、来源锚点和分叉关系，不读取原始聊天正文。长聊天整理先调用它增量了解已有结构。",
            json!({"type":"object","properties":{
                "root_id":{"type":"string","maxLength":100},
                "query":{"type":"string","maxLength":300},
                "kind":{"type":"string","maxLength":40},
                "status":{"type":"string","maxLength":40},
                "offset":{"type":"integer","minimum":0,"default":0},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                "projection":{"type":"string","enum":["summary","page","detail","full"],"default":"page"}
            }}),
        ),
        tool(
            "project_discussions_get_node",
            "读取一个讨论节点、直接父子节点、相邻分叉、来源锚点、关联文档和功能节点；不读取聊天或 Markdown 正文。",
            json!({"type":"object","required":["node_id"],"properties":{
                "node_id":{"type":"string","maxLength":100}
            }}),
        ),
        tool(
            "project_discussions_get_suggestions",
            "读取待应用的讨论图拆分和节点晋升建议及 revision；不修改项目。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "project_discussions_save_proposal",
            "保存从聊天提取的讨论节点、关系和文档晋升建议。只写建议，不改变当前讨论图；必须保留来源锚点，不能把假设自动标为当前事实。",
            save_schema(),
        ),
        tool(
            "project_discussions_apply",
            "应用已保存的讨论图建议，并创建建议中明确列出的新 Markdown；禁止覆盖现有不同内容。git_backed_full 默认生成整理前后文档提交。",
            json!({"type":"object","properties":{
                "authorization_mode":{"type":"string","enum":["git_backed_full","trusted_reversible","review_all","suggestions_only"],"default":"git_backed_full"},
                "reviewed":{"type":"boolean","default":false},
                "expected_graph_revision":{"type":"string"},
                "expected_suggestions_revision":{"type":"string"}
            }}),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let value = match name {
        "project_discussions_get_graph" => {
            let projection = ProjectionRequest::from_arguments(&arguments)?;
            let input: GraphArguments = decode(arguments)?;
            get_graph(workspace, input, projection)?
        }
        "project_discussions_get_node" => {
            let input: NodeArguments = decode(arguments)?;
            get_node(workspace, &input.node_id)?
        }
        "project_discussions_get_suggestions" => get_suggestions(workspace)?,
        "project_discussions_save_proposal" => {
            let input: SaveArguments = decode(arguments)?;
            save_proposal(
                workspace,
                input.proposal,
                input.authorization_mode,
                input.expected_graph_revision.as_deref(),
                input.expected_suggestions_revision.as_deref(),
            )?
        }
        "project_discussions_apply" => {
            let input: ApplyArguments = decode(arguments)?;
            apply_proposal(
                workspace,
                input.authorization_mode,
                input.reviewed,
                input.expected_graph_revision.as_deref(),
                input.expected_suggestions_revision.as_deref(),
            )?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn get_graph(
    workspace: &Path,
    input: GraphArguments,
    projection: ProjectionRequest,
) -> Result<Value> {
    let graph = load_graph(workspace)?;
    let proposal = load_proposal(workspace)?;
    let descendants = descendants(&graph.value.nodes, &input.root_id);
    let query = input.query.trim().to_lowercase();
    let kind = input.kind.trim().to_ascii_lowercase();
    let status = input.status.trim().to_ascii_lowercase();
    let mut matching = graph
        .value
        .nodes
        .iter()
        .filter(|node| descendants.is_empty() || descendants.contains(&node.id))
        .filter(|node| kind.is_empty() || node.kind == kind)
        .filter(|node| status.is_empty() || node.status == status)
        .filter(|node| {
            query.is_empty()
                || format!(
                    "{} {} {} {} {}",
                    node.id,
                    node.title,
                    node.summary,
                    node.tags.join(" "),
                    node.document_paths.join(" ")
                )
                .to_lowercase()
                .contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    matching.sort_by_key(|node| (node.order, node.title.clone()));
    let total = matching.len();
    let page = if projection.projection == "summary" {
        Vec::new()
    } else {
        matching
            .into_iter()
            .skip(projection.offset)
            .take(projection.limit)
            .collect::<Vec<_>>()
    };
    let visible = page
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let edges = graph
        .value
        .edges
        .iter()
        .filter(|edge| {
            visible.contains(edge.source.as_str()) && visible.contains(edge.target.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let returned = page.len();
    Ok(json!({
        "version": graph.value.version,
        "graph_revision": graph.revision,
        "suggestions_revision": proposal.revision,
        "sources": if projection.is_full() { json!(graph.value.sources) } else { json!([]) },
        "nodes": page,
        "edges": edges,
        "counts": {
            "sources": graph.value.sources.len(),
            "nodes": graph.value.nodes.len(),
            "edges": graph.value.edges.len(),
            "roots": graph.value.nodes.iter().filter(|node| node.parent_id.is_empty()).count(),
            "open": graph.value.nodes.iter().filter(|node| matches!(node.status.as_str(), "open" | "exploring")).count(),
        },
        "pagination": pagination(projection.offset, projection.limit, total, returned),
        "budget": {"classification_model_tokens":0,"chat_bodies_read":0,"metadata_only":true},
    }))
}

fn get_node(workspace: &Path, node_id: &str) -> Result<Value> {
    let graph = load_graph(workspace)?;
    let id = node_id.trim().to_ascii_lowercase();
    let node = graph
        .value
        .nodes
        .iter()
        .find(|node| node.id == id)
        .ok_or_else(|| anyhow::anyhow!("讨论节点不存在：{id}"))?;
    let parent = graph
        .value
        .nodes
        .iter()
        .find(|candidate| candidate.id == node.parent_id);
    let children = graph
        .value
        .nodes
        .iter()
        .filter(|candidate| candidate.parent_id == node.id)
        .collect::<Vec<_>>();
    let edges = graph
        .value
        .edges
        .iter()
        .filter(|edge| edge.source == node.id || edge.target == node.id)
        .collect::<Vec<_>>();
    Ok(json!({
        "graph_revision": graph.revision,
        "node": node,
        "parent": parent,
        "children": children,
        "edges": edges,
        "budget": {"classification_model_tokens":0,"chat_bodies_read":0,"metadata_only":true},
    }))
}

fn get_suggestions(workspace: &Path) -> Result<Value> {
    let proposal = load_proposal(workspace)?;
    let counts = proposal
        .value
        .as_ref()
        .map(|proposal| crate::project_discussion_graph_validation::counts(
            &proposal.graph,
            proposal.promotions.len(),
        ))
        .unwrap_or_else(|| json!({"sources":0,"nodes":0,"edges":0,"roots":0,"open":0,"accepted":0,"promotions":0}));
    Ok(json!({
        "suggestions": proposal.value,
        "suggestions_revision": proposal.revision,
        "counts": counts,
        "default_authorization_mode": DocumentAutomationMode::GitBackedFull,
    }))
}

fn descendants(nodes: &[DiscussionNode], root_id: &str) -> HashSet<String> {
    let root = root_id.trim().to_ascii_lowercase();
    if root.is_empty() {
        return HashSet::new();
    }
    let mut selected = HashSet::from([root]);
    loop {
        let before = selected.len();
        for node in nodes {
            if selected.contains(&node.parent_id) {
                selected.insert(node.id.clone());
            }
        }
        if selected.len() == before {
            break;
        }
    }
    selected
}

fn save_schema() -> Value {
    json!({"type":"object","required":["proposal"],"properties":{
        "authorization_mode":{"type":"string","enum":["git_backed_full","trusted_reversible","review_all","suggestions_only"],"default":"git_backed_full"},
        "expected_graph_revision":{"type":"string"},
        "expected_suggestions_revision":{"type":"string"},
        "proposal":{"type":"object","required":["status","graph"],"properties":{
            "version":{"type":"integer","const":1},
            "status":{"type":"string","enum":["ready"]},
            "summary":{"type":"string","maxLength":1000},
            "documents_read":{"type":"integer","minimum":0},
            "estimated_tokens_used":{"type":"integer","minimum":0},
            "graph":{"type":"object","properties":{
                "sources":{"type":"array","maxItems":512,"items":{"type":"object","required":["id","title"],"properties":{
                    "id":{"type":"string","maxLength":100},"title":{"type":"string","maxLength":160},
                    "kind":{"type":"string","maxLength":40},"reference":{"type":"string","maxLength":1000},
                    "imported_at":{"type":"string","maxLength":64}
                }}},
                "nodes":{"type":"array","maxItems":4096,"items":{"type":"object","required":["id","title"],"properties":{
                    "id":{"type":"string","maxLength":100},"root_id":{"type":"string","maxLength":100},
                    "parent_id":{"type":"string","maxLength":100},"kind":{"type":"string","maxLength":40},
                    "title":{"type":"string","maxLength":120},"summary":{"type":"string","maxLength":1200},
                    "status":{"type":"string","maxLength":40},"authority":{"type":"string","maxLength":40},
                    "section_id":{"type":"string","maxLength":100},"order":{"type":"integer","minimum":0},
                    "color":{"type":"string","pattern":"^#[0-9A-Fa-f]{6}$"},
                    "source_refs":{"type":"array","maxItems":48,"items":{"type":"string","maxLength":300}},
                    "conversation_refs":{"type":"array","maxItems":24,"items":{"type":"string","maxLength":300}},
                    "document_paths":{"type":"array","maxItems":48,"items":{"type":"string"}},
                    "feature_node_ids":{"type":"array","maxItems":48,"items":{"type":"string","maxLength":100}},
                    "tags":{"type":"array","maxItems":24,"items":{"type":"string","maxLength":80}}
                }}},
                "edges":{"type":"array","maxItems":8192,"items":{"type":"object","required":["id","source","target"],"properties":{
                    "id":{"type":"string","maxLength":120},"source":{"type":"string","maxLength":100},
                    "target":{"type":"string","maxLength":100},"relation":{"type":"string","maxLength":40},
                    "label":{"type":"string","maxLength":100}
                }}}
            }},
            "promotions":{"type":"array","maxItems":256,"items":{"type":"object","required":["id","node_id","path","title","content"],"properties":{
                "id":{"type":"string","maxLength":120},"node_id":{"type":"string","maxLength":100},
                "path":{"type":"string"},"title":{"type":"string","maxLength":160},
                "content":{"type":"string","maxLength":2097152},"document_type":{"type":"string","maxLength":40},
                "section_id":{"type":"string","maxLength":100}
            }}}
        }}
    }})
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value) -> Result<T> {
    serde_json::from_value(arguments).map_err(Into::into)
}
