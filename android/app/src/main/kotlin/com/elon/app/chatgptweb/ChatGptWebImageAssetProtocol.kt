package com.elon.app.chatgptweb

import java.util.Base64
import org.json.JSONObject

internal data class ChatGptWebImageAsset(
    val handle: String,
    val state: String,
    val mediaType: String? = null,
    val width: Int? = null,
    val height: Int? = null,
    private val encodedData: String? = null,
    val error: String? = null,
) {
    val ready: Boolean
        get() = state == STATE_READY

    fun decodedBytes(): ByteArray? = encodedData
        ?.let { runCatching { Base64.getDecoder().decode(it) }.getOrNull() }

    companion object {
        const val STATE_READY = "ready"
        const val STATE_FAILED = "failed"
    }
}

internal data class ChatGptWebImageGallerySnapshot(
    val state: String,
    val observedCount: Int,
) {
    companion object {
        const val STATE_LOADING = "loading"
        const val STATE_READY = "ready"
        const val STATE_FAILED = "failed"
    }
}

internal object ChatGptWebImageAssetProtocol {
    fun parseAsset(event: JSONObject): ChatGptWebImageAsset? {
        val handle = event.optString("handle").takeIf(HANDLE::matches) ?: return null
        return when (val state = event.optString("state")) {
            ChatGptWebImageAsset.STATE_READY -> {
                val mediaType = event.optString("mediaType").takeIf(MEDIA_TYPES::contains)
                    ?: return null
                val width = event.optInt("width", 0).takeIf { it in 1..MAX_DIMENSION }
                    ?: return null
                val height = event.optInt("height", 0).takeIf { it in 1..MAX_DIMENSION }
                    ?: return null
                val data = event.optString("data")
                    .takeIf { it.length in MIN_BASE64_LENGTH..MAX_BASE64_LENGTH }
                    ?.takeIf(::isBase64)
                    ?: return null
                ChatGptWebImageAsset(handle, state, mediaType, width, height, data)
            }
            ChatGptWebImageAsset.STATE_FAILED -> ChatGptWebImageAsset(
                handle = handle,
                state = state,
                error = event.optString("error").takeIf(ERRORS::contains) ?: "fetch_failed",
            )
            else -> null
        }
    }

    fun parseGallery(event: JSONObject): ChatGptWebImageGallerySnapshot? {
        val state = event.optString("state").takeIf(GALLERY_STATES::contains) ?: return null
        return ChatGptWebImageGallerySnapshot(
            state = state,
            observedCount = event.optInt("observedCount", 0).coerceIn(0, MAX_GALLERY_IMAGES),
        )
    }

    fun validHandle(value: String): Boolean = HANDLE.matches(value)

    private fun isBase64(value: String): Boolean = value.all { character ->
        character in 'A'..'Z' || character in 'a'..'z' || character in '0'..'9' ||
            character == '+' || character == '/' || character == '='
    }

    private const val MAX_DIMENSION = 4_096
    private const val MIN_BASE64_LENGTH = 16
    private const val MAX_BASE64_LENGTH = 1_400_000
    private const val MAX_GALLERY_IMAGES = 96
    private val HANDLE = Regex("image_[a-f0-9]{16}")
    private val MEDIA_TYPES = setOf("image/jpeg", "image/png", "image/webp")
    private val ERRORS = setOf(
        "invalid_request",
        "unknown_handle",
        "http_error",
        "source_too_large",
        "not_image",
        "canvas_unavailable",
        "preview_too_large",
        "fetch_failed",
    )
    private val GALLERY_STATES = setOf(
        ChatGptWebImageGallerySnapshot.STATE_LOADING,
        ChatGptWebImageGallerySnapshot.STATE_READY,
        ChatGptWebImageGallerySnapshot.STATE_FAILED,
    )
}
