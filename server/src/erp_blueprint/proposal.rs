use anyhow::{bail, Result};

use super::{
    model::{
        ErpBlueprintDefinition, ErpCapabilityDefinition, RequirementResolution,
        ResolveRequirementRequest,
    },
    validation::stable_need_key,
};

pub(crate) fn resolve_requirement(
    definition: &ErpBlueprintDefinition,
    request: ResolveRequirementRequest,
) -> Result<RequirementResolution> {
    if !matches!(
        request.expected_scope.as_deref(),
        None | Some("merchant_specific") | Some("potential_common")
    ) {
        bail!("expected_scope 只能是 merchant_specific 或 potential_common");
    }
    let requirement = request.requirement.trim();
    if requirement.chars().count() < 4 || requirement.chars().count() > 500 {
        bail!("需求描述必须在 4 到 500 个字符之间");
    }
    let scored_matches = scored_capabilities(definition, requirement, 8);
    let strong_match_count = scored_matches
        .iter()
        .filter(|(_, strong, _)| *strong)
        .count();
    let matched_capabilities = scored_matches
        .into_iter()
        .filter(|(_, strong, _)| strong_match_count == 0 || *strong)
        .map(|(_, _, capability)| capability)
        .collect();
    let (classification, recommendation, need_key, may_submit_signal) = if strong_match_count == 1 {
        (
            "existing",
            "优先复用已存在的蓝图能力，不创建新公共功能。",
            None,
            false,
        )
    } else if strong_match_count > 1 {
        (
            "composition",
            "使用现有能力编排商户项目内的工作流，先验证组合是否足够。",
            None,
            false,
        )
    } else if request.expected_scope.as_deref() == Some("potential_common") {
        (
            "candidate_common",
            "当前目录没有匹配能力；获得商户明确授权后，可提交脱敏通用需求信号。",
            Some(stable_need_key(requirement)),
            true,
        )
    } else {
        (
            "private_extension",
            "当前目录没有匹配能力，先在该商户项目的私有扩展命名空间实现。",
            None,
            false,
        )
    };
    Ok(RequirementResolution {
        schema: "yilong.erp.requirement_resolution.v1",
        classification: classification.to_string(),
        requirement: requirement.to_string(),
        matched_capabilities,
        need_key,
        recommendation: recommendation.to_string(),
        may_submit_signal,
        catalog_version: None,
    })
}

pub(crate) fn search_capabilities(
    definition: &ErpBlueprintDefinition,
    query: &str,
    limit: usize,
) -> Vec<ErpCapabilityDefinition> {
    scored_capabilities(definition, query, limit)
        .into_iter()
        .map(|(_, _, capability)| capability)
        .collect()
}

fn scored_capabilities(
    definition: &ErpBlueprintDefinition,
    query: &str,
    limit: usize,
) -> Vec<(i64, bool, ErpCapabilityDefinition)> {
    let normalized = normalize_search_text(query);
    let mut scored: Vec<_> = definition
        .capabilities
        .iter()
        .filter_map(|capability| {
            let key_score = match_score(&normalized, &capability.capability_key);
            let display_score = match_score(&normalized, &capability.display_name);
            let description_score = match_score(&normalized, &capability.description);
            let alias_scores = capability
                .aliases
                .iter()
                .map(|alias| match_score(&normalized, alias))
                .collect::<Vec<_>>();
            let score = key_score * 4
                + display_score * 3
                + description_score
                + alias_scores.iter().sum::<i64>() * 2;
            let strong = key_score >= 3
                || display_score >= 3
                || description_score >= 3
                || alias_scores.iter().any(|score| *score >= 3);
            (score > 0).then_some((score, strong, capability.clone()))
        })
        .collect();
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.capability_key.cmp(&right.2.capability_key))
    });
    scored.into_iter().take(limit.clamp(1, 100)).collect()
}

fn match_score(query: &str, candidate: &str) -> i64 {
    let candidate = normalize_search_text(candidate);
    if candidate.is_empty() {
        return 0;
    }
    if query == candidate {
        return 5;
    }
    if query.contains(&candidate) || candidate.contains(query) {
        return 3;
    }
    let query_tokens: Vec<_> = query.split_whitespace().collect();
    let candidate_tokens: Vec<_> = candidate.split_whitespace().collect();
    query_tokens
        .iter()
        .filter(|token| candidate_tokens.contains(token))
        .count() as i64
}

fn normalize_search_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_punctuation() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erp_blueprint::model::*;

    fn definition() -> ErpBlueprintDefinition {
        ErpBlueprintDefinition {
            schema: BLUEPRINT_SCHEMA.into(),
            blueprint_key: "official.erp".into(),
            name: "Official ERP".into(),
            description: String::new(),
            source_project_id: "project".into(),
            modules: vec![],
            capabilities: vec![ErpCapabilityDefinition {
                capability_key: "inventory.stock_query".into(),
                display_name: "库存查询".into(),
                description: "查询商品当前库存".into(),
                category: "inventory".into(),
                module_key: "inventory".into(),
                aliases: vec!["查库存".into()],
                composable_with: vec![],
            }],
            themes: vec!["default".into()],
            extension_points: vec![],
            proposal_threshold: 3,
        }
    }

    #[test]
    fn existing_capability_wins_before_common_proposal() {
        let result = resolve_requirement(
            &definition(),
            ResolveRequirementRequest {
                instance_id: None,
                requirement: "帮我查库存".into(),
                expected_scope: Some("potential_common".into()),
            },
        )
        .unwrap();
        assert_eq!(result.classification, "existing");
        assert!(!result.may_submit_signal);
    }

    #[test]
    fn weak_token_overlap_does_not_claim_the_capability_already_exists() {
        let mut definition = definition();
        definition.capabilities.push(ErpCapabilityDefinition {
            capability_key: "inventory.audit".into(),
            display_name: "Inventory audit".into(),
            description: "Review inventory records".into(),
            category: "inventory".into(),
            module_key: "inventory".into(),
            aliases: vec![],
            composable_with: vec![],
        });
        let result = resolve_requirement(
            &definition,
            ResolveRequirementRequest {
                instance_id: None,
                requirement: "inventory customer forecast".into(),
                expected_scope: Some("potential_common".into()),
            },
        )
        .unwrap();
        assert_eq!(result.classification, "candidate_common");
        assert!(result.may_submit_signal);
    }

    #[test]
    fn unknown_scope_is_rejected_instead_of_silently_becoming_private() {
        let error = resolve_requirement(
            &definition(),
            ResolveRequirementRequest {
                instance_id: None,
                requirement: "查询库存".into(),
                expected_scope: Some("global".into()),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("expected_scope"));
    }
}
