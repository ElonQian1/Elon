//! 在线节点自动调度：按执行质量、价格和硬件画像选择默认节点。

use std::collections::HashMap;

use homecli_proto::NodeHardwareProfile;
use serde::Serialize;

use crate::{node_registry::NodeSummary, store::NodeQualityScore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeScheduleDecision {
    pub node_id: String,
    pub score: i64,
    pub route_reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PcNodeDispatchScore {
    pub score: i64,
    pub tier: String,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct PcNodeDispatchCandidate<'a> {
    pub online: bool,
    pub cli_connected: bool,
    pub can_accept_project: bool,
    pub project_count: i64,
    pub project_slots_remaining: i64,
    pub route_a_ready: bool,
    pub api_runtime_ready: bool,
    pub server_runtime_ready: bool,
    pub hardware: Option<&'a NodeHardwareProfile>,
    pub quality: Option<&'a NodeQualityScore>,
}

#[allow(dead_code)]
pub fn score_pc_node_dispatch(candidate: PcNodeDispatchCandidate<'_>) -> PcNodeDispatchScore {
    let mut raw_score = 0_i64;
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();

    if candidate.online {
        raw_score += 30;
        reasons.push("节点在线".to_string());
    } else {
        warnings.push("节点离线".to_string());
    }

    if candidate.cli_connected {
        raw_score += 10;
        reasons.push("Win 端控制通道已连接".to_string());
    } else if candidate.online {
        warnings.push("CLI 控制通道未确认".to_string());
    }

    if candidate.route_a_ready {
        raw_score += 20;
        reasons.push("本机 Codex/Copilot/Claude/Gemini 可用".to_string());
    }
    if candidate.api_runtime_ready {
        raw_score += 12;
        reasons.push("本机 API runtime 可用".to_string());
    }
    if candidate.server_runtime_ready {
        raw_score += 8;
        reasons.push("平台 AI runtime 可兜底".to_string());
    }
    if !candidate.route_a_ready && !candidate.api_runtime_ready && !candidate.server_runtime_ready {
        warnings.push("未发现可用 AI runtime".to_string());
    }

    if candidate.can_accept_project {
        raw_score += 12;
        reasons.push("仍可接收项目".to_string());
    } else {
        raw_score -= 25;
        warnings.push("项目容量已满或工作区不可用".to_string());
    }

    if candidate.project_slots_remaining > 0 {
        raw_score += (candidate.project_slots_remaining * 4).clamp(0, 16);
        reasons.push(format!(
            "剩余 {} 个项目槽位",
            candidate.project_slots_remaining
        ));
    } else if candidate.project_count > 0 {
        warnings.push(format!("当前已绑定 {} 个项目", candidate.project_count));
    }

    raw_score += hardware_score(candidate.hardware, &mut reasons);
    raw_score += quality_score(candidate.quality, &mut reasons, &mut warnings);

    let score = raw_score.clamp(0, 100);
    PcNodeDispatchScore {
        score,
        tier: dispatch_tier(score).to_string(),
        reasons,
        warnings,
    }
}

#[allow(dead_code)]
fn hardware_score(hardware: Option<&NodeHardwareProfile>, reasons: &mut Vec<String>) -> i64 {
    let Some(hardware) = hardware else {
        return 0;
    };
    let mut score = 0;
    if let Some(cores) = hardware.cpu_cores {
        let bonus = (cores as i64).clamp(0, 16);
        score += bonus;
        if cores >= 8 {
            reasons.push(format!("CPU {} 核", cores));
        }
    }
    if let Some(bytes) = hardware.memory_total_bytes {
        let gib = bytes / 1024 / 1024 / 1024;
        let bonus = ((gib as i64) / 2).clamp(0, 16);
        score += bonus;
        if gib >= 16 {
            reasons.push(format!("内存 {} GiB", gib));
        }
    }
    if !hardware.gpu_names.is_empty() {
        score += 8;
        reasons.push("检测到 GPU".to_string());
    }
    if let Some(bytes) = hardware.gpu_memory_total_bytes {
        let gib = bytes / 1024 / 1024 / 1024;
        let bonus = (gib as i64).clamp(0, 10);
        score += bonus;
        if gib >= 8 {
            reasons.push(format!("显存 {} GiB", gib));
        }
    }
    score.clamp(0, 30)
}

#[allow(dead_code)]
fn quality_score(
    quality: Option<&NodeQualityScore>,
    reasons: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> i64 {
    let Some(quality) = quality else {
        reasons.push("暂无历史质量，按冷启动候选处理".to_string());
        return 6;
    };
    let success_bonus = (quality.success_rate_x1000.clamp(0, 1000) / 50).clamp(0, 20);
    if quality.success_rate_x1000 >= 900 {
        reasons.push(format!("历史成功率 {}‰", quality.success_rate_x1000));
    } else if quality.total_runs >= 3 {
        warnings.push(format!("历史成功率 {}‰", quality.success_rate_x1000));
    }
    let duration_bonus = quality
        .avg_duration_ms
        .map(|ms| if ms <= 10 * 60 * 1000 { 4 } else { 0 })
        .unwrap_or(2);
    success_bonus + duration_bonus
}

#[allow(dead_code)]
fn dispatch_tier(score: i64) -> &'static str {
    match score {
        80..=100 => "excellent",
        60..=79 => "good",
        35..=59 => "limited",
        _ => "unavailable",
    }
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
            storage: None,
            dev_runtime: None,
            lifecycle: None,
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

    #[test]
    fn pc_dispatch_score_prefers_ready_capacity_and_quality() {
        let hardware = NodeHardwareProfile {
            cpu_cores: Some(12),
            memory_total_bytes: Some(32 * 1024 * 1024 * 1024),
            gpu_names: vec!["GPU".to_string()],
            gpu_memory_total_bytes: Some(8 * 1024 * 1024 * 1024),
            ..Default::default()
        };
        let quality = NodeQualityScore {
            total_runs: 10,
            successful_runs: 10,
            success_rate_x1000: 1000,
            avg_duration_ms: Some(300_000),
            ..Default::default()
        };
        let score = score_pc_node_dispatch(PcNodeDispatchCandidate {
            online: true,
            cli_connected: true,
            can_accept_project: true,
            project_count: 1,
            project_slots_remaining: 3,
            route_a_ready: true,
            api_runtime_ready: false,
            server_runtime_ready: true,
            hardware: Some(&hardware),
            quality: Some(&quality),
        });

        assert_eq!(score.tier, "excellent");
        assert!(score.score >= 80);
        assert!(score
            .reasons
            .iter()
            .any(|reason| reason.contains("历史成功率")));
    }

    #[test]
    fn pc_dispatch_score_marks_offline_without_runtime_unavailable() {
        let score = score_pc_node_dispatch(PcNodeDispatchCandidate {
            online: false,
            cli_connected: false,
            can_accept_project: false,
            project_count: 4,
            project_slots_remaining: 0,
            route_a_ready: false,
            api_runtime_ready: false,
            server_runtime_ready: false,
            hardware: None,
            quality: None,
        });

        assert_eq!(score.tier, "unavailable");
        assert!(score
            .warnings
            .iter()
            .any(|warning| warning.contains("节点离线")));
        assert!(score
            .warnings
            .iter()
            .any(|warning| warning.contains("AI runtime")));
    }
}
