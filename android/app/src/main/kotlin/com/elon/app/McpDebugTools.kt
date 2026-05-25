package com.elon.app

import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import androidx.core.content.ContextCompat
import org.json.JSONObject
import java.util.Locale
import java.util.UUID

private const val PREF_DEBUG_SESSION_ID = "mcp_debug_session_id"
private const val PREF_DEBUG_SESSION_STARTED_AT = "mcp_debug_session_started_at"
private const val PREF_DEBUG_SESSION_NOTE = "mcp_debug_session_note"

internal fun mcpTraceRecent(args: JSONObject): JSONObject {
    val limit = args.optInt("limit", 80).coerceIn(1, 300)
    val traceId = args.optString("trace_id").takeIf { it.isNotBlank() }
    val phase = args.optString("phase").takeIf { it.isNotBlank() }
    val contains = args.optString("contains").takeIf { it.isNotBlank() }
    val sinceWallTimeMs = args.optLong("since_wall_time_ms", 0L).takeIf { it > 0L }
    val allEvents = DebugTraceStore.recentEvents(300)
    val filtered = allEvents.filter { event ->
        (traceId == null || event.details["trace_id"] == traceId) &&
            (phase == null || event.phase == phase) &&
            (contains == null || mcpTraceEventSearchText(event).contains(contains, ignoreCase = true)) &&
            (sinceWallTimeMs == null || event.wallTimeMs >= sinceWallTimeMs)
    }
    val events = filtered.takeLast(limit)
    val structured = JSONObject()
        .put("events", mcpTraceEventsJson(events))
        .put("limit", limit)
        .put("matched_count", filtered.size)
        .put(
            "filters",
            JSONObject()
                .put("trace_id", traceId ?: JSONObject.NULL)
                .put("phase", phase ?: JSONObject.NULL)
                .put("contains", contains ?: JSONObject.NULL)
                .put("since_wall_time_ms", sinceWallTimeMs ?: JSONObject.NULL)
        )
    return toolResult("Returned ${events.size} recent trace events.", structured)
}

internal fun mcpDebugSession(context: Context, args: JSONObject): JSONObject {
    val action = args.optString("action", "status").lowercase(Locale.ROOT)
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    val structured = when (action) {
        "start" -> startMcpDebugSession(
            prefs = prefs,
            requestedSessionId = args.optString("session_id").takeIf { it.isNotBlank() },
            note = args.optString("note").takeIf { it.isNotBlank() }
        ).put("action", action)
        "end" -> {
            val current = mcpDebugSessionJson(prefs)
            prefs.edit()
                .remove(PREF_DEBUG_SESSION_ID)
                .remove(PREF_DEBUG_SESSION_STARTED_AT)
                .remove(PREF_DEBUG_SESSION_NOTE)
                .apply()
            DebugTraceStore.record(
                "mcp_debug_session_end",
                mapOf("debug_session_id" to current.optString("session_id").takeIf { it.isNotBlank() })
            )
            mcpDebugSessionJson(prefs)
                .put("action", action)
                .put("ended_session", current)
        }
        "status" -> mcpDebugSessionJson(prefs).put("action", action)
        else -> return toolResult(
            "Unsupported debug session action: $action",
            JSONObject().put("action", action),
            isError = true
        )
    }
    return toolResult("Debug session status returned.", structured)
}

internal fun startMcpDebugSession(
    prefs: SharedPreferences,
    requestedSessionId: String?,
    note: String?
): JSONObject {
    val sessionId = requestedSessionId ?: "mcp_session_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}"
    val startedAt = System.currentTimeMillis()
    prefs.edit()
        .putString(PREF_DEBUG_SESSION_ID, sessionId)
        .putLong(PREF_DEBUG_SESSION_STARTED_AT, startedAt)
        .apply {
            if (note == null) remove(PREF_DEBUG_SESSION_NOTE) else putString(PREF_DEBUG_SESSION_NOTE, note)
        }
        .apply()
    DebugTraceStore.record(
        "mcp_debug_session_start",
        mapOf("debug_session_id" to sessionId, "note" to note)
    )
    return mcpDebugSessionJson(prefs)
}

internal fun mcpDebugSessionJson(prefs: SharedPreferences): JSONObject {
    val sessionId = prefs.getString(PREF_DEBUG_SESSION_ID, null)?.takeIf { it.isNotBlank() }
    val startedAt = prefs.getLong(PREF_DEBUG_SESSION_STARTED_AT, 0L).takeIf { it > 0L }
    return JSONObject()
        .put("active", sessionId != null && startedAt != null)
        .put("session_id", sessionId ?: JSONObject.NULL)
        .put("started_at_ms", startedAt ?: JSONObject.NULL)
        .put("age_ms", startedAt?.let { System.currentTimeMillis() - it } ?: JSONObject.NULL)
        .put("note", prefs.getString(PREF_DEBUG_SESSION_NOTE, null) ?: JSONObject.NULL)
}

internal fun mcpDebugKeepalive(context: Context, args: JSONObject): JSONObject {
    val action = args.optString("action", "status").lowercase(Locale.ROOT)
    when (action) {
        "start" -> {
            context.getSharedPreferences("elon", Context.MODE_PRIVATE)
                .edit()
                .putBoolean(McpDebugKeepAliveService.PREF_MANUAL_STOPPED, false)
                .putBoolean(McpDebugKeepAliveService.PREF_ACTIVE, true)
                .putLong(McpDebugKeepAliveService.PREF_STARTED_AT, System.currentTimeMillis())
                .apply()
            val intent = Intent(context, McpDebugKeepAliveService::class.java).apply {
                this.action = McpDebugKeepAliveService.ACTION_START
            }
            ContextCompat.startForegroundService(context, intent)
            DebugTraceStore.record("mcp_keepalive_requested", mapOf("action" to "start"))
        }
        "stop" -> {
            context.getSharedPreferences("elon", Context.MODE_PRIVATE)
                .edit()
                .putBoolean(McpDebugKeepAliveService.PREF_MANUAL_STOPPED, true)
                .putBoolean(McpDebugKeepAliveService.PREF_ACTIVE, false)
                .remove(McpDebugKeepAliveService.PREF_STARTED_AT)
                .apply()
            val intent = Intent(context, McpDebugKeepAliveService::class.java).apply {
                this.action = McpDebugKeepAliveService.ACTION_STOP
            }
            context.stopService(intent)
            DebugTraceStore.record("mcp_keepalive_requested", mapOf("action" to "stop"))
        }
        "status" -> Unit
        else -> return toolResult(
            "Unsupported keepalive action: $action",
            JSONObject().put("action", action),
            isError = true
        )
    }
    return toolResult(
        "MCP keepalive status returned.",
        McpDebugKeepAliveService.statusJson(context).put("action", action)
    )
}

internal fun mcpUpdateStatus(args: JSONObject): JSONObject {
    val url = args.optString("server_url")
        .takeIf { it.isNotBlank() }
        ?: "http://43.139.149.158:8080/app/version.json"
    val server = fetchJson(url)
    val latestCode = server?.optInt("versionCode", 0) ?: 0
    val structured = JSONObject()
        .put("installed_version_name", BuildConfig.VERSION_NAME)
        .put("installed_version_code", BuildConfig.VERSION_CODE)
        .put("server_url", url)
        .put("server_version", server ?: JSONObject.NULL)
        .put("update_available", latestCode > BuildConfig.VERSION_CODE)
    return toolResult("APK update status returned.", structured, isError = server == null)
}
