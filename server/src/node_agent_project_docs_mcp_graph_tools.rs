//! Low-token knowledge-map tools shared by every MCP-capable AI provider.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_federation_service::get_federation_index,
    project_document_knowledge_graph_service::{get_map, get_node, plan_context, review_map},
    project_document_response::ProjectionRequest,
};

#[derive(Debug, Deserialize)]
struct MapArguments {
    #[serde(default = "default_view")]
    view: String,
    #[serde(default)]
    root_id: Option<String>,
    #[serde(default = "default_depth")]
    depth: usize,
    #[serde(default)]
    query: Option<String>,
    #[serde(default = "default_max_nodes")]
    max_nodes: usize,
}

#[derive(Debug, Deserialize)]
struct NodeArguments {
    node_id: String,
}

#[derive(Debug, Deserialize)]
struct ReviewArguments {
    #[serde(default = "default_view")]
    view: String,
}

#[derive(Debug, Deserialize)]
struct ContextArguments {
    #[serde(default)]
    query: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default = "default_token_budget")]
    max_tokens: u64,
    #[serde(default = "default_document_limit")]
    max_documents: usize,
    #[serde(default = "default_rule_token_budget")]
    max_rule_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct FederationArguments {
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    query: Option<String>,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "project_docs_get_map",
            "读取统一项目知识图谱的紧凑局部视图，不读取 Markdown 正文。overview 只返回三种视图摘要；capabilities 表示用户能力，architecture 表示真实技术组件，topics 表示文档主题。支持 root/depth/query/max_nodes 限界。",
            json!({"type":"object","properties":{
                "view":{"type":"string","enum":["overview","capabilities","architecture","topics"],"default":"capabilities"},
                "root_id":{"type":"string","description":"可选局部根节点；先用 overview 或默认视图发现 id。"},
                "depth":{"type":"integer","minimum":1,"maximum":6,"default":2},
                "query":{"type":"string","maxLength":200},
                "max_nodes":{"type":"integer","minimum":1,"maximum":200,"default":80},
                "offset":{"type":"integer","minimum":0,"default":0},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                "projection":{"type":"string","enum":["summary","page","detail","full"],"default":"page"}
            }}),
        ),
        tool(
            "project_docs_get_node",
            "读取一个图谱节点的文档、实现证据、关系和确定性缺口；只返回元数据，不读取正文。",
            json!({"type":"object","required":["node_id"],"properties":{
                "node_id":{"type":"string","minLength":1,"maxLength":100}
            }}),
        ),
        tool(
            "project_docs_review_map",
            "对指定功能图、技术架构图或主题图返回确定性结构诊断、评审问题和建议落点。AI 应基于这些证据判断结构是否合理，再把变更写入 proposed_knowledge_graph。",
            json!({"type":"object","properties":{
                "view":{"type":"string","enum":["capabilities","architecture","topics"],"default":"capabilities"}
            }}),
        ),
        tool(
            "project_docs_plan_context",
            "根据任务或图谱节点在 token 预算内生成推荐阅读计划，只返回路径、标题层级、权威性和估算 token。先调用本工具，再对少量必要路径调用 project_docs_read。",
            json!({"type":"object","properties":{
                "query":{"type":"string","maxLength":500,"default":""},
                "node_id":{"type":"string","maxLength":100},
                "max_tokens":{"type":"integer","minimum":200,"maximum":12000,"default":2400},
                "max_rule_tokens":{"type":"integer","minimum":200,"maximum":6000,"default":1600,"description":"强制规则独立预算，不占相关正文预算。"},
                "max_documents":{"type":"integer","minimum":1,"maximum":24,"default":8},
                "offset":{"type":"integer","minimum":0,"default":0},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                "projection":{"type":"string","enum":["summary","page","detail","full"],"default":"page"}
            },"anyOf":[{"required":["query"]},{"required":["node_id"]}]}),
        ),
        tool(
            "project_docs_get_federation",
            "分页惰性读取项目→子项目→模块/主题联邦知识索引。默认只返回根或指定 parent_id 的直接子节点，不受旧 16 分区或 500 文档展示上限影响。",
            json!({"type":"object","properties":{
                "parent_id":{"type":"string","maxLength":100,"description":"为空时返回联邦根；展开时传父节点 id。"},
                "query":{"type":"string","maxLength":200},
                "offset":{"type":"integer","minimum":0,"default":0},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                "projection":{"type":"string","enum":["summary","page"],"default":"page"}
            }}),
        ),
    ]
}

pub(crate) fn proposal_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "nodes":{"type":"array","maxItems":256,"items":{
                "type":"object","required":["id","view","label"],"properties":{
                    "id":{"type":"string","pattern":"^[A-Za-z0-9._-]+$","maxLength":80},
                    "view":{"type":"string","enum":["capabilities","architecture"]},
                    "kind":{"type":"string","maxLength":40},
                    "label":{"type":"string","maxLength":60},
                    "detail":{"type":"string","maxLength":240},
                    "parent_id":{"type":"string","maxLength":80,"default":""},
                    "order":{"type":"integer","minimum":0,"maximum":9999,"default":0},
                    "color":{"type":"string","pattern":"^#[0-9A-Fa-f]{6}$"},
                    "entrypoint":{"type":"string","default":""},
                    "document_paths":{"type":"array","maxItems":48,"items":{"type":"string"}},
                    "implementation_refs":{"type":"array","maxItems":48,"items":{"type":"string","maxLength":500}},
                    "tags":{"type":"array","maxItems":24,"items":{"type":"string","maxLength":80}}
                }
            }},
            "edges":{"type":"array","maxItems":512,"items":{
                "type":"object","required":["id","source","target"],"properties":{
                    "id":{"type":"string","pattern":"^[A-Za-z0-9._-]+$","maxLength":100},
                    "source":{"type":"string","maxLength":80},
                    "target":{"type":"string","maxLength":80},
                    "relation":{"type":"string","maxLength":40,"default":"related_to"},
                    "label":{"type":"string","maxLength":80}
                }
            }}
        }
    })
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let projection = ProjectionRequest::from_arguments(&arguments)?;
    let result = match name {
        "project_docs_get_map" => {
            let input: MapArguments = decode(arguments)?;
            get_map(
                workspace,
                &input.view,
                input.root_id.as_deref(),
                input.depth,
                input.query.as_deref(),
                input
                    .max_nodes
                    .max(projection.offset.saturating_add(projection.limit))
                    .min(200),
            )?
        }
        "project_docs_get_node" => {
            let input: NodeArguments = decode(arguments)?;
            get_node(workspace, &input.node_id)?
        }
        "project_docs_review_map" => {
            let input: ReviewArguments = decode(arguments)?;
            review_map(workspace, &input.view)?
        }
        "project_docs_plan_context" => {
            let input: ContextArguments = decode(arguments)?;
            plan_context(
                workspace,
                &input.query,
                input.node_id.as_deref(),
                input.max_tokens,
                input.max_documents,
                input.max_rule_tokens,
            )?
        }
        "project_docs_get_federation" => {
            let input: FederationArguments = decode(arguments)?;
            get_federation_index(
                workspace,
                input.parent_id.as_deref(),
                input.query.as_deref(),
                &projection,
            )?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value) -> Result<T> {
    Ok(serde_json::from_value(arguments)?)
}

fn default_view() -> String {
    "capabilities".to_string()
}
fn default_depth() -> usize {
    2
}
fn default_max_nodes() -> usize {
    80
}
fn default_token_budget() -> u64 {
    2_400
}
fn default_document_limit() -> usize {
    8
}
fn default_rule_token_budget() -> u64 {
    6_000
}
