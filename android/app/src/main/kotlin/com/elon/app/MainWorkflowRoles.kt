package com.elon.app

internal object MainWorkflowRoles {
    val assistantEvidence = setOf("ai", "ai-intent")
    val staleWorkflow = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool")
    val historyStatus = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete")
    val terminal = setOf("ai", "ai-intent", "error", "ai-stopped")
}
