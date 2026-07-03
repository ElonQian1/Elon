package com.elon.app

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import org.json.JSONArray
import org.json.JSONObject
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
    val imageHeight: Int? = null,
    val durationSeconds: Int? = null,
    val transcription: String? = null,
    val annotations: List<ChatImageAnnotation> = emptyList()
) {
    fun isImage(): Boolean {
        return kind == "image" || mimeType.orEmpty().startsWith("image/")
    }

    fun isVoice(): Boolean {
        return kind == "audio" || mimeType.orEmpty().startsWith("audio/")
    }

    /** 优先使用本地文件路径（文件存在时），否则退回服务器 URL。 */
    fun playbackSource(): String? {
        val local = localPath?.trim()?.takeIf { it.isNotEmpty() && java.io.File(it).exists() }
        return local ?: url?.trim()?.takeIf { it.isNotEmpty() }
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
            imageHeight = item.positiveIntOrNull("image_height"),
            durationSeconds = item.positiveIntOrNull("duration_seconds"),
            transcription = item.stringOrNull("transcription"),
            annotations = chatImageAnnotationsFromGsonArray(item.arrayOrNull("annotations"))
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
                imageHeight = item.optInt("image_height", 0).takeIf { it > 0 },
                durationSeconds = item.optInt("duration_seconds", 0).takeIf { it > 0 },
                transcription = item.optString("transcription").takeIf { it.isNotBlank() },
                annotations = chatImageAnnotationsFromJsonArray(item.optJSONArray("annotations"))
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
            imageHeight = attachment.imageHeight,
            durationSeconds = attachment.durationSeconds,
            transcription = attachment.transcription,
            annotations = attachment.annotations
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
            imageHeight = attachment.imageHeight,
            annotations = attachment.annotations
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

private fun JsonObject.arrayOrNull(name: String): JsonArray? {
    return get(name)?.takeIf { it.isJsonArray }?.asJsonArray
}

private fun JsonObject.floatOrNull(name: String): Float? {
    return runCatching {
        get(name)?.takeIf { it.isJsonPrimitive }?.asFloat
    }.getOrNull()
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

internal fun chatImageAnnotationsToJsonString(annotations: List<ChatImageAnnotation>): String {
    return chatImageAnnotationsToJsonArray(annotations).toString()
}

internal fun chatImageAnnotationsFromJsonString(raw: String?): List<ChatImageAnnotation> {
    val text = raw?.trim()?.takeIf { it.isNotEmpty() } ?: return emptyList()
    return runCatching { chatImageAnnotationsFromJsonArray(JSONArray(text)) }.getOrDefault(emptyList())
}

internal fun chatImageAnnotationsToGsonArray(annotations: List<ChatImageAnnotation>): JsonArray {
    return JsonArray().apply {
        annotations.mapNotNull { it.normalizedForTransport() }.forEach { annotation ->
            add(JsonObject().apply {
                addProperty("x", annotation.x)
                addProperty("y", annotation.y)
                addProperty("width", annotation.width)
                addProperty("height", annotation.height)
                addProperty("note", annotation.note)
                annotation.iconX?.let { addProperty("icon_x", it) }
                annotation.iconY?.let { addProperty("icon_y", it) }
                annotation.iconWidth?.let { addProperty("icon_width", it) }
                annotation.iconHeight?.let { addProperty("icon_height", it) }
            })
        }
    }
}

private fun chatImageAnnotationsToJsonArray(annotations: List<ChatImageAnnotation>): JSONArray {
    return JSONArray().apply {
        annotations.mapNotNull { it.normalizedForTransport() }.forEach { annotation ->
            put(JSONObject().apply {
                put("x", annotation.x)
                put("y", annotation.y)
                put("width", annotation.width)
                put("height", annotation.height)
                put("note", annotation.note)
                annotation.iconX?.let { put("icon_x", it) }
                annotation.iconY?.let { put("icon_y", it) }
                annotation.iconWidth?.let { put("icon_width", it) }
                annotation.iconHeight?.let { put("icon_height", it) }
            })
        }
    }
}

private fun chatImageAnnotationsFromGsonArray(array: JsonArray?): List<ChatImageAnnotation> {
    array ?: return emptyList()
    return array.mapNotNull { element ->
        val item = element.takeIf { it.isJsonObject }?.asJsonObject ?: return@mapNotNull null
        ChatImageAnnotation(
            x = item.floatOrNull("x") ?: return@mapNotNull null,
            y = item.floatOrNull("y") ?: return@mapNotNull null,
            width = item.floatOrNull("width") ?: return@mapNotNull null,
            height = item.floatOrNull("height") ?: return@mapNotNull null,
            note = item.stringOrNull("note").orEmpty(),
            iconX = item.floatOrNull("icon_x"),
            iconY = item.floatOrNull("icon_y"),
            iconWidth = item.floatOrNull("icon_width"),
            iconHeight = item.floatOrNull("icon_height")
        ).normalizedForTransport()
    }
}

private fun chatImageAnnotationsFromJsonArray(array: JSONArray?): List<ChatImageAnnotation> {
    array ?: return emptyList()
    return List(array.length()) { index -> array.optJSONObject(index) }
        .mapNotNull { item ->
            item ?: return@mapNotNull null
            ChatImageAnnotation(
                x = item.optDoubleOrNull("x")?.toFloat() ?: return@mapNotNull null,
                y = item.optDoubleOrNull("y")?.toFloat() ?: return@mapNotNull null,
                width = item.optDoubleOrNull("width")?.toFloat() ?: return@mapNotNull null,
                height = item.optDoubleOrNull("height")?.toFloat() ?: return@mapNotNull null,
                note = item.optString("note").orEmpty(),
                iconX = item.optDoubleOrNull("icon_x")?.toFloat(),
                iconY = item.optDoubleOrNull("icon_y")?.toFloat(),
                iconWidth = item.optDoubleOrNull("icon_width")?.toFloat(),
                iconHeight = item.optDoubleOrNull("icon_height")?.toFloat()
            ).normalizedForTransport()
        }
}

private fun ChatImageAnnotation.normalizedForTransport(): ChatImageAnnotation? {
    val cleanNote = note.trim()
    if (cleanNote.isEmpty()) return null
    val cleanWidth = width.coerceIn(0f, 1f)
    val cleanHeight = height.coerceIn(0f, 1f)
    if (cleanWidth <= 0f || cleanHeight <= 0f) return null
    val cleanX = x.coerceIn(0f, 1f)
    val cleanY = y.coerceIn(0f, 1f)
    val boundedWidth = minOf(cleanWidth, 1f - cleanX)
    val boundedHeight = minOf(cleanHeight, 1f - cleanY)
    if (boundedWidth <= 0f || boundedHeight <= 0f) return null
    return copy(
        x = cleanX,
        y = cleanY,
        width = boundedWidth,
        height = boundedHeight,
        note = cleanNote,
        iconX = iconX?.coerceIn(-0.25f, 1.25f),
        iconY = iconY?.coerceIn(-0.25f, 1.25f),
        iconWidth = iconWidth?.coerceIn(0f, 1f),
        iconHeight = iconHeight?.coerceIn(0f, 1f)
    )
}

private fun JSONObject.optDoubleOrNull(name: String): Double? {
    if (!has(name) || isNull(name)) return null
    return optDouble(name).takeIf { !it.isNaN() && !it.isInfinite() }
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
    return cleaned
}

internal fun pendingAttachmentSummary(attachments: List<PendingAttachment>): String {
    if (attachments.isEmpty()) return "文本内容在此输入。"
    val imageCount = attachments.count { it.mimeType.startsWith("image/") || it.kind == "image" }
    return when {
        attachments.size == 1 && imageCount == 1 -> "已选择 1 张图片"
        attachments.size == imageCount -> "已选择 $imageCount 张图片"
        attachments.size == 1 && attachments[0].kind == "audio" -> "已选择 1 条语音"
        attachments.size == 1 -> "已选择 1 个附件"
        attachments.any { it.kind == "audio" } -> "已选择 ${attachments.size} 个附件，含语音"
        imageCount > 0 -> "已选择 ${attachments.size} 个附件，含 $imageCount 张图片"
        else -> "已选择 ${attachments.size} 个附件"
    }
}
