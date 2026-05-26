package com.elon.app.mcp

import com.elon.app.*
import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

internal fun mcpSelfCheckTool(
    context: Context,
    args: JSONObject,
    running: Boolean,
    lastError: String?,
    status: JSONObject,
    metrics: JSONObject,
    updateStatus: () -> Any
): JSONObject {
    val includeUpdateCheck = args.optBoolean("include_update_check", false)
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    val taskStatus = taskStatusJson(context, JSONObject())
    val keepalive = McpDebugKeepAliveService.statusJson(context)
    val backgroundRuntime = backgroundDebugStatusJson(context)
    val queuedEvents = queuedTaskEvents(prefs)
    val checks = JSONArray()
    fun addCheck(name: String, ok: Boolean, detail: String, critical: Boolean = true) {
        checks.put(
            JSONObject()
                .put("name", name)
                .put("ok", ok)
                .put("critical", critical)
                .put("detail", detail)
        )
    }

    val appForeground = status.optBoolean("app_foreground", false)
    val keepaliveActive = keepalive.optBoolean("active", false)
    addCheck("mcp_server_running", running, "MCP server thread is accepting local HTTP requests.")
    addCheck(
        "background_keepalive",
        appForeground || keepaliveActive,
        if (appForeground) "App is foreground; keepalive is optional." else "Foreground keepalive must be active while app is background."
    )
    addCheck("trace_buffer", DebugTraceStore.count() > 0, "Trace buffer has ${DebugTraceStore.count()} persisted events.", critical = false)
    addCheck(
        "request_health",
        lastError == null,
        lastError?.let { "Last MCP request error: $it" } ?: "No MCP request error recorded in this process."
    )
    addCheck(
        "task_queue",
        queuedEvents.size <= 120,
        "Queued background task events: ${queuedEvents.size}.",
        critical = false
    )
    addCheck(
        "notification_permission",
        backgroundRuntime.optJSONObject("notification_permission")?.optBoolean("granted", true) ?: true,
        "Notification permission affects whether the user can see background debug/task service notifications.",
        critical = false
    )
    addCheck(
        "battery_optimization",
        backgroundRuntime.optJSONObject("battery_optimization")?.optBoolean("ignoring", true) ?: true,
        "Battery optimization may let the system defer background work on some Android builds.",
        critical = false
    )
    addCheck(
        "network_validated",
        backgroundRuntime.optJSONObject("network")?.optBoolean("validated", true) ?: true,
        "Validated network helps distinguish phone network issues from server or APK issues.",
        critical = false
    )

    val recommendations = JSONArray()
    if (!appForeground && !keepaliveActive) {
        recommendations.put("Call debug_keepalive start, then keep the notification alive before switching apps.")
    }
    if (lastError != null) {
        recommendations.put("Call mcp_metrics and logcat_recent with pattern ElonMcpServer|ElonTrace to inspect the last request failure.")
    }
    if (taskStatus.optBoolean("busy", false)) {
        recommendations.put("Call task_status for timing, or task_control pause if the active phone task should be cancelled.")
    }
    if (queuedEvents.isNotEmpty()) {
        recommendations.put("Call task_events to inspect messages queued while the app UI was backgrounded.")
    }
    backgroundRuntime.optJSONArray("recommendations")?.let { runtimeRecommendations ->
        for (index in 0 until runtimeRecommendations.length()) {
            runtimeRecommendations.optString(index).takeIf { it.isNotBlank() }?.let { recommendations.put(it) }
        }
    }

    val updateStatusResult = if (includeUpdateCheck) updateStatus() else JSONObject.NULL
    val ready = (0 until checks.length()).all { check ->
        val item = checks.optJSONObject(check) ?: return@all false
        !item.optBoolean("critical", true) || item.optBoolean("ok", false)
    }
    val structured = JSONObject()
        .put("ready", ready)
        .put("status", status)
        .put("task_status", taskStatus)
        .put("background_debug", backgroundRuntime)
        .put("metrics", metrics)
        .put("queued_task_event_count", queuedEvents.size)
        .put("checks", checks)
        .put("recommendations", recommendations)
        .put("update_status", updateStatusResult)
    return toolResult("MCP self-check returned.", structured, isError = !ready)
}
