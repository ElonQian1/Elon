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
import android.widget.ProgressBar
import android.widget.TextView
import java.io.ByteArrayOutputStream
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import kotlin.math.max

internal fun bindChatAttachmentViews(
    container: LinearLayout?,
    attachments: List<ChatAttachment>?,
    onVoiceLongPress: ((ChatAttachment) -> Unit)? = null
) {
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
        val view = when {
            attachment.isImage() -> createImageAttachmentView(container.context, attachment)
            attachment.isVoice() -> createVoiceAttachmentView(container.context, attachment, onVoiceLongPress)
            else -> createFileAttachmentView(container.context, attachment)
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

/**
 * 语音消息气泡：显示 ▶/⏸ 播放按钮 + 进度条 + 时长。
 * 宽度按录音时长动态伸缩（最窄 100dp，最宽 220dp）。
 */
private fun createVoiceAttachmentView(
    context: Context,
    attachment: ChatAttachment,
    onLongPress: ((ChatAttachment) -> Unit)?
): View {
    val source = attachment.playbackSource() ?: ""
    val durationSec = attachment.durationSeconds ?: 0
    val durationText = formatVoiceDuration(durationSec)

    // 宽度按时长动态计算：1 秒 = +2dp，最小 100dp，最大 220dp
    val widthDp = (100 + durationSec.coerceIn(0, 60) * 2).coerceIn(100, 220)

    val container = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        layoutParams = LinearLayout.LayoutParams(context.dp(widthDp), LinearLayout.LayoutParams.WRAP_CONTENT)
        setPadding(context.dp(10), context.dp(10), context.dp(10), context.dp(10))
        background = GradientDrawable().apply {
            cornerRadius = context.dp(20).toFloat()
            setColor(Color.parseColor("#1A73E8"))
        }
        isClickable = true
        isFocusable = true
    }

    // ▶ / ⏸ 播放按钮
    val playBtn = TextView(context).apply {
        text = if (VoiceMessagePlayer.isCurrentlyPlaying(source)) "⏸" else "▶"
        textSize = 16f
        setTextColor(Color.WHITE)
        gravity = Gravity.CENTER
        layoutParams = LinearLayout.LayoutParams(context.dp(28), context.dp(28))
    }

    // 播放进度条
    val progressBar = ProgressBar(context, null, android.R.attr.progressBarStyleHorizontal).apply {
        layoutParams = LinearLayout.LayoutParams(0, context.dp(3), 1f).apply {
            marginStart = context.dp(6)
            marginEnd = context.dp(6)
        }
        max = 1000
        progress = 0
        progressTintList = android.content.res.ColorStateList.valueOf(Color.WHITE)
        progressBackgroundTintList = android.content.res.ColorStateList.valueOf(Color.parseColor("#66FFFFFF"))
    }

    // 时长文字（固定宽度避免气泡跳动）
    val durationView = TextView(context).apply {
        text = durationText
        textSize = 11f
        setTextColor(Color.WHITE)
        gravity = Gravity.CENTER
        layoutParams = LinearLayout.LayoutParams(context.dp(30), LinearLayout.LayoutParams.WRAP_CONTENT)
        includeFontPadding = false
    }

    container.addView(playBtn)
    container.addView(progressBar)
    container.addView(durationView)

    // 注册播放状态监听，随时更新此气泡的 UI
    val stateListener: (String, Boolean, Int, Int) -> Unit = listener@{ src, isPlaying, posMs, durMs ->
        if (src != source) return@listener
        container.post {
            playBtn.text = if (isPlaying) "⏸" else "▶"
            if (durMs > 0) {
                progressBar.progress = (posMs * 1000L / durMs).toInt()
            } else {
                progressBar.progress = 0
            }
            if (!isPlaying) {
                progressBar.progress = 0
            }
        }
    }
    VoiceMessagePlayer.addStateListener(stateListener)

    // 视图从窗口 detach 时移除监听，防止内存泄漏
    container.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
        override fun onViewAttachedToWindow(v: View) = Unit
        override fun onViewDetachedFromWindow(v: View) {
            VoiceMessagePlayer.removeStateListener(stateListener)
        }
    })

    // 点击：播放 / 暂停
    container.setOnClickListener {
        if (source.isBlank()) return@setOnClickListener
        VoiceMessagePlayer.playOrPause(source) {
            // 播放自然结束时重置按钮（completionAction 在主线程回调）
            playBtn.text = "▶"
            progressBar.progress = 0
        }
        // 立即乐观更新按钮
        playBtn.text = if (VoiceMessagePlayer.isCurrentlyPlaying(source)) "⏸" else "▶"
    }

    // 长按：转文字 / 取消
    if (onLongPress != null) {
        container.setOnLongClickListener {
            onLongPress(attachment)
            true
        }
    }

    return container
}

private fun formatVoiceDuration(seconds: Int): String {
    val m = seconds / 60
    val s = seconds % 60
    return "%d:%02d".format(m, s)
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
