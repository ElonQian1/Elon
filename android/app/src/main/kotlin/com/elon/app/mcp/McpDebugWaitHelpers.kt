package com.elon.app.mcp

import com.elon.app.*
import android.os.SystemClock
import org.json.JSONObject

/**
 * MCP 调试服务器的 chat 链路等待 / 诊断辅助函数。
 *
 * 从 [McpDebugServer] 拆出，所有函数都是纯逻辑，输入是 traceId / 事件流 / 超时配置，
 * 输出是 JSON 结构。不依赖 McpDebugServer 的私有计数器或 appContext。
 */

internal fun waitForTaskStartSignal(traceId: String, timeoutMs: Int): JSONObject {
    val started = SystemClock.elapsedRealtime()
    val deadline = started + timeoutMs
    var events = emptyList<DebugTraceStore.TraceEvent>()
    while (true) {
        events = DebugTraceStore.recentEvents(300)
            .filter { it.details["trace_id"] == traceId }
        val signal = taskStartSignalJson(
            traceId = traceId,
            events = events,
            elapsedWaitMs = SystemClock.elapsedRealtime() - started,
            timedOut = false
        )
        if (signal.optBoolean("confirmed", false) || timeoutMs <= 0) return signal
        if (SystemClock.elapsedRealtime() >= deadline) break
        Thread.sleep(minOf(100L, (deadline - SystemClock.elapsedRealtime()).coerceAtLeast(0L)))
    }
    return taskStartSignalJson(
        traceId = traceId,
        events = events,
        elapsedWaitMs = SystemClock.elapsedRealtime() - started,
        timedOut = timeoutMs > 0
    )
}

internal fun taskStartSignalJson(
    traceId: String,
    events: List<DebugTraceStore.TraceEvent>,
    elapsedWaitMs: Long,
    timedOut: Boolean
): JSONObject {
    fun latest(vararg phases: String) = events.lastOrNull { it.phase in phases }
    val started = latest("task_start_work")
    val resumed = latest("task_resume_pending")
    val rejected = latest("task_start_rejected_busy")
    val missingPayload = latest("task_start_missing_payload")
    val command = latest("task_service_command")
    val matched = started ?: resumed ?: rejected ?: missingPayload ?: command
    val status = when {
        started != null -> "started"
        resumed != null -> "resumed"
        rejected != null -> "rejected_busy"
        missingPayload != null -> "missing_payload"
        command != null -> "service_command_received"
        else -> "unconfirmed"
    }
    val confirmed = status != "unconfirmed" && status != "service_command_received"
    return JSONObject()
        .put("trace_id", traceId)
        .put("status", status)
        .put("confirmed", confirmed)
        .put("timed_out", timedOut && !confirmed)
        .put("elapsed_wait_ms", elapsedWaitMs)
        .put("event_count", events.size)
        .put("matched_phase", matched?.phase ?: JSONObject.NULL)
        .put("matched_wall_time_ms", matched?.wallTimeMs ?: JSONObject.NULL)
        .put("last_phase", events.lastOrNull()?.phase ?: JSONObject.NULL)
}

internal fun waitForTraceTarget(
    traceId: String,
    waitFor: String,
    timeoutMs: Int,
    pollIntervalMs: Int,
    startedAtWallMs: Long
): JSONObject {
    val targetPhases = waitTargetPhases(waitFor) ?: emptySet()
    val terminalPhases = setOf("task_finish_done", "task_finish_error")
    val deadline = SystemClock.elapsedRealtime() + timeoutMs
    var lastEvents = emptyList<DebugTraceStore.TraceEvent>()
    while (true) {
        lastEvents = DebugTraceStore.recentEvents(300)
            .filter { it.details["trace_id"] == traceId || it.phase == "mcp_chat_send" && it.details["trace_id"] == traceId }
        val matched = lastEvents.firstOrNull { it.phase in targetPhases }
        if (matched != null) {
            return waitResultJson(
                traceId = traceId,
                waitFor = waitFor,
                reached = true,
                timedOut = false,
                matched = matched,
                events = lastEvents,
                startedAtWallMs = startedAtWallMs,
                timeoutMs = timeoutMs,
                pollIntervalMs = pollIntervalMs
            )
        }
        val terminal = lastEvents.firstOrNull { it.phase in terminalPhases }
        if (terminal != null && waitFor != "finish") {
            return waitResultJson(
                traceId = traceId,
                waitFor = waitFor,
                reached = false,
                timedOut = false,
                matched = terminal,
                events = lastEvents,
                startedAtWallMs = startedAtWallMs,
                timeoutMs = timeoutMs,
                pollIntervalMs = pollIntervalMs,
                terminalBeforeTarget = true
            )
        }
        if (timeoutMs <= 0 || SystemClock.elapsedRealtime() >= deadline) break
        Thread.sleep(minOf(pollIntervalMs.toLong(), (deadline - SystemClock.elapsedRealtime()).coerceAtLeast(0L)))
    }
    return waitResultJson(
        traceId = traceId,
        waitFor = waitFor,
        reached = false,
        timedOut = true,
        matched = lastEvents.lastOrNull(),
        events = lastEvents,
        startedAtWallMs = startedAtWallMs,
        timeoutMs = timeoutMs,
        pollIntervalMs = pollIntervalMs
    )
}

internal fun waitTargetPhases(waitFor: String): Set<String>? {
    return when (waitFor) {
        "queued" -> setOf("mcp_chat_queued")
        "task_start" -> setOf("task_start_work", "task_resume_pending")
        "payload_sent" -> setOf("task_payload_sent")
        "first_server_event" -> setOf("task_first_server_event")
        "first_reply" -> setOf("task_first_chat_reply")
        "finish" -> setOf("task_finish_done", "task_finish_error")
        else -> null
    }
}

internal fun waitResultJson(
    traceId: String,
    waitFor: String,
    reached: Boolean,
    timedOut: Boolean,
    matched: DebugTraceStore.TraceEvent?,
    events: List<DebugTraceStore.TraceEvent>,
    startedAtWallMs: Long,
    timeoutMs: Int,
    pollIntervalMs: Int,
    terminalBeforeTarget: Boolean = false
): JSONObject {
    return JSONObject()
        .put("trace_id", traceId)
        .put("wait_for", waitFor)
        .put("reached", reached)
        .put("timed_out", timedOut)
        .put("terminal_before_target", terminalBeforeTarget)
        .put("matched_phase", matched?.phase ?: JSONObject.NULL)
        .put("matched_wall_time_ms", matched?.wallTimeMs ?: JSONObject.NULL)
        .put("last_phase", events.lastOrNull()?.phase ?: JSONObject.NULL)
        .put("event_count", events.size)
        .put("elapsed_wait_ms", System.currentTimeMillis() - startedAtWallMs)
        .put("timeout_ms", timeoutMs)
        .put("poll_interval_ms", pollIntervalMs)
        .put("diagnosis", waitFailureDiagnosis(waitFor, reached, terminalBeforeTarget, events))
}

internal fun waitFailureDiagnosis(
    waitFor: String,
    reached: Boolean,
    terminalBeforeTarget: Boolean,
    events: List<DebugTraceStore.TraceEvent>
): JSONObject {
    if (reached) {
        return JSONObject()
            .put("available", false)
            .put("reason", "target_reached")
    }
    val phases = events.map { it.phase }.toSet()
    val code = when {
        terminalBeforeTarget -> "terminal_before_target"
        "task_payload_send_failed" in phases -> "payload_send_failed"
        "task_payload_sent" in phases && "task_first_server_event" !in phases -> "waiting_for_backend_first_event"
        ("task_start_work" in phases || "task_resume_pending" in phases) && "task_payload_sent" !in phases -> "task_started_without_payload_sent"
        "mcp_chat_start_unconfirmed" in phases -> "service_start_unconfirmed"
        "mcp_chat_queued" in phases && "task_start_work" !in phases && "task_resume_pending" !in phases -> "queued_without_task_start"
        "task_service_command" in phases && "task_start_work" !in phases && "task_resume_pending" !in phases -> "service_command_without_task_start"
        else -> "insufficient_trace"
    }
    val area = when (code) {
        "service_start_unconfirmed",
        "queued_without_task_start",
        "service_command_without_task_start" -> "phone_task_start"
        "task_started_without_payload_sent",
        "payload_send_failed" -> "phone_payload_send"
        "waiting_for_backend_first_event" -> "backend_or_network_first_byte"
        "terminal_before_target" -> "task_terminal"
        else -> "unknown"
    }
    val action = when (area) {
        "phone_task_start" -> "Inspect task_service_command, task_start_work, and mcp_chat_start_unconfirmed events; the MCP request may not be reaching TaskWorkService reliably."
        "phone_payload_send" -> "Inspect WebSocket connection state and payload size; a connected client must still call sendPendingPayloadIfNeeded for each new task."
        "backend_or_network_first_byte" -> "Compare phone latency_report with backend server_trace for the same trace_id."
        "task_terminal" -> "Read task_status and task_server_message for the terminal error before the requested milestone."
        else -> "Collect diagnostic_bundle with include_logcat=true for the same trace_id."
    }
    return JSONObject()
        .put("available", true)
        .put("wait_for", waitFor)
        .put("code", code)
        .put("likely_area", area)
        .put("action", action)
}
