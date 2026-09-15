package com.elon.app

import android.content.Context
import android.graphics.BitmapFactory
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext

internal data class AiConversationShareMediaKey(val messageId: String, val location: Location, val index: Int) {
    enum class Location { CONTENT_PART, ATTACHMENT }
}

// These are byte-preparation types, not the parent-owned share/API models.
internal class AiConversationShareMediaUpload(
    val key: String,
    val bytes: ByteArray,
    val mimeType: String,
    val fileName: String,
    val width: Int,
    val height: Int,
    val isPreview: Boolean,
)

internal data class AiConversationShareMediaImageInfo(val mimeType: String, val width: Int, val height: Int)

internal class AiConversationShareMediaException(val reason: Reason) : Exception(reason.name) {
    enum class Reason {
        INVALID_SELECTION, IMAGE_PENDING_RETRY, UNSUPPORTED_PART, UNSUPPORTED_ATTACHMENT,
        INCOMPLETE_TEXT_BLOCK_RETRY, SOURCE_NOT_ALLOWED, LOCAL_IMAGE_MISSING_RETRY,
        ASSET_TOO_LARGE, BATCH_TOO_LARGE, INVALID_IMAGE,
    }

    fun userMessage(): String = when (reason) {
        Reason.INVALID_SELECTION -> "所选消息已变化，请重新选择后分享"
        Reason.IMAGE_PENDING_RETRY -> "所选图片尚未准备完成，请等待图片显示后重试"
        Reason.UNSUPPORTED_PART -> "所选消息包含暂不支持分享的内容，请取消选择该消息后重试"
        Reason.UNSUPPORTED_ATTACHMENT -> "暂不支持分享文件、音视频或带批注的附件，请取消选择该消息后重试"
        Reason.INCOMPLETE_TEXT_BLOCK_RETRY -> "所选写作或代码块尚未完整，请等待内容加载完成后重试"
        Reason.SOURCE_NOT_ALLOWED -> "所选图片不是可分享的应用缓存，请重新打开图片后重试"
        Reason.LOCAL_IMAGE_MISSING_RETRY -> "所选图片的本地缓存已失效，请重新打开图片后重试"
        Reason.ASSET_TOO_LARGE -> "单张图片不能超过 12 MiB，请减少图片大小后重试"
        Reason.BATCH_TOO_LARGE -> "一次最多分享 12 张图片，图片总大小不能超过 48 MiB，请减少所选内容"
        Reason.INVALID_IMAGE -> "所选图片无法读取、格式不支持或尺寸超限；宽高须为 1 至 4096 像素，请检查后重试"
    }
}

/** Prepares only explicit selected-message sources. Never resolves private handles or fetches URLs. */
internal class AiConversationShareMediaPreparer(
    private val sources: AiConversationShareMediaSources,
    private val inspectImage: (ByteArray) -> AiConversationShareMediaImageInfo?,
) {
    constructor(context: Context) : this(
        AiConversationShareMediaSources(context.applicationContext.cacheDir, context.packageName),
        ::inspectShareImage,
    )

    /** Compatibility with the parent's frozen-message Codec, without duplicating source handling. */
    @JvmName("prepareSelectedMessages")
    suspend fun <T : Any> prepare(
        selectedMessages: List<ChatMessage>,
        upload: suspend (AiConversationShareMediaUpload) -> T,
    ): Map<AiConversationShareMediaKey, T> {
        val plan = AiConversationShareMedia.plan(selectedMessages)
        val keys = plan.associate { source -> source.key to
            (source.messageKey ?: fail(AiConversationShareMediaException.Reason.INVALID_SELECTION)) }
        if (keys.values.toSet().size != keys.size) fail(AiConversationShareMediaException.Reason.INVALID_SELECTION)
        return prepare(plan, upload).mapKeys { keys.getValue(it.key) }
    }

    /**
     * The parent owns group authorization, upload receipts, markdown/text-block serialization and
     * final share creation. Use AiConversationShareMedia.plan(draft.messages), bind the callback
     * to one selected group, and never persist the source plan.
     * All local bytes validate before any upload. Callback failures/cancellation propagate unchanged;
     * the parent must not create a partial share and the server must expire uncommitted assets.
     */
    suspend fun <T : Any> prepare(
        sources: List<AiConversationShareMedia.Source>,
        upload: suspend (AiConversationShareMediaUpload) -> T,
    ): Map<String, T> {
        val requests = sources.toList()
        if (requests.size > MAX_ASSETS) fail(AiConversationShareMediaException.Reason.BATCH_TOO_LARGE)
        if (requests.map { it.key }.toSet().size != requests.size ||
            requests.any { !SOURCE_KEY.matches(it.key) || it.type != "image" }
        ) fail(AiConversationShareMediaException.Reason.INVALID_SELECTION)
        return withContext(Dispatchers.IO) {
            val coroutineContext = currentCoroutineContext()
            var totalBytes = 0L
            val prepared = requests.mapIndexed { index, request ->
                coroutineContext.ensureActive()
                val source = this@AiConversationShareMediaPreparer.sources.read(request.source, request.assetHandle) {
                    coroutineContext.ensureActive()
                }
                totalBytes += source.bytes.size
                if (totalBytes > MAX_BATCH_BYTES) fail(AiConversationShareMediaException.Reason.BATCH_TOO_LARGE)
                val info = inspectImage(source.bytes)
                    ?: fail(AiConversationShareMediaException.Reason.INVALID_IMAGE)
                val extension = EXTENSIONS[info.mimeType]
                    ?: fail(AiConversationShareMediaException.Reason.INVALID_IMAGE)
                if (info.width !in 1..MAX_DIMENSION || info.height !in 1..MAX_DIMENSION ||
                    info.width.toLong() * info.height > MAX_PIXELS
                ) fail(AiConversationShareMediaException.Reason.INVALID_IMAGE)
                AiConversationShareMediaUpload(request.key, source.bytes, info.mimeType, "image-${index + 1}.$extension",
                    info.width, info.height, source.isPreview)
            }
            val result = linkedMapOf<String, T>()
            prepared.forEachIndexed { index, asset ->
                currentCoroutineContext().ensureActive()
                result[requests[index].key] = upload(asset)
            }
            result
        }
    }

    companion object {
        const val MAX_ASSET_BYTES = 12 * 1024 * 1024
        const val MAX_BATCH_BYTES = 48L * 1024 * 1024
        const val MAX_ASSETS = 12
        private const val MAX_DIMENSION = 4_096
        private const val MAX_PIXELS = 40_000_000L
        private val SOURCE_KEY = Regex("m[0-9]{1,3}[pa][0-9]{1,3}")
        private val EXTENSIONS = mapOf("image/jpeg" to "jpg", "image/png" to "png", "image/webp" to "webp")

        private fun inspectShareImage(bytes: ByteArray): AiConversationShareMediaImageInfo? {
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
            val mime = bounds.outMimeType?.takeIf(EXTENSIONS::containsKey) ?: return null
            return AiConversationShareMediaImageInfo(mime, bounds.outWidth, bounds.outHeight)
        }

        private fun fail(reason: AiConversationShareMediaException.Reason): Nothing =
            throw AiConversationShareMediaException(reason)
    }
}

internal object AiConversationShareMedia {
    // Source stays local. In particular, do not serialize it or log its path/URL/MIME hints.
    class Source internal constructor(
        val key: String,
        val source: String,
        val name: String,
        val mimeType: String?,
        val type: String = "image",
        internal val assetHandle: String? = null,
        internal val messageKey: AiConversationShareMediaKey? = null,
    )

    fun partKey(messageIndex: Int, partIndex: Int): String = "m${messageIndex}p$partIndex"
    fun attachmentKey(messageIndex: Int, attachmentIndex: Int): String = "m${messageIndex}a$attachmentIndex"
    fun firstImageKey(sources: List<Source>): String? = sources.firstOrNull { it.type == "image" }?.key

    /** Keys use the frozen selected draft's order, not source-history indices or provider IDs. */
    fun plan(messages: List<ChatMessage>): List<Source> {
        val selected = messages.toList()
        if (selected.isEmpty() || selected.size > MAX_MESSAGES) {
            fail(AiConversationShareMediaException.Reason.INVALID_SELECTION)
        }
        val requests = mutableListOf<Source>()
        selected.forEachIndexed { messageIndex, message ->
            val parts = message.webChatMessage?.contentParts.orEmpty().toList()
            val attachments = message.attachments.orEmpty().toList()
            if (parts.size > MAX_PARTS || attachments.size > MAX_ASSETS) {
                fail(AiConversationShareMediaException.Reason.BATCH_TOO_LARGE)
            }
            parts.forEachIndexed partLoop@ { index, part ->
                if (part.richCard != null) return@partLoop
                part.textBlock?.let { block ->
                    if (!block.complete) fail(AiConversationShareMediaException.Reason.INCOMPLETE_TEXT_BLOCK_RETRY)
                    if (block.kind !in setOf("writing", "code")) {
                        fail(AiConversationShareMediaException.Reason.UNSUPPORTED_PART)
                    }
                    return@partLoop
                }
                when (part.type) {
                    "image" -> {
                        if (part.previewPending || !message.sendStatus.isNullOrBlank()) {
                            fail(AiConversationShareMediaException.Reason.IMAGE_PENDING_RETRY)
                        }
                        val source = part.imageSource?.takeIf(String::isNotBlank)
                            ?: fail(AiConversationShareMediaException.Reason.IMAGE_PENDING_RETRY)
                        val key = partKey(messageIndex, index)
                        requests += Source(key, source, "image-$key", part.mediaType, assetHandle = part.assetHandle,
                            messageKey = message.id?.let {
                                AiConversationShareMediaKey(it, AiConversationShareMediaKey.Location.CONTENT_PART, index)
                            })
                    }
                    "writing_block" -> fail(AiConversationShareMediaException.Reason.INCOMPLETE_TEXT_BLOCK_RETRY)
                    "code", "citation", "table", "math" -> {
                        if (message.content.isBlank()) fail(AiConversationShareMediaException.Reason.UNSUPPORTED_PART)
                    }
                    else -> fail(AiConversationShareMediaException.Reason.UNSUPPORTED_PART)
                }
            }
            attachments.forEachIndexed { index, attachment ->
                if (!attachment.isImage() || attachment.annotations.isNotEmpty()) {
                    fail(AiConversationShareMediaException.Reason.UNSUPPORTED_ATTACHMENT)
                }
                if (!message.sendStatus.isNullOrBlank()) {
                    fail(AiConversationShareMediaException.Reason.IMAGE_PENDING_RETRY)
                }
                val source = attachment.localPath?.takeIf(String::isNotBlank)
                    ?: attachment.url?.takeIf(String::isNotBlank)
                    ?: fail(AiConversationShareMediaException.Reason.LOCAL_IMAGE_MISSING_RETRY)
                val key = attachmentKey(messageIndex, index)
                requests += Source(key, source, "image-$key", attachment.mimeType,
                    messageKey = message.id?.let {
                        AiConversationShareMediaKey(it, AiConversationShareMediaKey.Location.ATTACHMENT, index)
                    })
            }
            if (requests.size > MAX_ASSETS) fail(AiConversationShareMediaException.Reason.BATCH_TOO_LARGE)
        }
        return requests
    }

    private const val MAX_MESSAGES = 200
    private const val MAX_PARTS = 64
    private const val MAX_ASSETS = AiConversationShareMediaPreparer.MAX_ASSETS
    private fun fail(reason: AiConversationShareMediaException.Reason): Nothing =
        throw AiConversationShareMediaException(reason)
}
