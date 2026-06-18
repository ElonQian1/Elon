package com.elon.chatvoice

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.util.TypedValue
import android.view.View

class ChatVoiceActionTrayView(
    context: Context,
    var mode: ChatVoiceMode = ChatVoiceMode.AGENT,
) : View(context) {
    private val outerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor(ChatVoiceInteractionContract.tokens.trayOuter)
        style = Paint.Style.FILL
    }
    private val innerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor(ChatVoiceInteractionContract.tokens.trayInner)
        style = Paint.Style.FILL
    }
    private val highlightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor(ChatVoiceInteractionContract.tokens.trayHighlight)
        style = Paint.Style.FILL
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#D6D6D6")
        textAlign = Paint.Align.CENTER
        textSize = sp(15f)
    }
    private val path = Path()

    var zone: ChatVoiceZone = ChatVoiceInteractionContract.defaultZone(mode)
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
        drawOuter(canvas, w, h, d)
        drawInner(canvas, w, h, d)
        drawHighlight(canvas, w, d)
        drawLabels(canvas, w, d)
    }

    private fun drawOuter(canvas: Canvas, width: Float, height: Float, density: Float) {
        path.reset()
        path.moveTo(0f, height)
        path.lineTo(0f, 72f * density)
        path.cubicTo(width * 0.22f, 16f * density, width * 0.78f, 16f * density, width, 72f * density)
        path.lineTo(width, height)
        path.close()
        canvas.drawPath(path, outerPaint)
    }

    private fun drawInner(canvas: Canvas, width: Float, height: Float, density: Float) {
        path.reset()
        path.moveTo(0f, height)
        path.lineTo(0f, 122f * density)
        path.cubicTo(width * 0.24f, 74f * density, width * 0.76f, 74f * density, width, 122f * density)
        path.lineTo(width, height)
        path.close()
        canvas.drawPath(path, innerPaint)
    }

    private fun drawHighlight(canvas: Canvas, width: Float, density: Float) {
        val actionY = ACTION_OPTION_CENTER_Y_DP * density
        val aiY = AI_REPLY_OPTION_CENTER_Y_DP * density
        val sendY = SEND_OPTION_CENTER_Y_DP * density
        val cx = when (zone) {
            ChatVoiceZone.CANCEL -> width * 0.19f
            ChatVoiceZone.AI_REPLY -> width * 0.50f
            ChatVoiceZone.TRANSCRIBE -> width * 0.81f
            ChatVoiceZone.SEND -> width * 0.50f
        }
        val cy = when (zone) {
            ChatVoiceZone.SEND -> sendY
            ChatVoiceZone.AI_REPLY -> aiY
            else -> actionY
        }
        val radius = if (zone == ChatVoiceZone.SEND) 48f * density else 38f * density
        canvas.drawCircle(cx, cy, radius, highlightPaint)
    }

    private fun drawLabels(canvas: Canvas, width: Float, density: Float) {
        val actionY = ACTION_OPTION_CENTER_Y_DP * density
        val aiY = AI_REPLY_OPTION_CENTER_Y_DP * density
        val sendY = SEND_OPTION_CENTER_Y_DP * density
        drawRotatedOption(canvas, "取消", width * 0.19f, actionY, -16f, zone == ChatVoiceZone.CANCEL)
        drawOption(canvas, "AI回复", width * 0.50f, aiY, zone == ChatVoiceZone.AI_REPLY)
        if (mode == ChatVoiceMode.FRIEND_CHAT) {
            drawOption(canvas, "发 送", width * 0.50f, sendY, zone == ChatVoiceZone.SEND)
        }
        drawRotatedOption(canvas, "转文字", width * 0.81f, actionY, 14f, zone == ChatVoiceZone.TRANSCRIBE)
    }

    private fun drawRotatedOption(
        canvas: Canvas,
        label: String,
        x: Float,
        centerY: Float,
        degrees: Float,
        selected: Boolean,
    ) {
        canvas.save()
        canvas.rotate(degrees, x, centerY)
        drawOption(canvas, label, x, centerY, selected)
        canvas.restore()
    }

    private fun drawOption(canvas: Canvas, label: String, x: Float, centerY: Float, selected: Boolean) {
        textPaint.isFakeBoldText = selected
        val metrics = textPaint.fontMetrics
        val baseline = centerY - (metrics.ascent + metrics.descent) / 2f
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
