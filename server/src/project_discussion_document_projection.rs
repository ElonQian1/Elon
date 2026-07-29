//! Safe projection of accepted discussion nodes into maintainable Markdown.

use anyhow::{bail, Result};
use chrono::Utc;

use crate::project_discussion_graph_model::{DiscussionGraph, DiscussionNode, DiscussionPromotion};

pub(crate) fn validate_promotion_readiness(
    graph: &DiscussionGraph,
    promotion: &DiscussionPromotion,
) -> Result<()> {
    let node = promotion_node(graph, promotion)?;
    if !matches!(node.status.as_str(), "accepted" | "implemented") {
        bail!(
            "讨论节点“{}”仍为 {}，只有 accepted 或 implemented 节点可以晋升",
            node.title,
            node.status
        );
    }
    if node.source_refs.is_empty() {
        bail!("讨论节点“{}”缺少来源锚点，不能晋升为文档", node.title);
    }
    if promotion.content.trim().is_empty() {
        bail!("晋升文档“{}”正文不能为空", promotion.title);
    }
    Ok(())
}

pub(crate) fn render_promotion(
    graph: &DiscussionGraph,
    promotion: &DiscussionPromotion,
) -> Result<String> {
    validate_promotion_readiness(graph, promotion)?;
    let node = promotion_node(graph, promotion)?;
    let role = document_role(promotion, node);
    let authority = if node.status == "implemented" {
        "current"
    } else {
        "accepted"
    };
    let title = serde_json::to_string(&promotion.title)?;
    let mut frontmatter = vec![
        "---".to_string(),
        format!("title: {title}"),
        "owner: project".to_string(),
        format!("reviewed_at: {}", Utc::now().format("%Y-%m-%d")),
        "review_interval_days: 90".to_string(),
        format!("role: {role}"),
        "lifecycle: active".to_string(),
        format!("authority: {authority}"),
        "default_retrieval: true".to_string(),
        "source_discussion_nodes:".to_string(),
        format!("  - {}", yaml_string(&node.id)?),
        "source_refs:".to_string(),
    ];
    frontmatter.extend(
        node.source_refs
            .iter()
            .map(|reference| yaml_string(reference).map(|value| format!("  - {value}")))
            .collect::<Result<Vec<_>>>()?,
    );
    if !node.feature_node_ids.is_empty() {
        frontmatter.push("implementation_refs:".to_string());
        frontmatter.extend(
            node.feature_node_ids
                .iter()
                .map(|id| yaml_string(&format!("feature:{id}")).map(|value| format!("  - {value}")))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    frontmatter.push("---".to_string());
    let body = strip_frontmatter(&promotion.content).trim();
    let heading = if body.lines().any(|line| line.trim_start().starts_with("# ")) {
        String::new()
    } else {
        format!("# {}\n\n", promotion.title)
    };
    Ok(format!("{}\n\n{heading}{body}\n", frontmatter.join("\n")))
}

pub(crate) fn link_promotions_to_graph(
    graph: &mut DiscussionGraph,
    promotions: &[DiscussionPromotion],
) {
    for promotion in promotions {
        let Some(node) = graph
            .nodes
            .iter_mut()
            .find(|node| node.id == promotion.node_id)
        else {
            continue;
        };
        if !node.document_paths.contains(&promotion.path) {
            node.document_paths.push(promotion.path.clone());
            node.document_paths.sort();
        }
    }
}

fn promotion_node<'a>(
    graph: &'a DiscussionGraph,
    promotion: &DiscussionPromotion,
) -> Result<&'a DiscussionNode> {
    graph
        .nodes
        .iter()
        .find(|node| node.id == promotion.node_id)
        .ok_or_else(|| anyhow::anyhow!("晋升操作引用了不存在的节点：{}", promotion.node_id))
}

fn document_role(promotion: &DiscussionPromotion, node: &DiscussionNode) -> &'static str {
    match promotion.document_type.as_str() {
        "decision" => "decision",
        "architecture" => "architecture",
        "requirement" | "requirements" => "requirement",
        "guide" | "runbook" => "guide",
        "result" | "evidence" => "evidence",
        _ => match node.kind.as_str() {
            "decision" => "decision",
            "requirement" => "requirement",
            "result" | "evidence" => "evidence",
            "task" => "task",
            _ => "reference",
        },
    }
}

fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---\n") {
        return content;
    }
    let rest = &content[4..];
    rest.find("\n---\n")
        .map(|index| &rest[index + 5..])
        .unwrap_or(content)
}

fn yaml_string(value: &str) -> Result<String> {
    serde_json::to_string(value).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_discussion_graph_model::DiscussionNode;

    fn accepted_graph() -> DiscussionGraph {
        DiscussionGraph {
            nodes: vec![DiscussionNode {
                id: "merchant-ai".into(),
                kind: "feature".into(),
                title: "商户 AI".into(),
                status: "accepted".into(),
                authority: "accepted".into(),
                source_refs: vec!["conversation-one#turn-0003".into()],
                feature_node_ids: vec!["merchant-ai-runtime".into()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn promotion() -> DiscussionPromotion {
        DiscussionPromotion {
            id: "merchant-ai-doc".into(),
            node_id: "merchant-ai".into(),
            path: "docs/requirements/merchant-ai.md".into(),
            title: "商户 AI 需求".into(),
            content: "# 商户 AI 需求\n\n实现经营自动化。".into(),
            document_type: "requirement".into(),
            ..Default::default()
        }
    }

    #[test]
    fn renders_traceable_authoritative_frontmatter() {
        let rendered = render_promotion(&accepted_graph(), &promotion()).unwrap();
        assert!(rendered.contains("authority: accepted"));
        assert!(rendered.contains("source_discussion_nodes:"));
        assert!(rendered.contains("\"conversation-one#turn-0003\""));
        assert!(rendered.contains("\"feature:merchant-ai-runtime\""));
    }

    #[test]
    fn rejects_unconfirmed_nodes() {
        let mut graph = accepted_graph();
        graph.nodes[0].status = "exploring".into();
        assert!(validate_promotion_readiness(&graph, &promotion()).is_err());
    }

    #[test]
    fn links_promoted_document_back_to_node() {
        let mut graph = accepted_graph();
        link_promotions_to_graph(&mut graph, &[promotion()]);
        assert_eq!(
            graph.nodes[0].document_paths,
            vec!["docs/requirements/merchant-ai.md"]
        );
    }
}
