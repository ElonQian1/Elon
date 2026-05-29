package com.elon.app

import android.app.Activity
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import kotlin.math.sin

/**
 * 仿微信"按住说话"的全屏录音遮罩。
 *
 * 三个区域，由手指水平位移决定（DOWN 时设零点）：
 *   - SEND：手指未明显偏移，松开 → 直接把转写文字作为消息发送
 *   - TRANSLATE：向右滑 >= [DRAG_THRESHOLD_DP] dp，松开 → 回填输入框供用户查看/编辑
 *   - CANCEL：向左滑 >= [DRAG_THRESHOLD_DP] dp，松开 → 取消并丢弃
 *
 * 仅渲染 UI，不持有 ASR 状态；由 [MainSpeechInputActions] 通过 [updatePartial] / [updateZone] 推送。
 */
internal class VoiceRecordingOverlay(private val activity: Activity) {

    enum class Zone { SEND, TRANSLATE, CANCEL }

    private val main = Handler(Looper.getMainLooper())
    private var root: FrameLayout? = null
    private var partialView: TextView? = null
    private var durationView: TextView? = null
    private var waveformView: WaveformView? = null
    private var leftZoneView: TextView? = null
    private var rightZoneView: TextView? = null
    private var centerActionText: TextView? = null
    private var startedAt = 0L
    private var zone: Zone = Zone.SEND

    private val durationTick = object : Runnable {
        override fun run() {
            val ms = SystemClock.elapsedRealtime() - startedAt
            durationView?.text = formatDuration(ms)
            waveformView?.advance()
            main.postDelayed(this, 80L)
        }
    }

    val isShowing: Boolean get() = root != null

    fun show() {
        if (root != null) return
        val parent = activity.window.decorView as? ViewGroup ?: return

        val overlay = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
            setBackgroundColor(Color.parseColor("#99000000"))
            isClickable = false
            isFocusable = false
        }

        // ─── 录音卡片（居中，底部留出操作栏空间）───────────────────────────────
        val waveform = WaveformView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(64)
            ).apply { bottomMargin = dp(14) }
        }
        val partial = TextView(activity).apply {
            textSize = 15f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
            maxLines = 3
            text = "正在听…"
            layoutParams = LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { bottomMargin = dp(6) }
        }
        val duration = TextView(activity).apply {
            textSize = 12f
            setTextColor(Color.parseColor("#CCFFFFFF"))
            gravity = Gravity.CENTER
            text = "0.0\""
            layoutParams = LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
            )
        }
        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#3DC561"))
                cornerRadius = dp(20).toFloat()
            }
            setPadding(dp(24), dp(28), dp(24), dp(20))
            elevation = dp(4).toFloat()
            layoutParams = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.CENTER
                marginStart = dp(32)
                marginEnd = dp(32)
                bottomMargin = dp(130)  // 向上偏移，为底部操作栏让出空间
            }
        }
        card.addView(waveform)
        card.addView(partial)
        card.addView(duration)

        // ─── 底部操作栏（仿微信：取消 ← | 松开发送 | → 转文字）──────────────────
        val leftZone = makeActionZone(activity, "取消", marginEndDp = 8)
        val rightZone = makeActionZone(activity, "转文字", marginStartDp = 8)
        val centerText = TextView(activity).apply {
            textSize = 16f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
            text = "松开 发送"
            layoutParams = LinearLayout.LayoutParams(dp(108), ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                gravity = Gravity.CENTER_VERTICAL
            }
        }
        val actionBar = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(20), dp(18), dp(20), dp(48))
            layoutParams = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { gravity = Gravity.BOTTOM }
        }
        actionBar.addView(leftZone)
        actionBar.addView(centerText)
        actionBar.addView(rightZone)

        overlay.addView(card)
        overlay.addView(actionBar)
        parent.addView(overlay)

        root = overlay
        partialView = partial
        durationView = duration
        waveformView = waveform
        leftZoneView = leftZone
        rightZoneView = rightZone
        centerActionText = centerText
        startedAt = SystemClock.elapsedRealtime()
        zone = Zone.SEND
        applyZone()
        main.post(durationTick)
    }

    private fun makeActionZone(
        ctx: android.content.Context,
        label: String,
        marginStartDp: Int = 0,
        marginEndDp: Int = 0
    ): TextView = TextView(ctx).apply {
        text = label
        textSize = 15f
        setTextColor(Color.parseColor("#AAAAAA"))
        gravity = Gravity.CENTER
        background = GradientDrawable().apply {
            setColor(Color.parseColor("#2E2E2E"))
            cornerRadius = dp(28).toFloat()
        }
        layoutParams = LinearLayout.LayoutParams(0, dp(58)).apply {
            weight = 1f
            if (marginStartDp > 0) this.marginStart = dp(marginStartDp)
            if (marginEndDp > 0) this.marginEnd = dp(marginEndDp)
        }
    }

    /** 实时识别文字（部分结果） */
    fun updatePartial(text: String) {
        partialView?.text = if (text.isBlank()) "正在听…" else text
    }

    /**
     * 根据手指水平位移更新区域。
     * @param dx 向右为正，向左为负（像素）
     */
    fun updateZone(dx: Float) {
        val threshold = dp(DRAG_THRESHOLD_DP).toFloat()
        val newZone = when {
            dx < -threshold -> Zone.CANCEL
            dx > threshold -> Zone.TRANSLATE
            else -> Zone.SEND
        }
        if (newZone != zone) {
            zone = newZone
            applyZone()
        }
    }

    val currentZone: Zone get() = zone

    fun hide() {
        main.removeCallbacks(durationTick)
        root?.let { (it.parent as? ViewGroup)?.removeView(it) }
        root = null
        partialView = null
        durationView = null
        waveformView = null
        leftZoneView = null
        rightZoneView = null
        centerActionText = null
    }

    private fun applyZone() {
        when (zone) {
            Zone.SEND -> {
                applyZoneStyle(leftZoneView, active = false, isCancelStyle = true)
                applyZoneStyle(rightZoneView, active = false, isCancelStyle = false)
                centerActionText?.text = "松开 发送"
                centerActionText?.setTextColor(Color.WHITE)
            }
            Zone.CANCEL -> {
                applyZoneStyle(leftZoneView, active = true, isCancelStyle = true)
                applyZoneStyle(rightZoneView, active = false, isCancelStyle = false)
                centerActionText?.text = "松开 取消"
                centerActionText?.setTextColor(Color.parseColor("#FF6B6B"))
            }
            Zone.TRANSLATE -> {
                applyZoneStyle(leftZoneView, active = false, isCancelStyle = true)
                applyZoneStyle(rightZoneView, active = true, isCancelStyle = false)
                centerActionText?.text = "松开 转文字"
                centerActionText?.setTextColor(Color.parseColor("#4FC3F7"))
            }
        }
    }

    private fun applyZoneStyle(tv: TextView?, active: Boolean, isCancelStyle: Boolean) {
        tv ?: return
        val bg = tv.background as? GradientDrawable ?: return
        val bgColor = when {
            active && isCancelStyle -> Color.parseColor("#AA2222")
            active && !isCancelStyle -> Color.parseColor("#1A6DAA")
            else -> Color.parseColor("#2E2E2E")
        }
        bg.setColor(bgColor)
        tv.setTextColor(if (active) Color.WHITE else Color.parseColor("#AAAAAA"))
    }

    private fun formatDuration(ms: Long) = String.format("%.1f\"", ms / 1000.0)
    private fun dp(v: Int) = (v * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val DRAG_THRESHOLD_DP = 60
    }

    // ─── 波形动画 View ──────────────────────────────────────────────────────────
    private inner class WaveformView(ctx: android.content.Context) : View(ctx) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE }
        private val barCount = 28
        private var tick = 0f

        fun advance() {
            tick += 1f
            invalidate()
        }

        override fun onDraw(canvas: Canvas) {
            val w = width.toFloat()
            val h = height.toFloat()
            if (w <= 0f || h <= 0f) return
            // gap = barW * 0.6  →  barW = w / (barCount + (barCount-1)*0.6)
            val barW = w / (barCount + (barCount - 1) * 0.6f)
            val step = barW * 1.6f
            for (i in 0 until barCount) {
                val x = i * step
                // 双频叠加，模拟自然声波起伏
                val wave = (sin((tick * 0.18f + i * 0.65f).toDouble()) * 0.45 +
                            sin((tick * 0.11f + i * 1.25f).toDouble()) * 0.2 + 0.55)
                    .toFloat().coerceIn(0.08f, 1f)
                val barH = wave * h
                val top = (h - barH) / 2f
                canvas.drawRoundRect(RectF(x, top, x + barW, top + barH), barW / 2, barW / 2, paint)
            }
        }
    }
}
