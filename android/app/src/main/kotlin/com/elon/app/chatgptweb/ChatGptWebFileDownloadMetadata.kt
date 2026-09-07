package com.elon.app.chatgptweb

import org.json.JSONObject
import java.util.Locale

/** Resolves materialized metadata only after the gateway consumes the original download lease. */
internal object ChatGptWebFileDownloadMetadata {
    const val VERSION = 1
    private val fields = setOf("version", "name", "mediaType")
    private val mime = Regex("[A-Za-z0-9.+-]{1,63}/[A-Za-z0-9.+-]{1,63}")
    private val extension = Regex("\\.([A-Za-z0-9]{1,16})$")

    fun resolve(lease: ChatGptWebFileDownloadLease.Value, packet: JSONObject): ChatGptWebFileDownloadLease.Value? {
        if (!packet.has("resolvedFile")) return lease
        val value = packet.opt("resolvedFile") as? JSONObject ?: return null
        if (value.keys().asSequence().toSet() != fields || value.opt("version") != VERSION) return null
        val name = value.opt("name") as? String ?: return null
        val mediaType = value.opt("mediaType") as? String ?: return null
        if (name.length !in 1..1024 || name.isBlank() || name.any { it < ' ' || it == '\u007f' } ||
            mediaType.isNotEmpty() && !mime.matches(mediaType)) return null
        val suffix = extension.find(name.trimEnd())?.value.orEmpty()
        val destination = ChatGptWebFileDownloadPolicy.safeName(name, maxLength = 1024)
        // Bound complete code points while reserving the exported suffix and native prefix.
        val stem = destination.removeSuffix(suffix)
        val bounded = StringBuilder()
        var remaining = 255 - "elon-${lease.id}-$suffix".toByteArray(Charsets.UTF_8).size
        var offset = 0
        while (offset < stem.length) {
            val codePoint = stem.codePointAt(offset)
            if (codePoint in 0xd800..0xdfff) return null
            val part = String(Character.toChars(codePoint))
            val size = part.toByteArray(Charsets.UTF_8).size
            if (size > remaining || bounded.length + part.length > 150 - suffix.length) break
            bounded.append(part)
            remaining -= size
            offset += Character.charCount(codePoint)
        }
        return lease.copy(name = bounded.toString() + suffix, mediaType = mediaType.lowercase(Locale.ROOT))
    }
}
