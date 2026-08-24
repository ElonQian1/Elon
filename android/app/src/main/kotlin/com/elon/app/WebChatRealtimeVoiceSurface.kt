package com.elon.app

import android.animation.AnimatorSet
import android.animation.ObjectAnimator
import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.animation.AccelerateDecelerateInterpolator
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.Space
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal enum class WebChatRealtimeVoiceStage {
    PREPARING,
    STARTING,
    ACTIVE,
    FAILED,
}

internal interface WebChatRealtimeVoiceSurface {
    fun show(
        onClose: () -> Unit,
        onRetry: () -> Unit,
        onOfficialFallback: () -> Unit,
    )

    fun render(stage: WebChatRealtimeVoiceStage, detail: String)
    fun hide()
    fun isVisible(): Boolean
}

internal data class WebChatRealtimeVoiceLayoutMetrics(
    val compact: Boolean,
    val horizontalPadding: Int,
    val verticalPadding: Int,
    val titleHeight: Int,
    val orbSize: Int,
    val statusTopMargin: Int,
    val detailTopMargin: Int,
    val closeSize: Int,
)

internal object WebChatRealtimeVoiceLayoutPolicy {
    fun resolve(widthPx: Int, heightPx: Int, density: Float): WebChatRealtimeVoiceLayoutMetrics {
        fun dp(value: Int): Int = (value * density).toInt()
        val compact = heightPx < dp(560)
        if (!compact) {
            return WebChatRealtimeVoiceLayoutMetrics(
                compact = false,
                horizontalPadding = dp(28),
                verticalPadding = dp(28),
                titleHeight = dp(48),
                orbSize = dp(220),
                statusTopMargin = dp(34),
                detailTopMargin = dp(10),
                closeSize = dp(64),
            )
        }
        val orbSize = minOf(
            dp(128),
            (heightPx / 3).coerceAtLeast(dp(84)),
            (widthPx * 0.62f).toInt(),
        )
        return WebChatRealtimeVoiceLayoutMetrics(
            compact = true,
            horizontalPadding = dp(16),
            verticalPadding = dp(8),
            titleHeight = dp(36),
            orbSize = orbSize,
            statusTopMargin = dp(8),
            detailTopMargin = dp(4),
            closeSize = dp(52),
        )
    }
}

internal class WebChatRealtimeVoiceOverlay(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
) : WebChatRealtimeVoiceSurface {
    private val root = FrameLayout(activity).apply {
        setBackgroundColor(Color.parseColor("#05070A"))
        visibility = View.GONE
        isClickable = true
        isFocusable = true
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_SURFACE
    }
    private val orb = View(activity).apply {
        background = GradientDrawable(
            GradientDrawable.Orientation.TL_BR,
            intArrayOf(
                Color.parseColor("#4C6FFF"),
                Color.parseColor("#9EB4FF"),
                Color.parseColor("#F2F6FF"),
            ),
        ).apply { shape = GradientDrawable.OVAL }
        contentDescription = "实时语音状态"
    }
    private val status = textView(22f, Color.WHITE, bold = true).apply {
        gravity = Gravity.CENTER
        text = "正在准备实时语音"
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_STATUS
    }
    private val detail = textView(14f, Color.parseColor("#AAB2C0")).apply {
        gravity = Gravity.CENTER
        maxLines = 3
    }
    private val retry = actionButton("重试").apply {
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_RETRY
        visibility = View.GONE
    }
    private val officialFallback = textView(15f, Color.parseColor("#AAB2C0"), bold = true).apply {
        text = "打开官网语音"
        gravity = Gravity.CENTER
        background = rounded(Color.parseColor("#171B22"), 24)
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_OFFICIAL_FALLBACK
        visibility = View.GONE
    }
    private val close = ImageButton(activity).apply {
        setImageResource(R.drawable.ic_voice_call_hangup)
        scaleType = android.widget.ImageView.ScaleType.CENTER
        setPadding(dp(17), dp(17), dp(17), dp(17))
        background = rounded(Color.parseColor("#D83A45"), 32)
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_CLOSE
    }
    private val title = textView(16f, Color.parseColor("#E8ECF4"), bold = true).apply {
        text = "语音 AI"
        gravity = Gravity.CENTER
        maxLines = 1
    }
    private val column = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
    }
    private var pulse: AnimatorSet? = null
    private var compactLayout = false
    private var currentStage = WebChatRealtimeVoiceStage.PREPARING

    init {
        buildContent()
    }

    override fun show(
        onClose: () -> Unit,
        onRetry: () -> Unit,
        onOfficialFallback: () -> Unit,
    ) {
        close.setOnClickListener { onClose() }
        retry.setOnClickListener { onRetry() }
        officialFallback.setOnClickListener { onOfficialFallback() }
        if (root.parent !== host) {
            (root.parent as? ViewGroup)?.removeView(root)
            host.addView(
                root,
                FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
        }
        root.visibility = View.VISIBLE
        root.bringToFront()
        root.requestFocus()
        startPulse()
    }

    override fun render(stage: WebChatRealtimeVoiceStage, detail: String) {
        currentStage = stage
        status.text = when (stage) {
            WebChatRealtimeVoiceStage.PREPARING -> "正在连接语音 AI"
            WebChatRealtimeVoiceStage.STARTING -> "正在启动实时语音"
            WebChatRealtimeVoiceStage.ACTIVE -> "实时语音已打开"
            WebChatRealtimeVoiceStage.FAILED -> "实时语音未能启动"
        }
        this.detail.text = detail
        updateStageVisibility()
        val failed = stage == WebChatRealtimeVoiceStage.FAILED
        if (failed) stopPulse() else startPulse()
    }

    override fun hide() {
        stopPulse()
        root.visibility = View.GONE
        close.setOnClickListener(null)
        retry.setOnClickListener(null)
        officialFallback.setOnClickListener(null)
    }

    override fun isVisible(): Boolean = root.visibility == View.VISIBLE

    private fun buildContent() {
        column.addView(
            title,
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(48)),
        )
        column.addView(Space(activity), LinearLayout.LayoutParams(1, 0, 0.9f))
        column.addView(
            orb,
            LinearLayout.LayoutParams(dp(220), dp(220)).apply {
                gravity = Gravity.CENTER_HORIZONTAL
            },
        )
        column.addView(status, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply { topMargin = dp(34) })
        column.addView(detail, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply { topMargin = dp(10) })
        column.addView(Space(activity), LinearLayout.LayoutParams(1, 0, 1f))
        column.addView(retry, LinearLayout.LayoutParams(dp(176), dp(50)).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            bottomMargin = dp(12)
        })
        column.addView(officialFallback, LinearLayout.LayoutParams(dp(176), dp(48)).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            bottomMargin = dp(18)
        })
        column.addView(close, LinearLayout.LayoutParams(dp(64), dp(64)).apply {
            gravity = Gravity.CENTER_HORIZONTAL
        })
        root.addView(column, FrameLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT,
        ))
        root.addOnLayoutChangeListener { _, left, top, right, bottom, _, _, _, _ ->
            applyLayout(right - left, bottom - top)
        }
    }

    private fun applyLayout(widthPx: Int, heightPx: Int) {
        if (widthPx <= 0 || heightPx <= 0) return
        val metrics = WebChatRealtimeVoiceLayoutPolicy.resolve(
            widthPx = widthPx,
            heightPx = heightPx,
            density = activity.resources.displayMetrics.density,
        )
        compactLayout = metrics.compact
        column.setPadding(
            metrics.horizontalPadding,
            metrics.verticalPadding,
            metrics.horizontalPadding,
            metrics.verticalPadding,
        )
        title.layoutParams = (title.layoutParams as LinearLayout.LayoutParams).apply {
            height = metrics.titleHeight
        }
        orb.layoutParams = (orb.layoutParams as LinearLayout.LayoutParams).apply {
            width = metrics.orbSize
            height = metrics.orbSize
        }
        status.layoutParams = (status.layoutParams as LinearLayout.LayoutParams).apply {
            topMargin = metrics.statusTopMargin
        }
        detail.layoutParams = (detail.layoutParams as LinearLayout.LayoutParams).apply {
            topMargin = metrics.detailTopMargin
        }
        close.layoutParams = (close.layoutParams as LinearLayout.LayoutParams).apply {
            width = metrics.closeSize
            height = metrics.closeSize
        }
        updateStageVisibility()
    }

    private fun updateStageVisibility() {
        val failed = currentStage == WebChatRealtimeVoiceStage.FAILED
        retry.visibility = if (failed) View.VISIBLE else View.GONE
        officialFallback.visibility = if (failed) View.VISIBLE else View.GONE
        orb.visibility = if (failed && compactLayout) View.GONE else View.VISIBLE
    }

    private fun startPulse() {
        if (pulse?.isRunning == true) return
        pulse = AnimatorSet().apply {
            playTogether(
                ObjectAnimator.ofFloat(orb, View.SCALE_X, 0.96f, 1.03f),
                ObjectAnimator.ofFloat(orb, View.SCALE_Y, 0.96f, 1.03f),
                ObjectAnimator.ofFloat(orb, View.ALPHA, 0.82f, 1f),
            )
            duration = 2_200L
            interpolator = AccelerateDecelerateInterpolator()
            childAnimations.forEach {
                (it as ObjectAnimator).repeatCount = ValueAnimator.INFINITE
                it.repeatMode = ObjectAnimator.REVERSE
            }
            start()
        }
    }

    private fun stopPulse() {
        pulse?.cancel()
        pulse = null
        orb.scaleX = 1f
        orb.scaleY = 1f
        orb.alpha = 1f
    }

    private fun actionButton(label: String): TextView =
        textView(16f, Color.parseColor("#07111F"), bold = true).apply {
            text = label
            gravity = Gravity.CENTER
            background = rounded(Color.parseColor("#EAF1FF"), 24)
        }

    private fun textView(size: Float, color: Int, bold: Boolean = false): TextView =
        TextView(activity).apply {
            textSize = size
            setTextColor(color)
            includeFontPadding = false
            if (bold) typeface = Typeface.DEFAULT_BOLD
        }

    private fun rounded(color: Int, radiusDp: Int): GradientDrawable = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(radiusDp).toFloat()
        setColor(color)
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()
}
