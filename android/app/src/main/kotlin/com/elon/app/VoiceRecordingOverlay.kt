package com.elon.app

import android.animation.ValueAnimator
import android.app.Activity
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.text.SpannableStringBuilder
import android.text.style.ForegroundColorSpan
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ScrollView
import android.widget.TextView

/**
 * WeChat-style full-screen overlay for "hold to talk".
 *
 * It only renders UI and touch-choice feedback. Speech/ASR state is still owned by
 * [MainSpeechInputActions].
 */
internal class VoiceRecordingOverlay(
    private val activity: Activity,
    val mode: Mode = Mode.AGENT
) {
    enum class Mode { AGENT, FRIEND_CHAT }
    enum class Zone { AI_REPLY, TRANSCRIBE, CANCEL, SEND }
    /** 识别生命周期各阶段，用于在 partial 文字为空时给用户状态反馈 */
    enum class ListeningState { PREPARING, LISTENING, HEARD, PROCESSING, SILENCE, NOISE }

    private var root: FrameLayout? = null
    private var bubbleView: VoiceWaveBubbleView? = null
    private var partialScroll: ScrollView? = null
    private var partialView: TextView? = null
    private var trayView: VoiceActionTrayView? = null
    private var partialText: String = ""
    private var historyText: String = ""  // 已确认的历史句段（Final 结果）
    private var stateHint: String = ""
    private var zone: Zone = Zone.AI_REPLY
    private var initialTouchRawY: Float? = null

    val isShowing: Boolean get() = root != null
    val currentZone: Zone get() = zone

    fun show() {
        if (root != null) return
        val parent = activity.window.decorView as? ViewGroup ?: return

        val overlay = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.parseColor("#CC000000"))
            // Touch events continue to be handled by the underlying hold button.
            isClickable = false
            isFocusable = false
        }

        val bubble = VoiceWaveBubbleView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(BUBBLE_WIDTH_DP), dp(BUBBLE_HEIGHT_DP)).apply {
                gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
                bottomMargin = dp(240)
            }
        }

        val partialTv = TextView(activity).apply {
            gravity = Gravity.CENTER
            includeFontPadding = false
            textSize = 15f
            setLineSpacing(0f, 1.25f)
            setTextColor(Color.parseColor("#DDEDEDED"))
            setPadding(0, 0, 0, 0)
        }
        val partial = ScrollView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(PARTIAL_MAX_HEIGHT_DP)
            ).apply {
                gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
                leftMargin = dp(36)
                rightMargin = dp(36)
                bottomMargin = dp(206)
            }
            isVerticalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            addView(partialTv)
        }

        val tray = VoiceActionTrayView(activity, mode).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(ACTION_TRAY_HEIGHT_DP),
                Gravity.BOTTOM
            )
        }

        overlay.addView(bubble)
        overlay.addView(partial)
        overlay.addView(tray)
        parent.addView(overlay)

        root = overlay
        bubbleView = bubble
        partialScroll = partial
        partialView = partialTv
        trayView = tray
        partialText = ""
        historyText = ""
        stateHint = "准备中..."
        initialTouchRawY = null
        zone = if (mode == Mode.AGENT) Zone.AI_REPLY else Zone.SEND
        applyZone()
    }

    fun updatePartial(text: String) {
        partialText = text.trim()
        renderPartial()
    }

    fun updateTouch(rawX: Float, rawY: Float) {
        val overlay = root ?: return
        val startY = initialTouchRawY
        if (startY == null) {
            initialTouchRawY = rawY
            if (mode == Mode.FRIEND_CHAT) {
                setZone(Zone.SEND)
                return
            }
        }
        val location = IntArray(2)
        overlay.getLocationOnScreen(location)
        val x = rawX - location[0]
        val y = rawY - location[1]
        val width = overlay.width.takeIf { it > 0 } ?: return
        val height = overlay.height.takeIf { it > 0 } ?: return
        val chooseTop = height - dp(TOUCH_CHOICE_HEIGHT_DP)
        if (mode == Mode.FRIEND_CHAT) {
            val movedUpEnough = rawY < (initialTouchRawY ?: rawY) - dp(CHOICE_DRAG_UP_DP)
            if (!movedUpEnough) {
                setZone(Zone.SEND)
                return
            }
        }
        val newZone = if (y < chooseTop) {
            when {
                x < width * 0.34f -> Zone.CANCEL
                x > width * 0.66f -> Zone.TRANSCRIBE
                else -> Zone.AI_REPLY
            }
        } else {
            if (mode == Mode.FRIEND_CHAT) Zone.SEND else Zone.AI_REPLY
        }
        setZone(newZone)
    }

    /** Backward-compatible horizontal-delta feedback for older call sites/tests. */
    fun updateZone(dx: Float) {
        val threshold = dp(DRAG_CHOICE_DX_DP).toFloat()
        val newZone = when {
            dx < -threshold -> Zone.CANCEL
            dx > threshold -> Zone.TRANSCRIBE
            mode == Mode.FRIEND_CHAT -> Zone.SEND
            else -> Zone.AI_REPLY
        }
        setZone(newZone)
    }

    fun hide() {
        bubbleView?.stopCountdown()
        root?.let { (it.parent as? ViewGroup)?.removeView(it) }
        root = null
        bubbleView = null
        partialScroll = null
        partialView = null
        trayView = null
        partialText = ""
        historyText = ""
        stateHint = ""
        initialTouchRawY = null
        zone = if (mode == Mode.AGENT) Zone.AI_REPLY else Zone.SEND
    }

    /** 更新麦克风实时音量（0-1），驱动波形条高度动态变化 */
    fun setVolume(v: Float) {
        bubbleView?.setVolume(v)
    }

    /**
     * 将已确认的最终识别结果追加到历史区（浅色）。
     * 用于多段连续说话时的滚动实时转写：每段 Final 到达时调用，
     * partialText 同步清空等待下段 Partial。
     */
    fun appendHistory(text: String) {
        val t = text.trim()
        if (t.isBlank()) return
        historyText = if (historyText.isBlank()) t else "$historyText\n$t"
        partialText = ""
        renderPartial()
    }

    /** 启动气泡倒计时弧（maxMs 毫秒内识别时长上限）。 */
    fun startCountdown(maxMs: Long) {
        bubbleView?.startCountdown(maxMs)
    }

    /** 停止倒计时弧。 */
    fun stopCountdown() {
        bubbleView?.stopCountdown()
    }

    /** 在 partial 文字为空时展示识别阶段状态文字 */
    fun setListeningState(state: ListeningState) {
        stateHint = when (state) {
            ListeningState.PREPARING -> "准备中..."
            ListeningState.LISTENING -> "正在听..."
            ListeningState.HEARD -> "听到了，松手发送"
            ListeningState.PROCESSING -> "识别中..."
            ListeningState.SILENCE -> "没有检测到声音"
            ListeningState.NOISE -> "环境较嘈杂，请靠近手机说话"
        }
        if (state == ListeningState.HEARD) {
            bubbleView?.playHeardAnimation()
        }
        renderPartial()
    }

    private fun setZone(newZone: Zone) {
        if (newZone == zone) return
        zone = newZone
        applyZone()
    }

    private fun applyZone() {
        trayView?.zone = zone
        bubbleView?.isCanceling = zone == Zone.CANCEL
        renderPartial()
    }

    private fun renderPartial() {
        val zoneFallback = when (zone) {
            Zone.AI_REPLY -> if (mode == Mode.FRIEND_CHAT) "滑到这 AI回复" else "松开 AI回复"
            Zone.TRANSCRIBE -> "松开 转文字"
            Zone.CANCEL -> "松开 取消"
            Zone.SEND -> "松开 发送"
        }
        val isDefaultZone = (zone == Zone.AI_REPLY && mode == Mode.AGENT) ||
                            (zone == Zone.SEND && mode == Mode.FRIEND_CHAT)
        val mainColor = Color.parseColor(
            when (zone) {
                Zone.AI_REPLY -> "#DDEDEDED"
                Zone.TRANSCRIBE -> "#EAF7F0"
                Zone.CANCEL -> "#FFE3E3"
                Zone.SEND -> "#DDEDEDED"
            }
        )
        // 有历史转写时：历史（半透明）+ 当前 partial 或状态提示
        if (historyText.isNotBlank()) {
            val sb = SpannableStringBuilder()
            sb.append(historyText)
            val currentLine = when {
                partialText.isNotBlank() -> "\n$partialText"
                else -> ""
            }
            if (currentLine.isNotBlank()) {
                sb.append(currentLine)
                sb.setSpan(
                    ForegroundColorSpan(Color.parseColor("#99EDEDED")),
                    0, historyText.length, android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
                )
            }
            partialView?.text = sb
        } else {
            partialView?.text = when {
                partialText.isNotBlank() -> partialText
                stateHint.isNotBlank() && isDefaultZone -> stateHint
                else -> zoneFallback
            }
        }
        partialView?.setTextColor(mainColor)
        // 自动滚到底部，让用户看到最新内容
        partialScroll?.post { partialScroll?.fullScroll(View.FOCUS_DOWN) }
    }

    private fun dp(v: Int): Int =
        (v * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val ACTION_TRAY_HEIGHT_DP = 194
        const val TOUCH_CHOICE_HEIGHT_DP = 118
        const val BUBBLE_WIDTH_DP = 192
        const val BUBBLE_HEIGHT_DP = 88
        const val CHOICE_DRAG_UP_DP = 56
        const val DRAG_CHOICE_DX_DP = 80
        const val PARTIAL_MAX_HEIGHT_DP = 112  // ≈ 5 行转写历史可见区域
    }
}

private class VoiceWaveBubbleView(context: Context) : View(context) {
    private val bubblePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#58BE6A")
        style = Paint.Style.FILL
    }
    private val cancelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#E65A5A")
        style = Paint.Style.FILL
    }
    private val wavePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#2C6C52")
        style = Paint.Style.FILL
    }
    /** 倒计时弧线画笔（STROKE 模式，宽度在 onSizeChanged 确定） */
    private val arcPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        color = Color.parseColor("#60FFFFFF")
    }
    private val arcRect = RectF()
    private val bubbleRect = RectF()
    private val bubblePath = Path()

    var isCanceling: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            invalidate()
        }

    /** 当前平滑后的音量值（0-1） */
    private var currentVolume: Float = 0f

    // ── 倒计时弧 ─────────────────────────────────────────────────────────
    private var countdownStartMs: Long = 0L
    private var countdownMaxMs: Long = 0L
    private var countdownProgress: Float = 0f  // 0=刚开始, 1=到时
    private val countdownRunnable = object : Runnable {
        override fun run() {
            if (countdownMaxMs <= 0L) return
            val elapsed = System.currentTimeMillis() - countdownStartMs
            countdownProgress = (elapsed.toFloat() / countdownMaxMs).coerceIn(0f, 1f)
            // 超过 75% 变黄色警告
            arcPaint.color = if (countdownProgress < 0.75f)
                Color.parseColor("#60FFFFFF")
            else
                Color.parseColor("#FFCC44")
            invalidate()
            if (countdownProgress < 1f) postDelayed(this, 80L)
        }
    }

    fun startCountdown(maxMs: Long) {
        removeCallbacks(countdownRunnable)
        countdownMaxMs = maxMs
        countdownStartMs = System.currentTimeMillis()
        countdownProgress = 0f
        post(countdownRunnable)
    }

    fun stopCountdown() {
        removeCallbacks(countdownRunnable)
        countdownMaxMs = 0L
        countdownProgress = 0f
        invalidate()
    }

    /** 说话被识别到（HEARD）时的短脉冲缩放动画，给用户确认感 */
    fun playHeardAnimation() {
        val anim = ValueAnimator.ofFloat(1f, 1.10f, 1f)
        anim.duration = 220L
        anim.addUpdateListener { va ->
            val s = va.animatedValue as Float
            scaleX = s
            scaleY = s
        }
        anim.start()
    }

    /**
     * 接收 onRmsChanged 归一化后的音量（0-1）。
     * 上升快（× 0.7 权重新值），下降慢（× 0.3 权重新值），让视觉感更自然。
     */
    fun setVolume(v: Float) {
        val target = v.coerceIn(0f, 1f)
        currentVolume = if (target > currentVolume) {
            currentVolume * 0.3f + target * 0.7f
        } else {
            currentVolume * 0.7f + target * 0.3f
        }
        invalidate()
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        val density = resources.displayMetrics.density
        val notch = 11f * density
        val margin = 6f * density
        arcRect.set(margin, margin, w - margin, h - notch - margin)
        arcPaint.strokeWidth = 3f * density
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val density = resources.displayMetrics.density
        val notch = 11f * density
        val radius = 12f * density
        bubbleRect.set(0f, 0f, width.toFloat(), height - notch)
        bubblePath.reset()
        bubblePath.addRoundRect(bubbleRect, radius, radius, Path.Direction.CW)
        bubblePath.moveTo(width / 2f - notch, height - notch)
        bubblePath.lineTo(width / 2f, height.toFloat())
        bubblePath.lineTo(width / 2f + notch, height - notch)
        bubblePath.close()
        canvas.drawPath(bubblePath, if (isCanceling) cancelPaint else bubblePaint)

        val centerX = width / 2f
        val centerY = (height - notch) / 2f
        val barWidth = 2.2f * density
        val gap = 3.2f * density
        val baseHeights = floatArrayOf(7f, 12f, 18f, 26f, 19f, 30f, 22f, 14f, 20f, 12f, 7f)
        // 无声音时保留 25% 高度（有形但安静），满音量时 100%
        val scale = 0.25f + currentVolume * 0.75f
        val totalWidth = baseHeights.size * barWidth + (baseHeights.size - 1) * gap
        var x = centerX - totalWidth / 2f
        baseHeights.forEach { hDp ->
            val half = hDp * scale * density / 2f
            canvas.drawRoundRect(
                x,
                centerY - half,
                x + barWidth,
                centerY + half,
                barWidth,
                barWidth,
                wavePaint
            )
            x += barWidth + gap
        }

        // 倒计时弧：从顶部顺时针缩短，快到时间变黄
        if (countdownMaxMs > 0L && countdownProgress < 1f) {
            val sweepAngle = 360f * (1f - countdownProgress)
            canvas.drawArc(arcRect, -90f, sweepAngle, false, arcPaint)
        }
    }
}

private class VoiceActionTrayView(
    context: Context,
    private val mode: VoiceRecordingOverlay.Mode = VoiceRecordingOverlay.Mode.AGENT
) : View(context) {
    private val outerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#70575757")
        style = Paint.Style.FILL
    }
    private val innerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#8A707070")
        style = Paint.Style.FILL
    }
    private val highlightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#48FFFFFF")
        style = Paint.Style.FILL
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#D6D6D6")
        textAlign = Paint.Align.CENTER
        textSize = sp(15f)
    }
    private val path = Path()

    var zone: VoiceRecordingOverlay.Zone = VoiceRecordingOverlay.Zone.AI_REPLY
        set(value) {
            if (field == value) return
            field = value
            invalidate()
        }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val w = width.toFloat()
        val h = height.toFloat()
        val d = resources.displayMetrics.density

        path.reset()
        path.moveTo(0f, h)
        path.lineTo(0f, 72f * d)
        path.cubicTo(w * 0.22f, 16f * d, w * 0.78f, 16f * d, w, 72f * d)
        path.lineTo(w, h)
        path.close()
        canvas.drawPath(path, outerPaint)

        path.reset()
        path.moveTo(0f, h)
        path.lineTo(0f, 122f * d)
        path.cubicTo(w * 0.24f, 74f * d, w * 0.76f, 74f * d, w, 122f * d)
        path.lineTo(w, h)
        path.close()
        canvas.drawPath(path, innerPaint)

        drawHighlight(canvas, w, d)
        drawLabels(canvas, w, d)
    }

    private fun drawHighlight(canvas: Canvas, width: Float, density: Float) {
        val actionCenterY = ACTION_OPTION_CENTER_Y_DP * density
        val aiReplyCenterY = AI_REPLY_OPTION_CENTER_Y_DP * density
        val sendCenterY = SEND_OPTION_CENTER_Y_DP * density
        val cx = when (zone) {
            VoiceRecordingOverlay.Zone.CANCEL -> width * 0.19f
            VoiceRecordingOverlay.Zone.AI_REPLY -> width * 0.50f
            VoiceRecordingOverlay.Zone.TRANSCRIBE -> width * 0.81f
            VoiceRecordingOverlay.Zone.SEND -> width * 0.50f
        }
        val cy = when {
            zone == VoiceRecordingOverlay.Zone.SEND -> sendCenterY
            zone == VoiceRecordingOverlay.Zone.AI_REPLY -> aiReplyCenterY
            else -> actionCenterY
        }
        val radius = if (zone == VoiceRecordingOverlay.Zone.SEND) 48f * density else 38f * density
        canvas.drawCircle(cx, cy, radius, highlightPaint)
    }

    private fun drawLabels(canvas: Canvas, width: Float, density: Float) {
        val actionCenterY = ACTION_OPTION_CENTER_Y_DP * density
        val aiReplyCenterY = AI_REPLY_OPTION_CENTER_Y_DP * density
        val sendCenterY = SEND_OPTION_CENTER_Y_DP * density
        drawRotatedOption(
            canvas,
            "取消",
            width * 0.19f,
            actionCenterY,
            -16f,
            zone == VoiceRecordingOverlay.Zone.CANCEL
        )

        if (mode == VoiceRecordingOverlay.Mode.AGENT) {
            drawOption(canvas, "AI回复", width * 0.50f, aiReplyCenterY, zone == VoiceRecordingOverlay.Zone.AI_REPLY)
        } else {
            // FRIEND_CHAT: AI回复 在上方中央，"发 送" 在下方默认区域
            drawOption(canvas, "AI回复", width * 0.50f, aiReplyCenterY, zone == VoiceRecordingOverlay.Zone.AI_REPLY)
            drawOption(canvas, "发 送", width * 0.50f, sendCenterY, zone == VoiceRecordingOverlay.Zone.SEND)
        }

        drawRotatedOption(
            canvas,
            "转文字",
            width * 0.81f,
            actionCenterY,
            14f,
            zone == VoiceRecordingOverlay.Zone.TRANSCRIBE
        )
    }

    private fun drawRotatedOption(
        canvas: Canvas,
        label: String,
        x: Float,
        centerY: Float,
        degrees: Float,
        selected: Boolean
    ) {
        canvas.save()
        canvas.rotate(degrees, x, centerY)
        drawOption(canvas, label, x, centerY, selected)
        canvas.restore()
    }

    private fun drawOption(canvas: Canvas, label: String, x: Float, centerY: Float, selected: Boolean) {
        textPaint.color = Color.parseColor(if (selected) "#D6D6D6" else "#D6D6D6")
        textPaint.isFakeBoldText = selected
        val fontMetrics = textPaint.fontMetrics
        val baseline = centerY - (fontMetrics.ascent + fontMetrics.descent) / 2f
        canvas.drawText(label, x, baseline, textPaint)
        textPaint.isFakeBoldText = false
    }

    private fun sp(value: Float): Float =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, value, resources.displayMetrics)

    private companion object {
        const val ACTION_OPTION_CENTER_Y_DP = 78f
        const val AI_REPLY_OPTION_CENTER_Y_DP = 58f
        const val SEND_OPTION_CENTER_Y_DP = 154f
    }
}
