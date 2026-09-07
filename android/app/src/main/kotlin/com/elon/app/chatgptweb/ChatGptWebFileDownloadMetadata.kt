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
        var destination = ChatGptWebFileDownloadPolicy.safeName(name)
        // A long cloud document title must not truncate its exported format suffix.
        if (name.length > 150 && suffix.isNotEmpty()) destination = destination.take(150 - suffix.length) + suffix
        return lease.copy(name = destination, mediaType = mediaType.lowercase(Locale.ROOT))
    }
}
