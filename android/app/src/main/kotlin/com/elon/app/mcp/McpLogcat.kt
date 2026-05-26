package com.elon.app.mcp

import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.TimeUnit

internal fun mcpLogcatRecent(args: JSONObject): JSONObject {
    val lineCount = args.optInt("line_count", 300).coerceIn(20, 1_000)
    val pattern = args.optString(
        "pattern",
        "ElonTrace|ElonTaskWork|ElonWsClient|ElonMcpServer|AndroidRuntime|FATAL EXCEPTION|ANR"
    ).takeIf { it.isNotBlank() }
        ?: "ElonTrace|ElonTaskWork|ElonWsClient|ElonMcpServer|AndroidRuntime|FATAL EXCEPTION|ANR"
    val regex = runCatching { Regex(pattern) }.getOrElse {
        return toolResult(
            "Invalid logcat pattern.",
            JSONObject().put("pattern", pattern).put("error", it.message),
            isError = true
        )
    }
    val command = listOf("logcat", "-d", "-v", "time", "-t", lineCount.toString())
    val process = runCatching { ProcessBuilder(command).redirectErrorStream(true).start() }.getOrElse {
        return toolResult(
            "Could not start logcat.",
            JSONObject().put("error", it.message),
            isError = true
        )
    }
    val finished = process.waitFor(3, TimeUnit.SECONDS)
    if (!finished) {
        process.destroy()
        return toolResult(
            "logcat timed out.",
            JSONObject().put("timeout_ms", 3_000),
            isError = true
        )
    }
    val output = process.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
    val lines = output
        .lineSequence()
        .filter { regex.containsMatchIn(it) }
        .map { redactLogLine(it) }
        .toList()
        .takeLast(300)
    val structured = JSONObject()
        .put("line_count", lineCount)
        .put("pattern", pattern)
        .put("limited_by_android_log_permissions", true)
        .put("lines", JSONArray().apply { lines.forEach { put(it) } })
    return toolResult("Filtered logcat returned.", structured)
}
