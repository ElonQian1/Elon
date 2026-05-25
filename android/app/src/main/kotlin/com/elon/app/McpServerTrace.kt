package com.elon.app

import android.content.Context
import android.os.SystemClock
import org.json.JSONObject
import java.net.URLEncoder

/**
 * MCP 调试服务器的 server_trace 工具实现。
 *
 * 从 [McpDebugServer] 拆出。根据 trace_id 拉取后端 `/api/debug/traces/<id>`
 * 的事件序列，用于把手机 trace 与服务器 trace 对齐排查。
 */
internal fun serverTraceJson(
    context: Context,
    args: JSONObject,
    defaultServerBaseUrl: String,
): JSONObject {
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    val events = DebugTraceStore.recentEvents(300)
    val traceId = args.optString("trace_id").takeIf { it.isNotBlank() }
        ?: pendingTaskJson(prefs)?.optString("trace_id")?.takeIf { it.isNotBlank() }
        ?: mcpLatestTraceId(events)
    val limit = args.optInt("limit", 120).coerceIn(1, 300)
    val baseUrl = args.optString("server_url").takeIf { it.isNotBlank() }
        ?: defaultServerBaseUrl
    if (traceId == null) {
        return JSONObject()
            .put("available", false)
            .put("reason", "missing_trace_id")
            .put("server_url", baseUrl)
            .put("limit", limit)
    }

    val encodedTraceId = URLEncoder.encode(traceId, "UTF-8")
    val url = "${baseUrl.trimEnd('/')}/api/debug/traces/$encodedTraceId?limit=$limit"
    val started = SystemClock.elapsedRealtime()
    val response = fetchJson(url)
    val durationMs = SystemClock.elapsedRealtime() - started
    if (response == null) {
        return JSONObject()
            .put("available", false)
            .put("reason", "server_unreachable_or_non_json")
            .put("trace_id", traceId)
            .put("url", url)
            .put("duration_ms", durationMs)
            .put("limit", limit)
    }
    return response
        .put("available", true)
        .put("url", url)
        .put("duration_ms", durationMs)
        .put("server_url", baseUrl)
}
