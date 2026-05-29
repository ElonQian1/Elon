package com.elon.app

import android.app.Activity
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
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

    private var root: FrameLayout? = null
    private var bubbleView: VoiceWaveBubbleView? = null
    private var partialView: TextView? = null
    private var trayView: VoiceActionTrayView? = null
    private var partialText: String = ""
    private var zone: Zone = Zone.AI_REPLY

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

        val partial = TextView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
                leftMargin = dp(36)
                rightMargin = dp(36)
                bottomMargin = dp(206)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            maxLines = 2
            textSize = 14f
            setTextColor(Color.parseColor("#DDEDEDED"))
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
        partialView = partial
        trayView = tray
        partialText = ""
        zone = if (mode == Mode.AGENT) Zone.AI_REPLY else Zone.SEND
        applyZone()
    }

    fun updatePartial(text: String) {
        partialText = text.trim()
        renderPartial()
    }

    fun updateTouch(rawX: Float, rawY: Float) {
        val overlay = root ?: return
        val location = IntArray(2)
        overlay.getLocationOnScreen(location)
        val x = rawX - location[0]
        val y = rawY - location[1]
        val width = overlay.width.takeIf { it > 0 } ?: return
        val height = overlay.height.takeIf { it > 0 } ?: return
        val chooseTop = height - dp(TOUCH_CHOICE_HEIGHT_DP)
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
            else -> Zone.AI_REPLY
        }
        setZone(newZone)
    }

    fun hide() {
        root?.let { (it.parent as? ViewGroup)?.removeView(it) }
        root = null
        bubbleView = null
        partialView = null
        trayView = null
        partialText = ""
        zone = if (mode == Mode.AGENT) Zone.AI_REPLY else Zone.SEND
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
        val fallback = when (zone) {
            Zone.AI_REPLY -> if (mode == Mode.FRIEND_CHAT) "滑到这 @AI回复" else "松开 AI回复"
            Zone.TRANSCRIBE -> "松开 转文字"
            Zone.CANCEL -> "松开 取消"
            Zone.SEND -> "松开 发送"
        }
        partialView?.text = partialText.ifBlank { fallback }
        partialView?.setTextColor(
            Color.parseColor(
                when (zone) {
                    Zone.AI_REPLY -> "#DDEDEDED"
                    Zone.TRANSCRIBE -> "#EAF7F0"
                    Zone.CANCEL -> "#FFE3E3"
                    Zone.SEND -> "#DDEDEDED"
                }
            )
        )
    }

    private fun dp(v: Int): Int =
        (v * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val ACTION_TRAY_HEIGHT_DP = 194
        const val TOUCH_CHOICE_HEIGHT_DP = 118
        const val BUBBLE_WIDTH_DP = 192
        const val BUBBLE_HEIGHT_DP = 88
        const val DRAG_CHOICE_DX_DP = 80
    }
}

private class VoiceWaveBubbleView(context: Context) : View(context) {
    private val bubblePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#3FBE7A")
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
    private val bubbleRect = RectF()
    private val bubblePath = Path()

    var isCanceling: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            invalidate()
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
        val heights = floatArrayOf(7f, 12f, 18f, 26f, 19f, 30f, 22f, 14f, 20f, 12f, 7f)
        val totalWidth = heights.size * barWidth + (heights.size - 1) * gap
        var x = centerX - totalWidth / 2f
        heights.forEach { hDp ->
            val half = hDp * density / 2f
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
        color = Color.parseColor("#EDEDED")
        textAlign = Paint.Align.CENTER
        textSize = sp(15f)
    }
    private val subTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#C8C8C8")
        textAlign = Paint.Align.CENTER
        textSize = sp(12f)
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
        drawLabels(canvas, w, h, d)
    }

    private fun drawHighlight(canvas: Canvas, width: Float, density: Float) {
        val cx = when (zone) {
            VoiceRecordingOverlay.Zone.CANCEL -> width * 0.19f
            VoiceRecordingOverlay.Zone.AI_REPLY -> width * 0.50f
            VoiceRecordingOverlay.Zone.TRANSCRIBE -> width * 0.81f
            VoiceRecordingOverlay.Zone.SEND -> width * 0.50f
        }
        val cy = when {
            zone == VoiceRecordingOverlay.Zone.SEND -> 150f * density
            else -> 72f * density
        }
        val radius = if (zone == VoiceRecordingOverlay.Zone.SEND) 48f * density else 38f * density
        canvas.drawCircle(cx, cy, radius, highlightPaint)
    }

    private fun drawLabels(canvas: Canvas, width: Float, height: Float, density: Float) {
        textPaint.color = Color.parseColor("#EFEFEF")
        subTextPaint.color = Color.parseColor("#D0D0D0")
        canvas.save()
        canvas.rotate(-16f, width * 0.19f, 78f * density)
        drawOption(canvas, "取消", width * 0.19f, 82f * density, zone == VoiceRecordingOverlay.Zone.CANCEL)
        canvas.restore()

        if (mode == VoiceRecordingOverlay.Mode.AGENT) {
            drawOption(canvas, "AI回复", width * 0.50f, 82f * density, zone == VoiceRecordingOverlay.Zone.AI_REPLY)
        } else {
            // FRIEND_CHAT: @AI 在上方中央，"发 送" 在下方默认区域
            drawOption(canvas, "@AI", width * 0.50f, 78f * density, zone == VoiceRecordingOverlay.Zone.AI_REPLY)
            drawOption(canvas, "发 送", width * 0.50f, 154f * density, zone == VoiceRecordingOverlay.Zone.SEND)
        }

        canvas.save()
        canvas.rotate(14f, width * 0.81f, 78f * density)
        drawOption(canvas, "转文字", width * 0.81f, 82f * density, zone == VoiceRecordingOverlay.Zone.TRANSCRIBE)
        canvas.restore()

        val releaseLabel = when (zone) {
            VoiceRecordingOverlay.Zone.AI_REPLY -> if (mode == VoiceRecordingOverlay.Mode.FRIEND_CHAT) "松开 @AI 回复" else "松开 AI回复"
            VoiceRecordingOverlay.Zone.TRANSCRIBE -> "松开 转文字"
            VoiceRecordingOverlay.Zone.CANCEL -> "松开 取消"
            VoiceRecordingOverlay.Zone.SEND -> "松开 发送"
        }
        subTextPaint.color = Color.parseColor("#2B2B2B")
        canvas.drawText(releaseLabel, width * 0.50f, height - 50f * density, subTextPaint)
    }

    private fun drawOption(canvas: Canvas, label: String, x: Float, y: Float, selected: Boolean) {
        textPaint.color = Color.parseColor(if (selected) "#FFFFFF" else "#E4E4E4")
        textPaint.isFakeBoldText = selected
        canvas.drawText(label, x, y, textPaint)
        textPaint.isFakeBoldText = false
    }

    private fun sp(value: Float): Float =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, value, resources.displayMetrics)
}
