package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

internal fun mcpDiagnosticAssessmentJson(
    selfCheck: JSONObject,
    backgroundRuntime: JSONObject,
    network: Any,
    taskStatus: JSONObject,
    trace: JSONObject,
    logcat: Any,
    latency: JSONObject,
    serverTrace: Any
): JSONObject {
    val findings = JSONArray()
    val nextActions = JSONArray()
    var rank = 0

    fun addFinding(severity: String, area: String, detail: String, action: String) {
        val currentRank = when (severity) {
            "error" -> 3
            "warning" -> 2
            "info" -> 1
            else -> 0
        }
        rank = maxOf(rank, currentRank)
        findings.put(
            JSONObject()
                .put("severity", severity)
                .put("area", area)
                .put("detail", detail)
                .put("action", action)
        )
        nextActions.put(action)
    }

    if (!selfCheck.optBoolean("ready", true)) {
        addFinding(
            "error",
            "mcp_self_check",
            "A critical MCP self-check failed.",
            "Inspect self_check.checks where critical=true and ok=false."
        )
    }

    if (!backgroundRuntime.optBoolean("background_reachable", true)) {
        addFinding(
            "error",
            "background",
            "MCP may stop being reachable when the user leaves the APK.",
            "Call debug_keepalive action=start, then verify background_debug_status.background_reachable=true."
        )
    } else if (backgroundRuntime.optString("reachability") == "at_risk") {
        addFinding(
            "warning",
            "background",
            "Background MCP is active but Android environment has risk factors.",
            "Review background_debug.caveats before collecting long-running traces."
        )
    }

    val networkJson = network as? JSONObject
    if (networkJson != null) {
        val activeNetwork = networkJson.optJSONObject("network")
        if (activeNetwork != null && (!activeNetwork.optBoolean("active") || !activeNetwork.optBoolean("internet"))) {
            addFinding(
                "error",
                "network",
                "Phone has no active internet-capable network.",
                "Reconnect Wi-Fi/cellular, then call network_check again."
            )
        }
        val tcpOk = networkJson.optJSONObject("tcp_probe")?.optBoolean("ok", true) ?: true
        if (!tcpOk) {
            addFinding(
                "error",
                "network",
                "Phone cannot open a TCP connection to the backend.",
                "Check phone network, server port 8080, and carrier/Wi-Fi firewall before debugging chat latency."
            )
        }
        val httpProbes = networkJson.optJSONArray("http_probes")
        if (httpProbes != null) {
            var failed = 0
            for (index in 0 until httpProbes.length()) {
                if (httpProbes.optJSONObject(index)?.optBoolean("ok", true) == false) failed += 1
            }
            if (failed > 0) {
                addFinding(
                    "warning",
                    "network",
                    "$failed backend HTTP probe(s) failed from the phone.",
                    "Inspect network_check.http_probes for status_code/error and compare with desktop curl."
                )
            }
        }
    }

    when (taskStatus.optString("status")) {
        "error" -> addFinding(
            "error",
            "task",
            "The latest traced phone task finished with an error.",
            "Open task_status.last_message_preview and trace_recent events for the active trace_id."
        )
        "start_unconfirmed" -> addFinding(
            "warning",
            "task",
            "MCP queued the task but did not confirm TaskWorkService startup in time.",
            "Inspect trace_recent for mcp_chat_start_unconfirmed and task_service_command; retry with chat_probe wait_for=task_start."
        )
        "service_received" -> addFinding(
            "warning",
            "task",
            "TaskWorkService received the command but did not emit task_start_work.",
            "Inspect task_service_command, task_start_missing_payload, and the pending task payload."
        )
        "rejected_busy" -> addFinding(
            "warning",
            "task",
            "A new task was rejected because another phone task was active.",
            "Wait for the active trace_id to finish or call task_control action=pause before retrying."
        )
    }
    val pendingAgeMs = taskStatus.optLong("pending_work_age_ms", 0L)
    if (taskStatus.optBoolean("busy", false) && pendingAgeMs > 10 * 60 * 1000L) {
        addFinding(
            "warning",
            "task",
            "A phone task has been pending for more than 10 minutes.",
            "Use diagnostic_bundle with trace_id=${taskStatus.optString("trace_id")} and inspect ws/task phases."
        )
    }
    val bottleneck = latency.optJSONObject("bottleneck")
    if (bottleneck?.optBoolean("available", false) == true) {
        val severity = bottleneck.optString("severity")
        if (severity == "slow" || severity == "watch") {
            addFinding(
                "warning",
                "latency",
                "Largest latency segment is ${bottleneck.optString("name")} at ${bottleneck.optLong("duration_ms")} ms.",
                bottleneck.optString("recommendation", "Inspect latency_report.bottleneck.")
            )
        }
    }

    val serverTraceJson = serverTrace as? JSONObject
    if (serverTraceJson != null) {
        val traceId = taskStatus.optString("trace_id").takeIf { it.isNotBlank() && it != "null" }
        if (traceId != null && !serverTraceJson.optBoolean("available", false)) {
            addFinding(
                "warning",
                "server_trace",
                "Server-side trace for the active trace_id could not be loaded.",
                "Call server_trace with trace_id=$traceId and compare with network_check."
            )
        } else if (traceId != null && serverTraceJson.optInt("matched_count", 0) == 0) {
            val phonePayloadSent = trace.optJSONArray("events")?.let { events ->
                (0 until events.length()).any { index ->
                    events.optJSONObject(index)?.optString("phase") == "task_payload_sent"
                }
            } ?: false
            if (phonePayloadSent) {
                addFinding(
                    "warning",
                    "server_trace",
                    "Phone says the payload was sent, but the backend has no matching server trace.",
                    "Check the backend deployment/version and whether the phone sent the expected trace_id."
                )
            }
        }
    }

    val matchedTraceCount = trace.optInt("matched_count", 0)
    if (matchedTraceCount == 0) {
        addFinding(
            "info",
            "trace",
            "No trace events matched the diagnostic window.",
            "Start a debug_session before reproducing the issue, then call diagnostic_bundle again."
        )
    }

    val logcatJson = logcat as? JSONObject
    val logLines = logcatJson?.optJSONArray("lines")
    if (logLines != null) {
        var crashLines = 0
        for (index in 0 until logLines.length()) {
            val line = logLines.optString(index)
            if (
                line.contains("FATAL EXCEPTION", ignoreCase = true) ||
                line.contains("AndroidRuntime", ignoreCase = true) ||
                line.contains("ANR", ignoreCase = true)
            ) {
                crashLines += 1
            }
        }
        if (crashLines > 0) {
            addFinding(
                "error",
                "logcat",
                "Logcat contains $crashLines crash/ANR related line(s).",
                "Inspect logcat_recent.lines before retrying; crash noise may explain missing chat timing events."
            )
        }
    }

    val severity = when (rank) {
        3 -> "error"
        2 -> "warning"
        1 -> "info"
        else -> "ok"
    }
    val summary = when (severity) {
        "error" -> "Critical issue found; fix the failing area before trusting latency results."
        "warning" -> "MCP is usable, but the bundle found risk factors that may affect debugging."
        "info" -> "MCP is usable; collect a fresh debug session if the trace window is empty."
        else -> "MCP debug environment looks ready."
    }
    return JSONObject()
        .put("severity", severity)
        .put("summary", summary)
        .put("findings", findings)
        .put("next_actions", nextActions)
}
