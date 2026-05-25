package com.elon.app

import com.google.gson.JsonArray
import com.google.gson.JsonObject

data class ChatAttachment(
    val kind: String? = null,
    val displayName: String? = null,
    val fileName: String? = null,
    val mimeType: String? = null,
    val url: String? = null,
    val localPath: String? = null,
    val sizeBytes: Long? = null
) {
    fun isImage(): Boolean {
        return kind == "image" || mimeType.orEmpty().startsWith("image/")
    }
}

internal fun chatAttachmentsFromRefs(refs: JsonArray): List<ChatAttachment> {
    return refs.mapNotNull { element ->
        if (!element.isJsonObject) return@mapNotNull null
        val item = element.asJsonObject
        ChatAttachment(
            kind = item.stringOrNull("kind"),
            displayName = item.stringOrNull("display_name"),
            fileName = item.stringOrNull("file_name"),
            mimeType = item.stringOrNull("mime_type"),
            url = item.stringOrNull("url"),
            localPath = item.stringOrNull("local_path"),
            sizeBytes = runCatching {
                item.get("size_bytes")?.takeIf { it.isJsonPrimitive }?.asLong
            }.getOrNull()
        )
    }
}

private fun JsonObject.stringOrNull(name: String): String? {
    return get(name)
        ?.takeIf { it.isJsonPrimitive }
        ?.asString
        ?.trim()
        ?.takeIf { it.isNotEmpty() }
}

internal fun chatAttachmentFromImageUrl(url: String?): List<ChatAttachment> {
    val imageUrl = url?.trim()?.takeIf { it.isNotEmpty() } ?: return emptyList()
    return listOf(
        ChatAttachment(
            kind = "image",
            displayName = imageUrl.substringAfterLast('/').substringBefore('?').ifBlank { "图片" },
            mimeType = guessMimeType(imageUrl.substringBefore('?')),
            url = imageUrl
        )
    )
}

internal fun visibleTextForPendingAttachments(rawText: String, attachments: List<PendingAttachment>): String {
    val cleaned = rawText
        .lines()
        .filterNot { line ->
            val trimmed = line.trim()
            attachments.any { attachment ->
                trimmed == "[${attachment.displayLabel}] ${attachment.displayName}"
            }
        }
        .joinToString("\n")
        .trim()
    if (cleaned.isNotBlank() || attachments.isEmpty()) return cleaned
    return defaultPendingAttachmentMessage(attachments)
}

private fun defaultPendingAttachmentMessage(attachments: List<PendingAttachment>): String {
    val imageCount = attachments.count { it.mimeType.startsWith("image/") || it.kind == "image" }
    return when {
        attachments.size == 1 && imageCount == 1 -> "请看这张图片。"
        attachments.size == imageCount -> "请看这些图片。"
        attachments.size == 1 -> "请看这个附件。"
        else -> "请看这些附件。"
    }
}
