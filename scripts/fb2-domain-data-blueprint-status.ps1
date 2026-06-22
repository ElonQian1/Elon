#requires -Version 7.0

function New-Fb2DomainDataLane {
    param(
        [string]$Id,
        [string]$UserNeed,
        [string[]]$ContextSections,
        [string[]]$SourceKinds,
        [string[]]$PrimaryTools,
        [string]$PermissionScope,
        [string[]]$AnswerLayers,
        [string[]]$ForbiddenOutputs,
        [string[]]$FutureIndexes
    )

    [ordered]@{
        id = $Id
        user_need = $UserNeed
        context_sections = @($ContextSections)
        source_kinds = @($SourceKinds)
        primary_tools = @($PrimaryTools)
        permission_scope = $PermissionScope
        answer_layers = @($AnswerLayers)
        forbidden_outputs = @($ForbiddenOutputs)
        future_indexes = @($FutureIndexes)
    }
}

function Get-Fb2DomainDataBlueprintState {
    $lanes = @(
        New-Fb2DomainDataLane `
            -Id "match_facts_and_odds" `
            -UserNeed "今天比赛怎么看 / 这场赔率怎么变" `
            -ContextSections @("match_facts", "retrieval_evidence") `
            -SourceKinds @("match", "odds", "context_audit") `
            -PrimaryTools @("match_analysis_brief", "search_matches", "get_match_detail") `
            -PermissionScope "group_context" `
            -AnswerLayers @("match_facts", "odds_facts", "ai_inference", "risk_boundary") `
            -ForbiddenOutputs @("fabricated_odds", "guaranteed_win") `
            -FutureIndexes @("match_index", "odds_snapshot_index")
        New-Fb2DomainDataLane `
            -Id "current_user_tickets" `
            -UserNeed "帮我分析我的票 / 我的订单风险" `
            -ContextSections @("user_order_slice", "match_facts", "retrieval_evidence") `
            -SourceKinds @("user_order", "ticket", "match", "odds", "context_audit") `
            -PrimaryTools @("match_analysis_brief", "search_user_orders", "get_order_detail") `
            -PermissionScope "current_user_only" `
            -AnswerLayers @("current_user_orders", "match_facts", "ai_inference", "risk_boundary") `
            -ForbiddenOutputs @("other_user_order_detail", "guaranteed_win") `
            -FutureIndexes @("order_risk_index", "ticket_result_review_index")
        New-Fb2DomainDataLane `
            -Id "platform_order_summary" `
            -UserNeed "平台今天订单风险怎么样" `
            -ContextSections @("platform_order_summary", "retrieval_evidence") `
            -SourceKinds @("platform_order_summary", "context_audit") `
            -PrimaryTools @("platform_orders") `
            -PermissionScope "privileged_anonymous_summary" `
            -AnswerLayers @("platform_aggregate", "ai_inference", "risk_boundary") `
            -ForbiddenOutputs @("single_user_order_detail", "user_identity_leak") `
            -FutureIndexes @("platform_order_risk_index")
        New-Fb2DomainDataLane `
            -Id "group_opinions" `
            -UserNeed "群里大家怎么看这场 / 总结群聊观点" `
            -ContextSections @("group_opinion_slice", "match_facts", "retrieval_evidence") `
            -SourceKinds @("group_message", "opinion_memory", "match", "context_audit") `
            -PrimaryTools @("group_opinion_summary", "search_group_opinions", "opinion_memories") `
            -PermissionScope "single_group_context" `
            -AnswerLayers @("group_opinion", "match_facts", "ai_inference", "risk_boundary") `
            -ForbiddenOutputs @("group_opinion_as_fact", "fabricated_group_view") `
            -FutureIndexes @("group_opinion_index", "opinion_memory_index")
        New-Fb2DomainDataLane `
            -Id "opinion_learning_loop" `
            -UserNeed "采纳用户观点并持续复盘，让群聊分析逐步进化" `
            -ContextSections @("quality_feedback", "group_opinion_slice") `
            -SourceKinds @("opinion_memory", "feedback", "opinion_adoption") `
            -PrimaryTools @("list_opinion_adoptions", "opinion_adoption_summary", "opinion_result_reviews", "opinion_result_review_summary") `
            -PermissionScope "single_group_quality_history" `
            -AnswerLayers @("opinion_history", "quality_signal", "ai_inference", "risk_boundary") `
            -ForbiddenOutputs @("quality_history_as_match_fact", "uncited_opinion_memory") `
            -FutureIndexes @("opinion_adoption_index", "opinion_result_review_index")
        New-Fb2DomainDataLane `
            -Id "quality_feedback_audit" `
            -UserNeed "回答有没有引用错来源 / 哪些失败样本需要改进" `
            -ContextSections @("quality_feedback", "retrieval_evidence") `
            -SourceKinds @("context_audit", "feedback", "opinion_adoption") `
            -PrimaryTools @("get_context_audit", "context_audit_summary", "list_context_feedbacks") `
            -PermissionScope "audit_metadata_only" `
            -AnswerLayers @("source_registry", "data_fact_boundary", "quality_feedback") `
            -ForbiddenOutputs @("uncited_source", "fabricated_source") `
            -FutureIndexes @("context_audit_index", "feedback_quality_index")
    )

    [ordered]@{
        schema = "fb2.main_project.domain_data_blueprint.v1"
        complete = $true
        context_format = "xml_wrapped_markdown_context_pack_with_json_metadata"
        first_phase_delivery = "rest_context_pack_plus_tool_manifest_plus_tools_execute"
        mcp_status = "future_wrapper_not_first_phase_fact_source"
        source_of_truth = "fb2_backend_live_business_data"
        stores_fb2_business_data_in_main_project = $false
        required_context_pack_sections = @(
            "usage_boundary",
            "match_facts",
            "user_order_slice",
            "platform_order_summary",
            "group_opinion_slice",
            "retrieval_evidence",
            "quality_feedback"
        )
        required_metadata = @(
            "context_pack_version",
            "generated_at",
            "context_audit_id",
            "citation_sources",
            "metrics",
            "tool_contract",
            "usage_policy",
            "answer_policy",
            "preflight_readiness"
        )
        lane_count = @($lanes).Count
        lanes = @($lanes)
        anti_patterns = @(
            "raw_html_prompt",
            "giant_json_prompt",
            "full_database_dump",
            "raw_embedding_dump",
            "uncited_odds",
            "uncited_order",
            "platform_order_detail_leak"
        )
        next_evolution = @(
            "keep REST Context Pack as the AI-facing payload",
            "add fb2-side domain indexes for faster retrieval",
            "wrap existing REST/tool contracts with MCP later only if it preserves permissions and audit"
        )
    }
}
