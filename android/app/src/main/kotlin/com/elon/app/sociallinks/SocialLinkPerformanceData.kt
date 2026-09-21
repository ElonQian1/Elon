package com.elon.app.sociallinks

import org.json.JSONArray
import org.json.JSONObject
import java.net.URI

/** The page is untrusted. Only bounded numbers, enums and hostnames enter diagnostics. */
internal object SocialLinkPerformanceData {
    private val timings = listOf("fcp_ms", "lcp_ms", "dns_ms", "connect_ms", "ttfb_ms", "response_end_ms", "dom_interactive_ms", "dcl_ms", "load_ms", "long_task_ms", "longest_task_ms")
    private val counts = listOf("js_errors", "promise_rejections", "long_tasks")
    private val kinds = setOf("script", "link", "img", "fetch", "xmlhttprequest", "iframe", "css")
    fun host(value: String?): String = runCatching {
        val host = URI(value.orEmpty()).host.orEmpty().lowercase()
        hostname(host)
    }.getOrDefault("")
    private fun hostname(value: String): String = value.takeIf {
        it.length in 1..253 && Regex("[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?").matches(it)
    }.orEmpty()
    fun sanitize(raw: JSONObject): JSONObject {
        val out = JSONObject().put("schema", 1)
        out.put("content_visible", raw.opt("content_visible") == true)
        out.put("content_state", raw.optString("content_state").takeIf { it in setOf("absent", "hidden", "visible", "painted", "pending") } ?: "unknown")
        out.put("ready_state", raw.optString("ready_state").takeIf { it in setOf("loading", "interactive", "complete") } ?: "unknown")
        timings.forEach { key ->
            val value = raw.opt(key) as? Number
            value?.toDouble()?.takeIf { it.isFinite() && it in 0.0..600000.0 }?.let { out.put(key, it) }
        }
        counts.forEach { key -> out.put(key, (raw.opt(key) as? Number)?.toInt()?.coerceIn(0, 10000) ?: 0) }
        val resources = JSONArray(); val input = raw.optJSONArray("slow_resources") ?: JSONArray()
        for (i in 0 until minOf(input.length(), 8)) {
            val item = input.optJSONObject(i) ?: continue
            val host = hostname(item.optString("host").lowercase())
            val duration = (item.opt("duration_ms") as? Number)?.toDouble() ?: continue
            if (host.isEmpty() || !duration.isFinite() || duration !in 0.0..600000.0) continue
            resources.put(JSONObject().put("host", host).put("duration_ms", duration)
                .put("kind", item.optString("kind").takeIf { it in kinds } ?: "other"))
        }
        out.put("slow_resources", resources)
        val failures = JSONArray(); val errors = raw.optJSONArray("resource_error_hosts") ?: JSONArray()
        for (i in 0 until minOf(errors.length(), 8)) hostname(errors.optString(i).lowercase()).takeIf { it.isNotEmpty() }?.let { failures.put(it) }
        out.put("resource_error_hosts", failures)
        return out
    }
}
