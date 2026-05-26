package com.elon.app.mcp

import com.elon.app.*
import android.content.Context
import android.content.Intent
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale

internal fun mcpTaskStatus(context: Context, args: JSONObject): JSONObject {
    return toolResult("Task status returned.", taskStatusJson(context, args))
}

internal fun mcpTaskControl(context: Context, args: JSONObject): JSONObject {
    val action = args.optString("action", "status").lowercase(Locale.ROOT)
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    when (action) {
        "pause" -> {
            val activeTraceId = pendingTaskJson(prefs)?.optString("trace_id")?.takeIf { it.isNotBlank() }
            clearPersistedTask(prefs)
            val serviceIntent = Intent(context, TaskWorkService::class.java).apply {
                this.action = TaskWorkService.ACTION_PAUSE
            }
            val serviceSignal = runCatching { context.startService(serviceIntent) }
                .fold(onSuccess = { "pause_sent" }, onFailure = { "pause_start_failed:${it.javaClass.simpleName}" })
            val stopped = if (serviceSignal.startsWith("pause_start_failed")) {
                runCatching { context.stopService(Intent(context, TaskWorkService::class.java)) }
                    .getOrDefault(false)
            } else {
                false
            }
            DebugTraceStore.record(
                "mcp_task_control",
                mapOf(
                    "action" to action,
                    "active_trace_id" to activeTraceId,
                    "service_signal" to serviceSignal,
                    "stop_service" to stopped
                )
            )
            return toolResult(
                "Task pause requested.",
                taskStatusJson(context, JSONObject())
                    .put("action", action)
                    .put("service_signal", serviceSignal)
                    .put("stop_service", stopped)
            )
        }
        "status" -> Unit
        else -> return toolResult(
            "Unsupported task control action: $action",
            JSONObject().put("action", action),
            isError = true
        )
    }
    return toolResult("Task status returned.", taskStatusJson(context, JSONObject()).put("action", action))
}
internal fun mcpTaskEvents(context: Context, args: JSONObject): JSONObject {
    val limit = args.optInt("limit", 40).coerceIn(1, 120)
    val clear = args.optBoolean("clear", false)
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    val allEvents = queuedTaskEvents(prefs)
    val returned = allEvents.takeLast(limit)
    if (clear) {
        prefs.edit().remove(TaskWorkService.PREF_QUEUED_TASK_EVENTS).apply()
        DebugTraceStore.record("mcp_task_events_cleared", mapOf("cleared_count" to allEvents.size))
    }
    val structured = JSONObject()
        .put("events", JSONArray().apply { returned.forEach { put(rawTaskEventJson(it)) } })
        .put("returned_count", returned.size)
        .put("queued_count", allEvents.size)
        .put("limit", limit)
        .put("cleared", clear)
    return toolResult("Queued task events returned.", structured)
}
