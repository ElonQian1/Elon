package com.elon.app

import com.google.gson.JsonObject

internal class MainAssistantStreamEvents(
    private val handleTaskEvent: (String, String?, String) -> Unit,
    private val maybeAppendTaskEventNarrative: (String, String) -> Boolean,
    private val maybeAppendWorkflowProgressNarrative: (String) -> Boolean,
    private val maybeAppendToolCallNarrative: (String) -> Boolean,
    private val handleProgress: (String, Boolean) -> Unit,
    private val handleFoldedCliOutput: (String) -> Unit,
    private val markToolCallStarted: (String) -> Unit,
    private val markToolResult: (String) -> Unit,
    private val recordEvidence: (String, String) -> Unit,
    private val isDevelopmentRequest: () -> Boolean,
    private val addProjectEvent: (String) -> Unit
) {
    fun taskEventMessage(json: JsonObject): ChatMessage? {
        val event = jsonStringOrNull(json, "event").orEmpty()
        val taskId = jsonStringOrNull(json, "task_id")
        val content = jsonStringOrNull(json, "message").orEmpty()
        handleTaskEvent(event, taskId, content)
        if (maybeAppendTaskEventNarrative(event, content)) return null
        return if ((event == "accepted" || event == "runtime_note_received") && shouldShowProgressBubble(content)) {
            ChatMessage("ai-progress", workflowProgressMessage(content))
        } else {
            null
        }
    }

    fun progressMessage(json: JsonObject): ChatMessage? {
        val content = jsonStringOrNull(json, "message") ?: ""
        if (isCliOutputProgress(content)) {
            handleFoldedCliOutput(content)
            return null
        }
        val surfaced = maybeAppendWorkflowProgressNarrative(content)
        handleProgress(content, !surfaced)
        if (surfaced) return null
        return if (shouldShowProgressBubble(content)) {
            ChatMessage("ai-progress", workflowProgressMessage(content))
        } else {
            null
        }
    }

    fun handleToolCall(json: JsonObject) {
        val tool = jsonStringOrNull(json, "tool") ?: "工具"
        maybeAppendToolCallNarrative(tool)
        markToolCallStarted(tool)
    }

    fun handleToolResult(json: JsonObject) {
        val tool = jsonStringOrNull(json, "tool") ?: "工具"
        val result = jsonStringOrNull(json, "result").orEmpty()
        val evidence = if (result.isBlank()) {
            "完成：${toolLabel(tool)}"
        } else {
            "完成：${toolLabel(tool)}，${summarize(result, 80)}"
        }
        recordEvidence(toolEvidenceKind(tool), evidence)
        markToolResult(tool)
    }

    fun assistantMessage(json: JsonObject): ChatMessage? {
        val text = jsonStringOrNull(json, "text").orEmpty().trim()
        if (text.isBlank()) return null
        if (isDevelopmentRequest()) {
            addProjectEvent("AI 说明：${summarize(text, 36)}")
        }
        val modelUsed = jsonStringOrNull(json, "model_used")
        val streamId = jsonStringOrNull(json, "stream_id")
        // Copilot 有时会把工具块作为第一条 assistant_message 发出
        // 如果 text 以 ● 开头，直接放进 evidenceDetails，不显示在主气泡
        val isCopilotMsg = modelUsed?.startsWith("node-") == true
        val isCopilotToolBlock = isCopilotMsg && text.startsWith("● ")
        return if (isCopilotToolBlock) {
            ChatMessage(
                "ai-intent",
                content = "",
                modelUsed = modelUsed,
                streamId = streamId,
                evidenceTitle = "执行详情",
                evidenceDetails = text
            )
        } else {
            ChatMessage("ai-intent", text, modelUsed = modelUsed, streamId = streamId)
        }
    }
}
