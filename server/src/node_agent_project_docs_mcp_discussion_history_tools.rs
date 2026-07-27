//! MCP tools for semantic discussion-graph history and node lifecycle tracing.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

use crate::{
    project_discussion_graph_history::{
        compare_discussion_versions, list_discussion_versions, load_discussion_graph_version,
        trace_discussion_node,
    },
    project_discussion_graph_model::DiscussionNode,
    project_document_response::{pagination, ProjectionRequest},
};

#[derive(Debug, Deserialize)]
struct HistoryArguments {
    #[serde(default)]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct VersionArguments {
    commit: String,
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
struct CompareArguments {
    base_commit: String,
    #[serde(default)]
    target_commit: String,
}

#[derive(Debug, Deserialize)]
struct TraceArguments {
    node_id: String,
    #[serde(default)]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "project_discussions_get_history",
            "读取讨论图的语义版本时间轴。每个版本只返回节点、关系和来源的变化数量，不读取聊天或文档正文。",
            json!({"type":"object","properties":{
                "limit":{"type":"integer","minimum":1,"maximum":100,"default":30}
            }}),
        ),
        tool(
            "project_discussions_get_graph_at_version",
            "按 Git 提交读取旧版讨论图并分页筛选，供 AI 或 PC 端回看当时的推理结构；不读取原始聊天正文。",
            json!({"type":"object","required":["commit"],"properties":{
                "commit":{"type":"string","pattern":"^[0-9A-Fa-f]{7,64}$"},
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
            "project_discussions_compare_versions",
            "语义比较两个讨论图版本，返回节点状态、父子关系、来源和边的增删改；不返回难读的原始 JSON 补丁。",
            json!({"type":"object","required":["base_commit"],"properties":{
                "base_commit":{"type":"string","pattern":"^[0-9A-Fa-f]{7,64}$"},
                "target_commit":{"type":"string","description":"可省略，默认与当前 HEAD 比较","pattern":"^(HEAD|[0-9A-Fa-f]{7,64})$"}
            }}),
        ),
        tool(
            "project_discussions_trace_node",
            "沿讨论图 Git 版本追踪单个稳定节点的创建、内容变化、状态迁移、父节点变化和关系变化。",
            json!({"type":"object","required":["node_id"],"properties":{
                "node_id":{"type":"string","maxLength":100},
                "limit":{"type":"integer","minimum":1,"maximum":100,"default":50}
            }}),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let value = match name {
        "project_discussions_get_history" => {
            let input: HistoryArguments = decode(arguments)?;
            list_discussion_versions(workspace, default_limit(input.limit, 30))?
        }
        "project_discussions_get_graph_at_version" => {
            let projection = ProjectionRequest::from_arguments(&arguments)?;
            let input: VersionArguments = decode(arguments)?;
            graph_at_version(workspace, input, projection)?
        }
        "project_discussions_compare_versions" => {
            let input: CompareArguments = decode(arguments)?;
            compare_discussion_versions(
                workspace,
                &input.base_commit,
                (!input.target_commit.trim().is_empty()).then_some(input.target_commit.as_str()),
            )?
        }
        "project_discussions_trace_node" => {
            let input: TraceArguments = decode(arguments)?;
            trace_discussion_node(workspace, &input.node_id, default_limit(input.limit, 50))?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn graph_at_version(
    workspace: &Path,
    input: VersionArguments,
    projection: ProjectionRequest,
) -> Result<Value> {
    let snapshot = load_discussion_graph_version(workspace, &input.commit)?;
    let descendants = descendants(&snapshot.graph.nodes, &input.root_id);
    let query = input.query.trim().to_lowercase();
    let kind = input.kind.trim().to_ascii_lowercase();
    let status = input.status.trim().to_ascii_lowercase();
    let mut matching = snapshot
        .graph
        .nodes
        .iter()
        .filter(|node| descendants.is_empty() || descendants.contains(&node.id))
        .filter(|node| kind.is_empty() || node.kind == kind)
        .filter(|node| status.is_empty() || node.status == status)
        .filter(|node| matches_query(node, &query))
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
    let edges = snapshot
        .graph
        .edges
        .iter()
        .filter(|edge| {
            visible.contains(edge.source.as_str()) && visible.contains(edge.target.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let returned = page.len();
    Ok(json!({
        "commit": snapshot.commit,
        "created_at": snapshot.created_at,
        "summary": snapshot.summary,
        "graph_revision": snapshot.graph_revision,
        "version": snapshot.graph.version,
        "sources": if projection.is_full() { json!(snapshot.graph.sources) } else { json!([]) },
        "nodes": page,
        "edges": edges,
        "counts": crate::project_discussion_graph_validation::counts(&snapshot.graph, 0),
        "pagination": pagination(projection.offset, projection.limit, total, returned),
        "budget": {"classification_model_tokens":0,"chat_bodies_read":0,"document_bodies_read":0,"metadata_only":true},
    }))
}

fn matches_query(node: &DiscussionNode, query: &str) -> bool {
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
        .contains(query)
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

fn default_limit(value: usize, default: usize) -> usize {
    if value == 0 {
        default
    } else {
        value
    }
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value) -> Result<T> {
    serde_json::from_value(arguments).map_err(Into::into)
}
