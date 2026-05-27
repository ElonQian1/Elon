package com.elon.app

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.util.LruCache
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import java.io.ByteArrayOutputStream
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import kotlin.math.max

internal fun bindChatAttachmentViews(container: LinearLayout?, attachments: List<ChatAttachment>?) {
    if (container == null) return
    val items = attachments.orEmpty()
    val visibleItems = items.take(MAX_CHAT_ATTACHMENTS)
    val signature = visibleItems.attachmentRenderSignature()
    if (signature.isNotEmpty() && container.tag == signature && container.childCount == visibleItems.size) {
        container.visibility = View.VISIBLE
        return
    }
    container.tag = signature
    container.removeAllViews()
    if (items.isEmpty()) {
        container.visibility = View.GONE
        return
    }
    container.visibility = View.VISIBLE
    visibleItems.forEachIndexed { index, attachment ->
        val view = if (attachment.isImage()) {
            createImageAttachmentView(container.context, attachment)
        } else {
            createFileAttachmentView(container.context, attachment)
        }
        view.layoutParams = (view.layoutParams as LinearLayout.LayoutParams).apply {
            bottomMargin = if (index == visibleItems.lastIndex) 0 else container.context.dp(6)
        }
        container.addView(view)
    }
}

private fun createImageAttachmentView(context: Context, attachment: ChatAttachment): View {
    val image = ImageView(context).apply {
        layoutParams = imageAttachmentLayoutParams(context, attachment)
        background = GradientDrawable().apply {
            cornerRadius = context.dp(7).toFloat()
            setColor(Color.parseColor("#1F1F1F"))
        }
        contentDescription = attachment.displayName ?: "图片"
        scaleType = ImageView.ScaleType.FIT_CENTER
        setPadding(0, 0, 0, 0)
    }

    val source = chatAttachmentImageSource(attachment)
    if (source == null) {
        image.setImageResource(android.R.drawable.ic_menu_report_image)
        return image
    }

    image.tag = source
    image.setOnClickListener {
        ChatImageViewer.show(context, attachment)
    }

    val cached = ChatImagePreviewLoader.cached(source)
    if (cached != null) {
        image.setImageBitmap(cached)
    } else {
        image.setImageResource(android.R.drawable.ic_menu_gallery)
        ChatImagePreviewLoader.load(source) { bitmap ->
            image.post {
                if (image.tag == source) {
                    image.setImageBitmap(bitmap)
                }
            }
        }
    }
    return image
}

private fun imageAttachmentLayoutParams(
    context: Context,
    attachment: ChatAttachment
): LinearLayout.LayoutParams {
    val maxWidth = context.dp(220)
    val maxHeight = context.dp(260)
    val minSide = context.dp(112)
    val sourceWidth = attachment.imageWidth?.takeIf { it > 0 } ?: 4
    val sourceHeight = attachment.imageHeight?.takeIf { it > 0 } ?: 3
    val ratio = sourceWidth.toFloat() / sourceHeight.toFloat()
    var targetWidth = maxWidth
    var targetHeight = (targetWidth / ratio).toInt().coerceAtLeast(minSide)
    if (targetHeight > maxHeight) {
        targetHeight = maxHeight
        targetWidth = (targetHeight * ratio).toInt().coerceAtLeast(minSide)
    }
    return LinearLayout.LayoutParams(
        targetWidth.coerceIn(minSide, maxWidth),
        targetHeight.coerceIn(minSide, maxHeight)
    )
}

private fun createFileAttachmentView(context: Context, attachment: ChatAttachment): View {
    return TextView(context).apply {
        layoutParams = LinearLayout.LayoutParams(context.dp(196), LinearLayout.LayoutParams.WRAP_CONTENT)
        background = GradientDrawable().apply {
            cornerRadius = context.dp(7).toFloat()
            setColor(Color.parseColor("#22FFFFFF"))
            setStroke(context.dp(1), Color.parseColor("#33000000"))
        }
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        maxLines = 2
        setPadding(context.dp(10), context.dp(9), context.dp(10), context.dp(9))
        setTextColor(Color.parseColor("#202020"))
        textSize = 13f
        text = buildString {
            append(attachment.displayName ?: attachment.fileName ?: "附件")
            attachment.sizeBytes?.takeIf { it > 0 }?.let {
                append("\n")
                append(formatAttachmentSize(it))
            }
        }
        attachment.url?.let { url ->
            setOnClickListener {
                runCatching {
                    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
                }
            }
        }
    }
}

internal fun chatAttachmentImageSource(attachment: ChatAttachment): String? {
    val local = attachment.localPath
        ?.takeIf { it.isNotBlank() }
        ?.takeIf { File(it).exists() }
    return local ?: attachment.url?.takeIf { it.isNotBlank() }
}

private fun formatAttachmentSize(bytes: Long): String {
    return if (bytes >= 1_048_576) {
        "%.1f MB".format(bytes / 1_048_576.0)
    } else {
        "${max(1, bytes / 1024)} KB"
    }
}

private fun Context.dp(value: Int): Int {
    return (value * resources.displayMetrics.density).toInt()
}

internal object ChatImagePreviewLoader {
    private const val MAX_IMAGE_BYTES = 12 * 1024 * 1024
    private const val MAX_THUMBNAIL_PIXELS = 1_200_000
    private val cache = object : LruCache<String, Bitmap>(16 * 1024 * 1024) {
        override fun sizeOf(key: String, value: Bitmap): Int = value.byteCount
    }

    fun load(source: String, onReady: (Bitmap) -> Unit) {
        cache.get(source)?.let {
            onReady(it)
            return
        }
        Thread {
            val bitmap = runCatching { loadBitmap(source) }.getOrNull() ?: return@Thread
            cache.put(source, bitmap)
            onReady(bitmap)
        }.start()
    }

    fun cached(source: String): Bitmap? = cache.get(source)

    private fun loadBitmap(source: String): Bitmap {
        val bytes = if (source.startsWith("http://") || source.startsWith("https://")) {
            readUrlBytes(source)
        } else {
            File(source).readBytes()
        }
        return decodeSampledBitmap(bytes)
    }

    private fun readUrlBytes(source: String): ByteArray {
        val connection = (URL(source).openConnection() as HttpURLConnection).apply {
            connectTimeout = 8_000
            readTimeout = 12_000
        }
        return connection.inputStream.use { input ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
            var total = 0
            while (true) {
                val read = input.read(buffer)
                if (read <= 0) break
                total += read
                if (total > MAX_IMAGE_BYTES) error("image preview is too large")
                output.write(buffer, 0, read)
            }
            output.toByteArray()
        }
    }

    private fun decodeSampledBitmap(bytes: ByteArray): Bitmap {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        val options = BitmapFactory.Options().apply {
            inSampleSize = thumbnailSampleSize(bounds.outWidth, bounds.outHeight)
        }
        return BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options)
            ?: error("cannot decode image preview")
    }

    private fun thumbnailSampleSize(width: Int, height: Int): Int {
        if (width <= 0 || height <= 0) return 1
        var sample = 1
        while ((width / sample) * (height / sample) > MAX_THUMBNAIL_PIXELS) {
            sample *= 2
        }
        return sample
    }
}

private fun List<ChatAttachment>.attachmentRenderSignature(): String {
    return joinToString(separator = "\u001F") { attachment ->
        val source = if (attachment.isImage()) chatAttachmentImageSource(attachment).orEmpty() else attachment.url.orEmpty()
        listOf(
            attachment.kind.orEmpty(),
            attachment.mimeType.orEmpty(),
            source,
            attachment.displayName.orEmpty(),
            attachment.fileName.orEmpty(),
            attachment.sizeBytes?.toString().orEmpty(),
            attachment.imageWidth?.toString().orEmpty(),
            attachment.imageHeight?.toString().orEmpty()
        ).joinToString(separator = "\u001E")
    }
}

private const val MAX_CHAT_ATTACHMENTS = 6
