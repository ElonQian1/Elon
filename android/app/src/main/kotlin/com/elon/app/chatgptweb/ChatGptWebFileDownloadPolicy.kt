package com.elon.app.chatgptweb

import java.net.URI
import java.util.UUID

internal object ChatGptWebFileDownloadPolicy {
    val HANDLE = Regex("download_[a-f0-9]{32}")

    fun signedUrl(value: String): String? = runCatching {
        if (value.length !in 1..16384 || value.any { it <= ' ' }) return null
        val uri = URI(value)
        value.takeIf {
            uri.scheme == "https" && uri.host?.matches(Regex("[a-z0-9][a-z0-9.-]*\\.oaiusercontent\\.com")) == true &&
                uri.port in setOf(-1, 443) && uri.rawUserInfo == null && uri.rawFragment == null
        }
    }.getOrNull()

    fun safeName(value: String): String = value
        .replace(Regex("[\\x00-\\x1f\\x7f-\\x9f/\\\\:*?\"<>|\\u202a-\\u202e\\u2066-\\u2069]"), "_")
        .trim(' ', '.').take(150).ifBlank { "download.bin" }
}

internal class ChatGptWebFileDownloadLease {
    data class Value(
        val id: String,
        val token: String,
        val generation: Long,
        val href: String,
        val name: String,
        val mediaType: String,
        val expiresAt: Long,
    )
    private var active: Value? = null

    fun begin(token: String, generation: Long, href: String, name: String, mediaType: String, nowMs: Long): Value? {
        if (active?.let { nowMs < it.expiresAt && it.token == token && it.generation == generation && it.href == href } == true) return null
        return Value(UUID.randomUUID().toString(), token, generation, href,
            ChatGptWebFileDownloadPolicy.safeName(name), mediaType, nowMs + 25_000).also { active = it }
    }

    fun consume(id: String, token: String, generation: Long, href: String, nowMs: Long): Value? {
        val value = active ?: return null
        if (id != value.id || token != value.token || generation != value.generation ||
            href != value.href || nowMs >= value.expiresAt) return null
        active = null
        return value
    }

    fun cancel() { active = null }
}
