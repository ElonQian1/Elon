package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.util.LruCache
import android.view.Gravity
import android.view.View
import android.view.animation.LinearInterpolator
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import java.io.File
import kotlin.math.max
import kotlin.math.min

internal fun bindChatAttachmentViews(
    container: LinearLayout?,
    attachments: List<ChatAttachment>?,
    isSent: Boolean = false,
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
            attachment.isVoice() -> createVoiceAttachmentView(container.context, attachment, isSent, onVoiceLongPress)
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
            setColor(Color.parseColor("#222222"))
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
        ChatImagePreviewLoader.load(context, source) { bitmap ->
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
 * 微信风格语音消息气泡：发送方绿色（右），接收方深色（左）。
 * 发送方布局：[时长] [波形图标]；接收方：[波形图标] [时长]。
 * 宽度按录音时长动态伸缩（最窄 68dp，最宽 190dp）。
 */
private fun createVoiceAttachmentView(
    context: Context,
    attachment: ChatAttachment,
    isSent: Boolean,
    onLongPress: ((ChatAttachment) -> Unit)?
): View {
    val source = attachment.playbackSource() ?: ""
    val durationSec = (attachment.durationSeconds ?: 1).coerceAtLeast(1)
    // 微信风格时长：N"
    val durationText = "$durationSec\""

    // 宽度按时长动态计算：短语音约 92dp，最长约 216dp。
    val widthDp = (VOICE_BUBBLE_MIN_WIDTH_DP +
        (durationSec.coerceIn(1, 60) - 1) * VOICE_BUBBLE_WIDTH_PER_SECOND_DP)
        .coerceAtMost(VOICE_BUBBLE_MAX_WIDTH_DP)

    // 颜色匹配气泡：发送方绿色，接收方深色半透明
    val bgColor = if (isSent) Color.parseColor("#C8C8C8") else Color.parseColor("#2A2A2A")
    val textColor = if (isSent) Color.parseColor("#101010") else Color.parseColor("#D6D6D6")
    val waveColor = if (isSent) Color.parseColor("#101010") else Color.parseColor("#D6D6D6")

    val container = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        layoutParams = LinearLayout.LayoutParams(context.dp(widthDp), context.dp(VOICE_BUBBLE_HEIGHT_DP))
        minimumWidth = context.dp(VOICE_BUBBLE_MIN_WIDTH_DP)
        setPadding(context.dp(14), 0, context.dp(14), 0)
        background = voiceBubbleBackground(context, isSent, bgColor)
        isClickable = true
        isFocusable = true
    }

    // 波形图标（仿微信 3 道弧线，播放时动画）
    val waveIcon = VoiceWaveIconView(context, waveColor, faceRight = !isSent).apply {
        layoutParams = LinearLayout.LayoutParams(context.dp(30), context.dp(24))
        isPlaying = VoiceMessagePlayer.isCurrentlyPlaying(source)
    }

    // 时长文字
    val durationView = TextView(context).apply {
        text = durationText
        textSize = 14f
        setTextColor(textColor)
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        )
    }

    val spacer = View(context).apply {
        layoutParams = LinearLayout.LayoutParams(0, 1, 1f)
    }

    // 发送方：[时长] [波形]；接收方：[波形] [时长]
    if (isSent) {
        container.addView(durationView)
        container.addView(spacer)
        container.addView(waveIcon)
    } else {
        container.addView(waveIcon)
        container.addView(spacer)
        container.addView(durationView)
    }

    // 注册播放状态监听，随时更新此气泡的 UI
    val stateListener: (String, Boolean, Int, Int) -> Unit = listener@{ src, isPlaying, _, _ ->
        if (src != source) return@listener
        container.post {
            waveIcon.isPlaying = isPlaying
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
            waveIcon.isPlaying = false
        }
        waveIcon.isPlaying = VoiceMessagePlayer.isCurrentlyPlaying(source)
    }

    // 长按：转文字
    if (onLongPress != null) {
        container.setOnLongClickListener {
            onLongPress(attachment)
            true
        }
    }

    return container
}

private fun voiceBubbleBackground(context: Context, isSent: Boolean, color: Int): GradientDrawable {
    val radius = context.dp(8).toFloat()
    val tightRadius = context.dp(2).toFloat()
    return GradientDrawable().apply {
        setColor(color)
        cornerRadii = if (isSent) {
            floatArrayOf(
                radius, radius,
                tightRadius, tightRadius,
                radius, radius,
                radius, radius
            )
        } else {
            floatArrayOf(
                tightRadius, tightRadius,
                radius, radius,
                radius, radius,
                radius, radius
            )
        }
    }
}

/**
 * 仿微信语音波形图标：3 道半弧，播放时依次循环点亮动画。
 * [faceRight]=true 弧口朝右（接收方），false 朝左（发送方）。
 */
private class VoiceWaveIconView(
    context: Context,
    private val iconColor: Int,
    private val faceRight: Boolean
) : View(context) {

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        color = iconColor
    }

    private var animPhase = 0f
    private val animator = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = 900
        repeatCount = ValueAnimator.INFINITE
        repeatMode = ValueAnimator.RESTART
        interpolator = LinearInterpolator()
        addUpdateListener { animPhase = it.animatedFraction; invalidate() }
    }

    var isPlaying: Boolean = false
        set(v) {
            if (field == v) return
            field = v
            if (v) { if (!animator.isRunning) animator.start() }
            else { animator.cancel(); animPhase = 0f; invalidate() }
        }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        animator.cancel()
    }

    override fun onDraw(canvas: Canvas) {
        val w = width.toFloat()
        val h = height.toFloat()
        if (w <= 0f || h <= 0f) return

        val density = resources.displayMetrics.density
        val stroke = (min(w, h) * 0.12f).coerceIn(1.8f * density, 2.6f * density)
        val inset = stroke / 2f + density
        val centerY = h / 2f
        val maxRadius = (h / 2f - inset).coerceAtLeast(1f)
        paint.strokeWidth = stroke

        for (i in 0..2) {
            val r = maxRadius * (0.42f + i * 0.29f)
            val alpha: Float = if (isPlaying) {
                // 三道弧依次循环点亮
                val t = ((animPhase - i * 0.25f + 1f) % 1f)
                val sin = Math.sin(t * Math.PI * 2).toFloat()
                (0.3f + 0.7f * ((sin + 1f) / 2f)).coerceIn(0.2f, 1f)
            } else {
                // 静止：从内到外渐亮
                0.35f + i * 0.32f
            }
            paint.alpha = (alpha * 255).toInt()

            val cx = if (faceRight) inset + r else w - inset - r
            val oval = RectF(cx - r, centerY - r, cx + r, centerY + r)
            val startAngle = if (faceRight) -50f else 130f
            canvas.drawArc(oval, startAngle, 100f, false, paint)
        }
    }
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
        setTextColor(Color.parseColor("#222222"))
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

    fun load(context: Context, source: String, onReady: (Bitmap) -> Unit) {
        cache.get(source)?.let {
            onReady(it)
            return
        }
        val appContext = context.applicationContext
        Thread {
            val bitmap = runCatching { loadBitmap(appContext, source) }
                .onFailure { ChatImageDiskCache.remove(appContext, source) }
                .getOrNull() ?: return@Thread
            cache.put(source, bitmap)
            onReady(bitmap)
        }.start()
    }

    fun cached(source: String): Bitmap? = cache.get(source)

    private fun loadBitmap(context: Context, source: String): Bitmap {
        val bytes = ChatImageDiskCache.readBytes(context, source, MAX_IMAGE_BYTES)
        return decodeSampledBitmap(bytes)
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
            attachment.imageHeight?.toString().orEmpty(),
            attachment.durationSeconds?.toString().orEmpty(),
            attachment.annotations.joinToString(separator = "|") { annotation ->
                listOf(
                    annotation.x,
                    annotation.y,
                    annotation.width,
                    annotation.height,
                    annotation.iconX,
                    annotation.iconY,
                    annotation.iconWidth,
                    annotation.iconHeight,
                    annotation.note
                ).joinToString(separator = ",")
            }
        ).joinToString(separator = "\u001E")
    }
}

private const val MAX_CHAT_ATTACHMENTS = 6
private const val VOICE_BUBBLE_HEIGHT_DP = 42
private const val VOICE_BUBBLE_MIN_WIDTH_DP = 104
private const val VOICE_BUBBLE_MAX_WIDTH_DP = 216
private const val VOICE_BUBBLE_WIDTH_PER_SECOND_DP = 2
