package com.elon.app.grid.host

import java.net.URI

/** Page-local console classifications. Raw messages and stacks never leave this call. */
internal class BinanceScriptDiagnostics {
    private data class Failure(val kind: String, val source: String, val line: Int, val capability: String)
    private val recent = linkedSetOf<Failure>()
    private var count = 0

    fun record(message: String?, source: String?, line: Int) {
        count = (count + 1).coerceAtMost(10000)
        val text = message.orEmpty().take(2048)
        val kind = ERROR.find(text)?.groupValues?.get(1) ?: when {
            text.contains("Content Security Policy", ignoreCase = true) -> "content_security_policy"
            text.contains("Failed to load resource", ignoreCase = true) -> "resource_load"
            else -> "unknown"
        }
        val capability = CAPABILITIES.firstOrNull {
            Regex("(?:^|\\W)${Regex.escape(it)} (?:is not defined|is not a function)(?:$|\\W)").containsMatchIn(text)
        } ?: "none"
        val position = publicScript(source)
        val failure = Failure(kind, position, if (position == "none") 0 else line.coerceIn(0, 1000000), capability)
        if (recent.size < 6) recent.add(failure)
    }

    fun facts(): Map<String, Any> = mapOf(
        "schema" to "yilong.binance_script_diagnostics.v1",
        "error_count" to count,
        "errors" to recent.map { mapOf("kind" to it.kind, "source" to it.source, "line" to it.line, "capability" to it.capability) }
    )

    fun clear() { count = 0; recent.clear() }

    private fun publicScript(raw: String?): String = runCatching {
        if (raw == null || raw.length > 2048) return "none"
        val uri = URI(raw)
        val host = uri.host?.lowercase() ?: return "none"
        val path = uri.rawPath ?: return "none"
        if (uri.scheme != "https" || uri.rawUserInfo != null || uri.port !in setOf(-1, 443) ||
            host !in setOf("www.binance.com", "bin.bnbstatic.com", "public.bnbstatic.com") ||
            !path.startsWith("/static/") || !SCRIPT_PATH.matches(path) ||
            path.split('/').any { it == "." || it == ".." }) return "none"
        "https://$host$path"
    }.getOrDefault("none")

    companion object {
        private val ERROR = Regex("(?:^|\\W)(ReferenceError|TypeError|SyntaxError|RangeError|EvalError|URIError)(?=:|\\s|$)")
        private val SCRIPT_PATH = Regex("/[A-Za-z0-9_./@-]{1,500}\\.(?:js|mjs)")
        private val CAPABILITIES = listOf("process", "require", "Buffer", "SharedArrayBuffer", "ResizeObserver", "IntersectionObserver", "structuredClone", "crypto.randomUUID")
    }
}
