package com.elon.app.mcp

import com.elon.app.*
import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale
import java.util.UUID

internal fun mcpChatProbe(
    context: Context,
    args: JSONObject,
    defaultServerBaseUrl: String,
    diagnosticBundle: (JSONObject) -> JSONObject
): JSONObject {
    val message = args.optString("message").trim()
    if (message.isEmpty()) {
        return toolResult("message is required", JSONObject().put("field", "message"), isError = true)
    }
    val waitFor = args.optString("wait_for", "first_reply").lowercase(Locale.ROOT)
    val timeoutMs = args.optInt("wait_timeout_ms", 25_000).coerceIn(0, 120_000)
    val pollIntervalMs = args.optInt("poll_interval_ms", 350).coerceIn(100, 2_000)
    val timelineLimit = args.optInt("timeline_limit", 80).coerceIn(1, 300)
    val includeDiagnosticBundle = if (args.has("include_diagnostic_bundle")) {
        args.optBoolean("include_diagnostic_bundle")
    } else {
        true
    }
    val includeLogcat = args.optBoolean("include_logcat", false)
    val includeNetworkCheck = if (args.has("include_network_check")) {
        args.optBoolean("include_network_check")
    } else {
        true
    }
    val includeServerTrace = if (args.has("include_server_trace")) {
        args.optBoolean("include_server_trace")
    } else {
        true
    }

    if (waitTargetPhases(waitFor) == null) {
        return toolResult(
            "Unsupported wait_for value: $waitFor",
            JSONObject()
                .put("wait_for", waitFor)
                .put("supported", JSONArray().apply {
                    listOf("queued", "task_start", "payload_sent", "first_server_event", "first_reply", "finish")
                        .forEach { put(it) }
                }),
            isError = true
        )
    }

    val probeStartedAt = System.currentTimeMillis()
    val chatArgs = JSONObject(args.toString())
    if (!chatArgs.has("is_development")) chatArgs.put("is_development", false)
    if (!chatArgs.has("trace_id")) {
        chatArgs.put("trace_id", "mcp_probe_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}")
    }
    DebugTraceStore.record(
        "mcp_chat_probe_start",
        mapOf(
            "trace_id" to chatArgs.optString("trace_id"),
            "wait_for" to waitFor,
            "timeout_ms" to timeoutMs,
            "is_development" to chatArgs.optBoolean("is_development")
        )
    )

    val sendResult = chatSend(context, chatArgs)
    val sendStructured = sendResult.optJSONObject("structuredContent") ?: JSONObject()
    val traceId = sendStructured.optString("trace_id").takeIf { it.isNotBlank() }
        ?: chatArgs.optString("trace_id").takeIf { it.isNotBlank() }
    if (sendResult.optBoolean("isError", false) || traceId == null) {
        return toolResult(
            "Chat probe could not queue a phone chat request.",
            JSONObject()
                .put("send_result", sendStructured)
                .put("wait_for", waitFor)
                .put("queued", false),
            isError = true
        )
    }

    val waitResult = waitForTraceTarget(
        traceId = traceId,
        waitFor = waitFor,
        timeoutMs = timeoutMs,
        pollIntervalMs = pollIntervalMs,
        startedAtWallMs = probeStartedAt
    )
    val latency = latencyReportJson(context, JSONObject().put("trace_id", traceId).put("timeline_limit", timelineLimit))
    val taskStatus = taskStatusJson(context, JSONObject().put("trace_id", traceId))
    val serverTrace = if (includeServerTrace) {
        JSONObject()
            .put("trace_id", traceId)
            .put("limit", args.optInt("server_trace_limit", 120).coerceIn(1, 300))
            .apply {
                args.optString("server_url").takeIf { it.isNotBlank() }?.let { put("server_url", it) }
            }
            .let { serverTraceJson(context, it, defaultServerBaseUrl) }
    } else {
        JSONObject.NULL
    }
    val diagnostic = if (includeDiagnosticBundle) {
        diagnosticBundle(
            JSONObject()
                .put("trace_id", traceId)
                .put("include_logcat", includeLogcat)
                .put("include_network_check", includeNetworkCheck)
                .put("include_server_trace", includeServerTrace)
                .put("server_trace_limit", args.optInt("server_trace_limit", 120).coerceIn(1, 300))
                .put("trace_limit", timelineLimit)
                .put("timeline_limit", timelineLimit)
                .put("since_wall_time_ms", probeStartedAt)
                .apply {
                    args.optString("server_url").takeIf { it.isNotBlank() }?.let { put("server_url", it) }
                }
        )
    } else {
        JSONObject.NULL
    }

    DebugTraceStore.record(
        "mcp_chat_probe_finish",
        mapOf(
            "trace_id" to traceId,
            "wait_for" to waitFor,
            "reached" to waitResult.optBoolean("reached"),
            "matched_phase" to waitResult.optString("matched_phase").takeIf { it.isNotBlank() },
            "elapsed_wait_ms" to waitResult.optLong("elapsed_wait_ms")
        )
    )
    val structured = JSONObject()
        .put("trace_id", traceId)
        .put("wait_for", waitFor)
        .put("timeout_ms", timeoutMs)
        .put("queued", true)
        .put("send_result", sendStructured)
        .put("wait_result", waitResult)
        .put("task_status", taskStatus)
        .put("latency_report", latency)
        .put("server_trace", serverTrace)
        .put("diagnostic_bundle", diagnostic)
    return toolResult(
        if (waitResult.optBoolean("reached", false)) {
            "Chat probe reached $waitFor."
        } else {
            "Chat probe timed out before $waitFor."
        },
        structured,
        isError = waitResult.optBoolean("timed_out", false)
    )
}
