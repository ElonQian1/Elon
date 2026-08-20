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
    private var pulse: AnimatorSet? = null

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
        status.text = when (stage) {
            WebChatRealtimeVoiceStage.PREPARING -> "正在连接 ChatGPT"
            WebChatRealtimeVoiceStage.STARTING -> "正在启动实时语音"
            WebChatRealtimeVoiceStage.ACTIVE -> "实时语音已打开"
            WebChatRealtimeVoiceStage.FAILED -> "实时语音未能启动"
        }
        this.detail.text = detail
        val failed = stage == WebChatRealtimeVoiceStage.FAILED
        retry.visibility = if (failed) View.VISIBLE else View.GONE
        officialFallback.visibility = if (failed) View.VISIBLE else View.GONE
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
        val column = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(dp(28), dp(28), dp(28), dp(34))
        }
        column.addView(
            textView(16f, Color.parseColor("#E8ECF4"), bold = true).apply {
                text = "ChatGPT 网页 AI"
                gravity = Gravity.CENTER
            },
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
