package com.elon.app

import java.net.URI

internal object WebChatSourceLinkPolicy {
    fun normalize(raw: String): String? {
        if (raw.length > 8192) return null
        val value = raw.trim()
        if (value.isEmpty() || value.any { it <= ' ' || it == '\u007f' || it == '\\' }) return null
        val uri = runCatching { URI(value) }.getOrNull() ?: return null
        if (!uri.scheme.equals("https", ignoreCase = true) || uri.host.isNullOrBlank() ||
            uri.rawUserInfo != null || uri.port !in setOf(-1, 443)) return null
        return uri.toASCIIString().takeIf { it.length <= 8192 }
    }

    fun currentUrl(file: WebChatConversationFile, index: WebChatConversationFileIndex?, nowMs: Long): String? {
        if (file.kind != "source" || file.downloadHandle.isNotEmpty() || index == null || !index.isFresh(nowMs) ||
            index.files.none { it == file }) return null
        return normalize(file.sourceUrl)
    }
}
