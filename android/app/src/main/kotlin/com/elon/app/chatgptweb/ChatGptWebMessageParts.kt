package com.elon.app.chatgptweb

import org.json.JSONObject

internal data class ChatGptWebMessagePart(
    val type: String,
    val label: String,
    val metadata: ChatGptWebMessagePartMetadata? = null,
)

internal data class ChatGptWebMessagePartMetadata(
    val kind: String? = null,
    val language: String? = null,
    val mediaType: String? = null,
    val targetKind: String? = null,
    val targetHost: String? = null,
    val assetHandle: String? = null,
    val imageWidth: Int? = null,
    val imageHeight: Int? = null,
    val lineCount: Int? = null,
    val rowCount: Int? = null,
    val columnCount: Int? = null,
) {
    val isEmpty: Boolean
        get() = kind == null &&
            language == null &&
            mediaType == null &&
            targetKind == null &&
            targetHost == null &&
            assetHandle == null &&
            imageWidth == null &&
            imageHeight == null &&
            lineCount == null &&
            rowCount == null &&
            columnCount == null
}

internal object ChatGptWebMessagePartParser {
    fun parse(message: JSONObject): List<ChatGptWebMessagePart> {
        val content = message.optJSONArray("content") ?: return emptyList()
        return buildList {
            for (index in 0 until minOf(content.length(), MAX_CONTENT_PARTS)) {
                val part = content.optJSONObject(index) ?: continue
                val type = part.optString("type")
                if (type !in SUPPORTED_TYPES) continue
                val label = part.optString("text").trim().take(MAX_LABEL_LENGTH)
                if (label.isBlank()) continue
                add(ChatGptWebMessagePart(type, label, parseMetadata(part)))
            }
        }.take(MAX_PARTS)
    }

    private fun parseMetadata(part: JSONObject): ChatGptWebMessagePartMetadata? {
        val metadata = ChatGptWebMessagePartMetadata(
            kind = boundedToken(part, "kind", KIND, MAX_KIND_LENGTH),
            language = boundedToken(part, "language", LANGUAGE, MAX_LANGUAGE_LENGTH),
            mediaType = boundedToken(part, "mediaType", MEDIA_TYPE, MAX_MEDIA_TYPE_LENGTH),
            targetKind = part.optString("targetKind").takeIf(TARGET_KINDS::contains),
            targetHost = boundedToken(part, "targetHost", TARGET_HOST, MAX_TARGET_HOST_LENGTH),
            assetHandle = part.optString("assetHandle")
                .takeIf(ChatGptWebImageAssetProtocol::validHandle),
            imageWidth = boundedCount(part, "imageWidth", MAX_IMAGE_DIMENSION),
            imageHeight = boundedCount(part, "imageHeight", MAX_IMAGE_DIMENSION),
            lineCount = boundedCount(part, "lineCount", MAX_LINE_COUNT),
            rowCount = boundedCount(part, "rowCount", MAX_TABLE_DIMENSION),
            columnCount = boundedCount(part, "columnCount", MAX_TABLE_DIMENSION),
        )
        return metadata.takeUnless { it.isEmpty }
    }

    private fun boundedToken(
        part: JSONObject,
        key: String,
        pattern: Regex,
        maxLength: Int,
    ): String? = part.optString(key)
        .trim()
        .take(maxLength)
        .takeIf(pattern::matches)

    private fun boundedCount(part: JSONObject, key: String, max: Int): Int? =
        part.optInt(key, 0).takeIf { it in 1..max }

    private const val MAX_CONTENT_PARTS = 20
    private const val MAX_PARTS = 16
    private const val MAX_LABEL_LENGTH = 180
    private const val MAX_KIND_LENGTH = 32
    private const val MAX_LANGUAGE_LENGTH = 32
    private const val MAX_MEDIA_TYPE_LENGTH = 96
    private const val MAX_TARGET_HOST_LENGTH = 253
    private const val MAX_IMAGE_DIMENSION = 4_096
    private const val MAX_LINE_COUNT = 1_000_000
    private const val MAX_TABLE_DIMENSION = 10_000
    private val SUPPORTED_TYPES = setOf(
        "image",
        "file",
        "citation",
        "code",
        "table",
        "artifact",
        "audio",
        "video",
        "math",
        "chart",
        "map",
        "interactive",
    )
    private val TARGET_KINDS = setOf("same_origin", "external")
    private val KIND = Regex("[a-z][a-z0-9_]{0,31}")
    private val LANGUAGE = Regex("[A-Za-z0-9_+.#-]{1,32}")
    private val MEDIA_TYPE = Regex("[A-Za-z0-9.+-]{1,63}/[A-Za-z0-9.+-]{1,63}")
    private val TARGET_HOST = Regex("[A-Za-z0-9.-]{1,253}")
}
