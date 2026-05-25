package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

internal fun mcpLatestTraceId(events: List<DebugTraceStore.TraceEvent>): String? {
    return events.asReversed()
        .firstNotNullOfOrNull { it.details["trace_id"]?.takeIf(String::isNotBlank) }
}

internal fun mcpTraceDetailLong(
    events: List<DebugTraceStore.TraceEvent>,
    phase: String,
    key: String
): Any {
    return events.firstOrNull { it.phase == phase }
        ?.details
        ?.get(key)
        ?.toLongOrNull()
        ?: JSONObject.NULL
}

internal fun mcpTraceEventsJson(events: List<DebugTraceStore.TraceEvent>): JSONArray {
    return JSONArray().apply {
        events.forEach { event ->
            put(
                JSONObject()
                    .put("wall_time_ms", event.wallTimeMs)
                    .put("elapsed_ms", event.elapsedMs)
                    .put("phase", event.phase)
                    .put("details", JSONObject().apply {
                        event.details.forEach { (key, value) -> put(key, value) }
                    })
            )
        }
    }
}

internal fun mcpTraceEventSearchText(event: DebugTraceStore.TraceEvent): String {
    return buildString {
        append(event.phase)
        event.details.forEach { (key, value) ->
            append(' ').append(key).append('=').append(value)
        }
    }
}

internal fun mcpBottleneckJson(bottleneck: Pair<String, Long>?): JSONObject {
    if (bottleneck == null) {
        return JSONObject()
            .put("available", false)
            .put("severity", "insufficient_data")
            .put("name", JSONObject.NULL)
            .put("duration_ms", JSONObject.NULL)
            .put("likely_area", JSONObject.NULL)
            .put("recommendation", "Collect a fresh debug_session and call latency_report after the task reaches at least payload_sent.")
    }
    val name = bottleneck.first
    val durationMs = bottleneck.second
    val severity = when {
        durationMs >= 30_000L -> "slow"
        durationMs >= 10_000L -> "watch"
        else -> "normal"
    }
    val likelyArea = when (name) {
        "mcp_send_to_queue",
        "mcp_queue_to_service_command",
        "service_command_to_task_start",
        "mcp_queue_to_task_start",
        "mcp_queue_to_now" -> "phone_task_start"
        "task_start_to_ws_connected" -> "phone_websocket_connect"
        "ws_connected_to_payload_sent", "task_start_to_payload_sent" -> "phone_payload_send"
        "payload_sent_to_first_server_event" -> "backend_or_network_first_byte"
        "first_server_event_to_first_chat_reply" -> "backend_model_first_reply"
        "first_chat_reply_to_finish" -> "backend_completion_or_apk_release"
        else -> "unknown"
    }
    val recommendation = when (likelyArea) {
        "phone_task_start" -> "Inspect task queue and foreground-service startup; use background_debug_status and task_events."
        "phone_websocket_connect" -> "Compare network_check with ws_failure/ws_connected trace events; backend port or phone network may be slow."
        "phone_payload_send" -> "Check payload_bytes and attachment handling; large payloads can delay WebSocket send."
        "backend_or_network_first_byte" -> "Backend accepted the phone payload slowly; compare server logs for this trace_id."
        "backend_model_first_reply" -> "Backend/model generation is the likely first-reply bottleneck."
        "backend_completion_or_apk_release" -> "First reply arrived; remaining time is likely long-running build/deploy/completion work."
        else -> "Inspect trace_recent for the same trace_id."
    }
    return JSONObject()
        .put("available", true)
        .put("severity", severity)
        .put("name", name)
        .put("duration_ms", durationMs)
        .put("likely_area", likelyArea)
        .put("recommendation", recommendation)
}
