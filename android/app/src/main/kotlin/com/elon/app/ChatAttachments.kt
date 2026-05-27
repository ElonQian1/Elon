package com.elon.app

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import org.json.JSONArray
import java.io.File

data class ChatAttachment(
    val kind: String? = null,
    val displayName: String? = null,
    val fileName: String? = null,
    val mimeType: String? = null,
    val url: String? = null,
    val localPath: String? = null,
    val sizeBytes: Long? = null,
    val imageWidth: Int? = null,
    val imageHeight: Int? = null
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
            }.getOrNull(),
            imageWidth = item.positiveIntOrNull("image_width"),
            imageHeight = item.positiveIntOrNull("image_height")
        )
    }
}

internal fun chatAttachmentsFromJsonArray(array: JSONArray?): List<ChatAttachment> {
    array ?: return emptyList()
    return List(array.length()) { index -> array.optJSONObject(index) }
        .mapNotNull { item ->
            item ?: return@mapNotNull null
            ChatAttachment(
                kind = item.optString("kind").takeIf { it.isNotBlank() },
                displayName = item.optString("display_name").takeIf { it.isNotBlank() },
                fileName = item.optString("file_name").takeIf { it.isNotBlank() },
                mimeType = item.optString("mime_type").takeIf { it.isNotBlank() },
                url = item.optString("url").takeIf { it.isNotBlank() },
                localPath = item.optString("local_path").takeIf { it.isNotBlank() },
                sizeBytes = item.optLong("size_bytes", 0L).takeIf { it > 0L },
                imageWidth = item.optInt("image_width", 0).takeIf { it > 0 },
                imageHeight = item.optInt("image_height", 0).takeIf { it > 0 }
            )
        }
}

internal fun chatAttachmentsFromPending(attachments: List<PendingAttachment>): List<ChatAttachment> {
    return attachments.map { attachment ->
        ChatAttachment(
            kind = attachment.kind,
            displayName = attachment.displayName,
            fileName = attachment.fileName,
            mimeType = attachment.mimeType,
            localPath = attachment.file.absolutePath,
            sizeBytes = attachment.file.length(),
            imageWidth = attachment.imageWidth,
            imageHeight = attachment.imageHeight
        )
    }
}

internal fun pendingAttachmentsFromChatAttachments(attachments: List<ChatAttachment>): List<PendingAttachment> {
    return attachments.mapNotNull { attachment ->
        val localPath = attachment.localPath?.trim()?.takeIf { it.isNotEmpty() } ?: return@mapNotNull null
        val file = File(localPath)
        if (!file.isFile) return@mapNotNull null
        val fileName = attachment.fileName?.trim()?.takeIf { it.isNotEmpty() } ?: file.name
        val mimeType = attachment.mimeType?.trim()?.takeIf { it.isNotEmpty() } ?: guessMimeType(fileName)
        val kind = attachment.kind?.trim()?.takeIf { it.isNotEmpty() }
            ?: if (mimeType.startsWith("image/")) "image" else "file"
        PendingAttachment(
            kind = kind,
            displayLabel = if (kind == "image" || mimeType.startsWith("image/")) "图片" else "附件",
            displayName = attachment.displayName?.trim()?.takeIf { it.isNotEmpty() } ?: fileName,
            fileName = fileName,
            mimeType = mimeType,
            file = file,
            imageWidth = attachment.imageWidth,
            imageHeight = attachment.imageHeight
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

private fun JsonObject.positiveIntOrNull(name: String): Int? {
    return runCatching {
        get(name)?.takeIf { it.isJsonPrimitive }?.asInt?.takeIf { value -> value > 0 }
    }.getOrNull()
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

internal fun pendingAttachmentSummary(attachments: List<PendingAttachment>): String {
    if (attachments.isEmpty()) return "文本内容在此输入。"
    val imageCount = attachments.count { it.mimeType.startsWith("image/") || it.kind == "image" }
    return when {
        attachments.size == 1 && imageCount == 1 -> "已选择 1 张图片"
        attachments.size == imageCount -> "已选择 $imageCount 张图片"
        attachments.size == 1 -> "已选择 1 个附件"
        imageCount > 0 -> "已选择 ${attachments.size} 个附件，含 $imageCount 张图片"
        else -> "已选择 ${attachments.size} 个附件"
    }
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
