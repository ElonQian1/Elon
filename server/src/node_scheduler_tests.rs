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
