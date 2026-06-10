//! 在线节点自动调度：按执行质量、价格和硬件画像选择默认节点。

use std::collections::HashMap;

use crate::{node_registry::NodeSummary, store::NodeQualityScore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeScheduleDecision {
    pub node_id: String,
    pub score: i64,
    pub route_reason: String,
}

pub fn select_best_node(
    candidates: &[NodeSummary],
    quality_by_node: &HashMap<String, NodeQualityScore>,
    model_id: &str,
) -> Option<NodeScheduleDecision> {
    let mut decisions = candidates
        .iter()
        .filter(|candidate| candidate.online)
        .filter(|candidate| {
            candidate
                .models
                .iter()
                .any(|model| model.model_id == model_id)
        })
        .map(|candidate| {
            score_candidate(candidate, quality_by_node.get(&candidate.node_id), model_id)
        })
        .collect::<Vec<_>>();

    decisions.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    decisions.into_iter().next()
}

fn score_candidate(
    candidate: &NodeSummary,
    quality: Option<&NodeQualityScore>,
    model_id: &str,
) -> NodeScheduleDecision {
    let price = candidate
        .models
        .iter()
        .find(|model| model.model_id == model_id)
        .map(|model| model.price_per_1k_credits.max(0.0))
        .unwrap_or(0.0);
    let price_penalty = (price * 100.0).round().clamp(0.0, 10_000.0) as i64;
    let hardware_bonus = hardware_bonus(candidate);

    let (history_score, reason) = if let Some(quality) = quality {
        let avg_ms_penalty = quality
            .avg_duration_ms
            .map(|value| (value / 50).clamp(0, 6_000))
            .unwrap_or(0);
        let failed_penalty = quality.failed_runs.clamp(0, 50) * 250;
        let history_score =
            (quality.success_rate_x1000.clamp(0, 1000) * 10) - avg_ms_penalty - failed_penalty;
        (
            history_score,
            format!(
                "auto_quality success_rate={}‰ failed={} avg_ms={} price={:.4}",
                quality.success_rate_x1000,
                quality.failed_runs,
                quality
                    .avg_duration_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                price
            ),
        )
    } else {
        (6_000, format!("auto_quality cold_start price={price:.4}"))
    };

    let score = 10_000 + history_score + hardware_bonus - price_penalty;
    NodeScheduleDecision {
        node_id: candidate.node_id.clone(),
        score,
        route_reason: format!("{reason} score={score}"),
    }
}

fn hardware_bonus(candidate: &NodeSummary) -> i64 {
    let Some(hardware) = &candidate.hardware else {
        return 0;
    };
    let mut bonus = 0;
    if !hardware.gpu_names.is_empty() {
        bonus += 500;
    }
    if let Some(bytes) = hardware.gpu_memory_total_bytes {
        let gib = bytes / 1024 / 1024 / 1024;
        bonus += (gib as i64 * 40).clamp(0, 1_000);
    }
    if let Some(cores) = hardware.cpu_cores {
        bonus += (cores as i64 * 10).clamp(0, 320);
    }
    bonus
}

#[cfg(test)]
mod tests {
    use super::*;
    use homecli_proto::{ModelCapability, NodeHardwareProfile};

    fn model(price: f64) -> ModelCapability {
        ModelCapability {
            model_id: "qwen".to_string(),
            display_name: "qwen".to_string(),
            context_len: 4096,
            provider: "test".to_string(),
            price_per_1k_credits: price,
        }
    }

    fn candidate(node_id: &str, price: f64, gpu_bytes: Option<u64>) -> NodeSummary {
        NodeSummary {
            node_id: node_id.to_string(),
            owner_user_id: format!("owner-{node_id}"),
            device_name: None,
            hardware: gpu_bytes.map(|bytes| NodeHardwareProfile {
                gpu_names: vec!["GPU".to_string()],
                gpu_memory_total_bytes: Some(bytes),
                ..Default::default()
            }),
            models: vec![model(price)],
            tts_worker_url: None,
            connected_at: 1,
            online: true,
        }
    }

    #[test]
    fn prefers_stable_node_over_cheaper_failed_node() {
        let candidates = vec![
            candidate("unstable", 0.01, None),
            candidate("stable", 0.10, None),
        ];
        let quality = HashMap::from([
            (
                "unstable".to_string(),
                NodeQualityScore {
                    node_id: "unstable".to_string(),
                    total_runs: 10,
                    successful_runs: 5,
                    failed_runs: 5,
                    avg_duration_ms: Some(700),
                    success_rate_x1000: 500,
                    ..Default::default()
                },
            ),
            (
                "stable".to_string(),
                NodeQualityScore {
                    node_id: "stable".to_string(),
                    total_runs: 10,
                    successful_runs: 10,
                    failed_runs: 0,
                    avg_duration_ms: Some(900),
                    success_rate_x1000: 1000,
                    ..Default::default()
                },
            ),
        ]);

        let picked = select_best_node(&candidates, &quality, "qwen").unwrap();
        assert_eq!(picked.node_id, "stable");
    }

    #[test]
    fn keeps_cold_start_nodes_eligible() {
        let candidates = vec![
            candidate("cold-a", 0.10, Some(8 * 1024 * 1024 * 1024)),
            candidate("cold-b", 0.10, None),
        ];
        let picked = select_best_node(&candidates, &HashMap::new(), "qwen").unwrap();
        assert_eq!(picked.node_id, "cold-a");
        assert!(picked.route_reason.contains("cold_start"));
    }
}
