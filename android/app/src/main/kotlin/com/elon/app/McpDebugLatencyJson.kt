package com.elon.app

import android.content.Context
import android.os.SystemClock
import org.json.JSONArray
import org.json.JSONObject

/**
 * MCP 调试服务器的任务状态与延迟报告 JSON 构造器。
 *
 * 从 [McpDebugServer] 拆出，纯 JSON 构造逻辑，输入是 args + Android Context，
 * 输出是结构化任务状态 / 延迟分析 JSON。不依赖 McpDebugServer 的私有计数器。
 */

internal fun taskStatusJson(context: Context, args: JSONObject): JSONObject {
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    val pendingTask = pendingTaskJson(prefs)
    val requestedTraceId = args.optString("trace_id").takeIf { it.isNotBlank() }
    val events = DebugTraceStore.recentEvents(300)
    val traceId = requestedTraceId
        ?: pendingTask?.optString("trace_id")?.takeIf { it.isNotBlank() }
        ?: mcpLatestTraceId(events)
    val traceEvents = if (traceId == null) {
        emptyList()
    } else {
        events.filter { it.details["trace_id"] == traceId }
    }
    val finish = traceEvents.lastOrNull {
        it.phase == "task_finish_done" || it.phase == "task_finish_error"
    }
    val rejected = traceEvents.lastOrNull {
        it.phase == "task_start_rejected_busy" || it.phase == "mcp_chat_rejected_busy"
    }
    val serviceCommand = traceEvents.lastOrNull { it.phase == "task_service_command" }
    val startUnconfirmed = traceEvents.lastOrNull { it.phase == "mcp_chat_start_unconfirmed" }
    val hasTaskStart = traceEvents.any { it.phase == "task_start_work" || it.phase == "task_resume_pending" }
    val status = when {
        finish?.phase == "task_finish_done" -> "done"
        finish?.phase == "task_finish_error" -> "error"
        rejected != null && !hasTaskStart -> "rejected_busy"
        isTaskBusy(prefs) && traceId != null && isTracePending(prefs, traceId) -> "running"
        traceEvents.any { it.phase == "task_payload_sent" } -> "sent"
        hasTaskStart -> "started"
        startUnconfirmed != null -> "start_unconfirmed"
        serviceCommand != null -> "service_received"
        traceEvents.isNotEmpty() -> "observed"
        else -> "idle"
    }
    val lastMessage = traceEvents.lastOrNull { it.phase == "task_server_message" }
    return JSONObject()
        .put("status", status)
        .put("busy", isTaskBusy(prefs))
        .put("trace_id", traceId ?: JSONObject.NULL)
        .put("kind", pendingTaskKind(prefs))
        .put("pending_work_age_ms", pendingWorkAgeMs(prefs) ?: JSONObject.NULL)
        .put("event_count", traceEvents.size)
        .put("sent_elapsed_ms", mcpTraceDetailLong(traceEvents, "task_payload_sent", "elapsed_ms"))
        .put("first_server_event_elapsed_ms", mcpTraceDetailLong(traceEvents, "task_first_server_event", "elapsed_ms"))
        .put("first_chat_reply_elapsed_ms", mcpTraceDetailLong(traceEvents, "task_first_chat_reply", "elapsed_ms"))
        .put("finish_elapsed_ms", finish?.details?.get("elapsed_ms")?.toLongOrNull() ?: JSONObject.NULL)
        .put("last_message_type", lastMessage?.details?.get("type") ?: JSONObject.NULL)
        .put("last_message_preview", lastMessage?.details?.get("message_preview") ?: JSONObject.NULL)
        .put("has_apk_url", finish?.details?.get("has_apk_url") ?: JSONObject.NULL)
        .put("last_phase", traceEvents.lastOrNull()?.phase ?: JSONObject.NULL)
        .put("last_event_wall_time_ms", traceEvents.lastOrNull()?.wallTimeMs ?: JSONObject.NULL)
}

internal fun latencyReportJson(context: Context, args: JSONObject): JSONObject {
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    val allEvents = DebugTraceStore.recentEvents(300)
    val requestedTraceId = args.optString("trace_id").takeIf { it.isNotBlank() }
    val pendingTraceId = pendingTaskJson(prefs)?.optString("trace_id")?.takeIf { it.isNotBlank() }
    val traceId = requestedTraceId ?: pendingTraceId ?: mcpLatestTraceId(allEvents)
    val traceEvents = if (traceId == null) {
        emptyList()
    } else {
        allEvents.filter { it.details["trace_id"] == traceId }
    }
    val timelineLimit = args.optInt("timeline_limit", 80).coerceIn(1, 300)
    val taskStatusArgs = JSONObject()
    traceId?.let { taskStatusArgs.put("trace_id", it) }
    val taskStatus = taskStatusJson(context, taskStatusArgs)

    fun first(phase: String) = traceEvents.firstOrNull { it.phase == phase }
    fun last(phase: String) = traceEvents.lastOrNull { it.phase == phase }

    val mcpSend = first("mcp_chat_send")
    val mcpQueued = first("mcp_chat_queued")
    val serviceCommand = first("task_service_command")
    val startUnconfirmed = last("mcp_chat_start_unconfirmed")
    val taskStart = first("task_start_work") ?: first("task_resume_pending")
    val wsConnected = first("task_ws_connected")
    val payloadSent = first("task_payload_sent")
    val firstServer = first("task_first_server_event")
    val firstReply = first("task_first_chat_reply")
    val finish = first("task_finish_done") ?: first("task_finish_error")
    val baseline = taskStart ?: traceEvents.firstOrNull()
    val milestones = JSONObject()

    fun elapsedFromBaseline(event: DebugTraceStore.TraceEvent): Any {
        val fromTrace = event.details["elapsed_ms"]?.toLongOrNull()
        return when {
            fromTrace != null -> fromTrace
            baseline != null -> (event.wallTimeMs - baseline.wallTimeMs).coerceAtLeast(0L)
            else -> JSONObject.NULL
        }
    }

    fun putMilestone(name: String, event: DebugTraceStore.TraceEvent?) {
        if (event == null) return
        milestones.put(
            name,
            JSONObject()
                .put("phase", event.phase)
                .put("wall_time_ms", event.wallTimeMs)
                .put("elapsed_from_task_start_ms", elapsedFromBaseline(event))
                .put("details", JSONObject().apply {
                    event.details.forEach { (key, value) -> put(key, value) }
                })
        )
    }

    putMilestone("mcp_send", mcpSend)
    putMilestone("mcp_queued", mcpQueued)
    putMilestone("service_command", serviceCommand)
    putMilestone("mcp_start_unconfirmed", startUnconfirmed)
    putMilestone("task_start", taskStart)
    putMilestone("ws_connected", wsConnected)
    putMilestone("payload_sent", payloadSent)
    putMilestone("first_server_event", firstServer)
    putMilestone("first_chat_reply", firstReply)
    putMilestone("finish", finish)

    val segments = JSONArray()
    val bottleneckCandidates = mutableListOf<Pair<String, Long>>()
    fun addSegment(name: String, from: DebugTraceStore.TraceEvent?, to: DebugTraceStore.TraceEvent?, candidate: Boolean = true) {
        if (from == null || to == null) return
        val duration = (to.wallTimeMs - from.wallTimeMs).coerceAtLeast(0L)
        segments.put(
            JSONObject()
                .put("name", name)
                .put("from_phase", from.phase)
                .put("to_phase", to.phase)
                .put("duration_ms", duration)
        )
        if (candidate) bottleneckCandidates += name to duration
    }

    addSegment("mcp_send_to_queue", mcpSend, mcpQueued)
    addSegment("mcp_queue_to_service_command", mcpQueued, serviceCommand)
    addSegment("service_command_to_task_start", serviceCommand, taskStart)
    addSegment("mcp_queue_to_task_start", mcpQueued, taskStart)
    if (mcpQueued != null && taskStart == null && finish == null) {
        val now = DebugTraceStore.TraceEvent(
            wallTimeMs = System.currentTimeMillis(),
            elapsedMs = SystemClock.elapsedRealtime(),
            phase = "now",
            details = mapOf("trace_id" to traceId.orEmpty())
        )
        addSegment("mcp_queue_to_now", mcpQueued, now)
    }
    addSegment("task_start_to_ws_connected", taskStart, wsConnected)
    addSegment("ws_connected_to_payload_sent", wsConnected, payloadSent)
    addSegment("task_start_to_payload_sent", taskStart, payloadSent)
    addSegment("payload_sent_to_first_server_event", payloadSent, firstServer)
    addSegment("first_server_event_to_first_chat_reply", firstServer, firstReply)
    addSegment("first_chat_reply_to_finish", firstReply, finish)
    addSegment("task_start_to_finish", taskStart, finish, candidate = false)

    val bottleneck = bottleneckCandidates.maxByOrNull { it.second }
    val missingMilestones = JSONArray().apply {
        if (taskStart == null) put("task_start")
        if (payloadSent == null) put("payload_sent")
        if (firstServer == null) put("first_server_event")
        if (firstReply == null) put("first_chat_reply")
        if (finish == null) put("finish")
    }
    val timeline = JSONArray().apply {
        traceEvents.takeLast(timelineLimit).forEach { event ->
            put(
                JSONObject()
                    .put("phase", event.phase)
                    .put("wall_time_ms", event.wallTimeMs)
                    .put("elapsed_from_task_start_ms", elapsedFromBaseline(event))
                    .put("details", JSONObject().apply {
                        event.details.forEach { (key, value) -> put(key, value) }
                    })
            )
        }
    }

    return JSONObject()
        .put("trace_id", traceId ?: JSONObject.NULL)
        .put("status", taskStatus.optString("status", "idle"))
        .put("kind", taskStatus.optString("kind", "idle"))
        .put("event_count", traceEvents.size)
        .put("timeline_limit", timelineLimit)
        .put("milestones", milestones)
        .put("segments", segments)
        .put("bottleneck", mcpBottleneckJson(bottleneck))
        .put("missing_milestones", missingMilestones)
        .put("timeline", timeline)
        .put("task_status", taskStatus)
}
