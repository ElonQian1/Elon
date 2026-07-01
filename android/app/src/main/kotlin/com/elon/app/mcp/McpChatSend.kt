package com.elon.app.mcp

import com.elon.app.*
import android.content.Context
import android.content.Intent
import androidx.core.content.ContextCompat
import org.json.JSONObject
import java.util.UUID

/**
 * MCP 调试服务器的 chat_send 工具实现。
 *
 * 从 [McpDebugServer] 拆出。接收 chat_send 的 JSON 参数，
 * 负责构造 trace_id、payload、调度 [TaskWorkService] 启动前台任务，
 * 并在启动确认窗口内回收 task_start 信号。
 */
internal fun chatSend(context: Context, args: JSONObject): JSONObject {
    val message = args.optString("message").trim()
    if (message.isEmpty()) {
        return toolResult("message is required", JSONObject().put("field", "message"), isError = true)
    }
    val prefs = AuthManager.userDataPrefs(context)
    val force = args.optBoolean("force", false)
    val userId = AuthManager.effectiveUserId(context)
    val projectId = args.optString("project_id").takeIf { it.isNotBlank() }
        ?: prefs.getString(TaskWorkService.PREF_ACTIVE_PROJECT_ID, null)
        ?: "elon-self"
    val projectTitle = args.optString("project_title").takeIf { it.isNotBlank() }
        ?: prefs.getString("project_title", null)
        ?: "Elon debug project"
    val traceId = args.optString("trace_id").takeIf { it.isNotBlank() }
        ?: "mcp_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}"
    val agent = args.optString("agent").takeIf { it.isNotBlank() }
    val conversationId = args.optString("conversation_id").takeIf { it.isNotBlank() }
    val conversationTitle = args.optString("conversation_title").takeIf { it.isNotBlank() }
    val isDevelopment = if (args.has("is_development")) args.optBoolean("is_development") else true
    val startAckTimeoutMs = args.optInt("start_ack_timeout_ms", 1_800).coerceIn(0, 10_000)

    val payload = JSONObject()
        .put("trace_id", traceId)
        .put("user_id", userId)
        .put("project_id", projectId)
        .put("project_title", projectTitle)
        .put("message", message)
    if (agent != null) payload.put("agent", agent)
    if (conversationId != null) payload.put("conversation_id", conversationId)
    if (conversationTitle != null) payload.put("conversation_title", conversationTitle)
    val payloadText = payload.toString()

    if (!force) {
        reservePendingTask(prefs, payloadText, isDevelopment)
    }

    DebugTraceStore.record(
        "mcp_chat_send",
        mapOf(
            "trace_id" to traceId,
            "project_id" to projectId,
            "chars" to message.length,
            "reserved_pending" to !force
        )
    )
    val intent = Intent(context, TaskWorkService::class.java).apply {
        action = TaskWorkService.ACTION_START_WORK
        putExtra(TaskWorkService.EXTRA_PAYLOAD, payloadText)
        putExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, isDevelopment)
        putExtra(TaskWorkService.EXTRA_FORCE_START, force)
    }
    val startResult = runCatching {
        ContextCompat.startForegroundService(context, intent)
    }.exceptionOrNull()
    if (startResult != null) {
        if (!force) clearReservedPendingTask(prefs, traceId)
        DebugTraceStore.record(
            "mcp_chat_start_failed",
            mapOf("trace_id" to traceId, "error" to startResult.message)
        )
        return toolResult(
            "Could not start phone task service.",
            JSONObject()
                .put("trace_id", traceId)
                .put("project_id", projectId)
                .put("error", startResult.message ?: startResult.javaClass.simpleName),
            isError = true
        )
    }
    DebugTraceStore.record(
        "mcp_chat_queued",
        mapOf("trace_id" to traceId, "project_id" to projectId, "force" to force)
    )
    var serviceStart = waitForTaskStartSignal(traceId, startAckTimeoutMs)
    if (!serviceStart.optBoolean("confirmed", false) && startAckTimeoutMs > 0) {
        DebugTraceStore.record(
            "mcp_chat_start_unconfirmed",
            mapOf(
                "trace_id" to traceId,
                "timeout_ms" to startAckTimeoutMs,
                "last_phase" to serviceStart.optString("last_phase").takeIf { it.isNotBlank() }
            )
        )
        val resumeIntent = Intent(context, TaskWorkService::class.java).apply {
            action = TaskWorkService.ACTION_RESUME_PENDING
        }
        val fallbackError = runCatching {
            ContextCompat.startForegroundService(context, resumeIntent)
        }.exceptionOrNull()
        if (fallbackError != null) {
            DebugTraceStore.record(
                "mcp_chat_start_fallback_failed",
                mapOf("trace_id" to traceId, "error" to fallbackError.message)
            )
            serviceStart = serviceStart
                .put("fallback_attempted", true)
                .put("fallback_error", fallbackError.message ?: fallbackError.javaClass.simpleName)
        } else {
            DebugTraceStore.record("mcp_chat_start_fallback_resume", mapOf("trace_id" to traceId))
            val fallback = waitForTaskStartSignal(traceId, startAckTimeoutMs)
                .put("fallback_attempted", true)
            serviceStart = fallback.put("initial", serviceStart)
        }
    }

    val structured = JSONObject()
        .put("trace_id", traceId)
        .put("project_id", projectId)
        .put("project_title", projectTitle)
        .put("conversation_id", conversationId ?: JSONObject.NULL)
        .put("is_development", isDevelopment)
        .put("force", force)
        .put("message_chars", message.length)
        .put("service_start", serviceStart)
    return toolResult("Chat request queued on phone.", structured)
}
