package com.elon.app.chatgptweb

import java.net.URI
import java.util.Locale

internal object ChatGptWebResearchResourceShape {
    private val VOICE_PATH_HINT = Regex(
        "voice|audio|speech|dictat|transcri|realtime|webrtc|rtc",
        RegexOption.IGNORE_CASE,
    )
    private val UUID_SEGMENT = Regex("^[0-9a-f]{8}-[0-9a-f-]{20,}$", RegexOption.IGNORE_CASE)
    private val LONG_ID_SEGMENT = Regex("^[A-Za-z0-9_-]{17,}$")
    private val NUMERIC_ID_SEGMENT = Regex("^[0-9]{7,}$")
    private val SAFE_SEGMENT = Regex("^[A-Za-z0-9._-]{1,40}$")
    private val MUTATING_METHODS = setOf("POST", "PUT", "PATCH", "DELETE")
    private val IGNORED_PATH_PREFIXES = listOf("/ces/", "/cdn-cgi/")

    fun from(method: String, rawUrl: String, contentType: String?): String? {
        val normalizedMethod = method.trim().uppercase(Locale.ROOT)
        if (normalizedMethod !in MUTATING_METHODS && normalizedMethod != "GET") return null
        val uri = runCatching { URI(rawUrl) }.getOrNull() ?: return null
        if (uri.scheme !in setOf("http", "https")) return null
        val family = hostFamily(uri.host.orEmpty()) ?: return null
        val path = safePath(uri.rawPath.orEmpty())
        if (IGNORED_PATH_PREFIXES.any(path::startsWith)) return null
        if (normalizedMethod == "GET" && !VOICE_PATH_HINT.containsMatchIn(path)) return null
        return listOf(
            "v1",
            "resource-start",
            normalizedMethod.lowercase(Locale.ROOT),
            family,
            path,
            contentKind(contentType),
        ).joinToString("|").take(160)
    }

    private fun hostFamily(host: String): String? {
        val value = host.lowercase(Locale.ROOT)
        return when {
            value == "chatgpt.com" || value.endsWith(".chatgpt.com") -> "chatgpt"
            value == "openai.com" || value.endsWith(".openai.com") -> "openai"
            else -> null
        }
    }

    private fun safePath(rawPath: String): String = rawPath
        .ifBlank { "/" }
        .split('/')
        .joinToString("/") { segment ->
            when {
                segment.isEmpty() -> ""
                '%' in segment -> "{segment}"
                UUID_SEGMENT.matches(segment) -> "{id}"
                NUMERIC_ID_SEGMENT.matches(segment) -> "{id}"
                LONG_ID_SEGMENT.matches(segment) -> "{id}"
                SAFE_SEGMENT.matches(segment) -> segment
                else -> "{segment}"
            }
        }
        .take(96)
        .ifBlank { "/" }

    private fun contentKind(contentType: String?): String {
        val value = contentType.orEmpty().lowercase(Locale.ROOT)
        return when {
            value.isBlank() -> "none"
            "json" in value -> "json"
            "multipart" in value || "form" in value -> "form"
            value.startsWith("audio/") -> "audio"
            value.startsWith("text/") -> "text"
            else -> "other"
        }
    }
}
