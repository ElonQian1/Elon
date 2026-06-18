package com.elon.chatvoice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.view.View

class ChatVoiceWaveBubbleView(context: Context) : View(context) {
    private val bubblePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor(ChatVoiceInteractionContract.tokens.bubbleNormal)
        style = Paint.Style.FILL
    }
    private val cancelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor(ChatVoiceInteractionContract.tokens.bubbleCancel)
        style = Paint.Style.FILL
    }
    private val wavePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor(ChatVoiceInteractionContract.tokens.waveBar)
        style = Paint.Style.FILL
    }
    private val arcPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        color = Color.parseColor(ChatVoiceInteractionContract.tokens.countdownNormal)
    }
    private val arcRect = RectF()
    private val bubbleRect = RectF()
    private val bubblePath = Path()
    private var volume = 0f
    private var countdownStartMs = 0L
    private var countdownMaxMs = 0L
    private var countdownProgress = 0f
    private val countdownRunnable = object : Runnable {
        override fun run() {
            if (countdownMaxMs <= 0L) return
            val elapsed = System.currentTimeMillis() - countdownStartMs
            countdownProgress = (elapsed.toFloat() / countdownMaxMs).coerceIn(0f, 1f)
            arcPaint.color = Color.parseColor(
                if (countdownProgress < ChatVoiceInteractionContract.holdOptions.countdownWarningRatio) {
                    ChatVoiceInteractionContract.tokens.countdownNormal
                } else {
                    ChatVoiceInteractionContract.tokens.countdownWarning
                }
            )
            invalidate()
            if (countdownProgress < 1f) postDelayed(this, 80L)
        }
    }

    var isCanceling: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            invalidate()
        }

    fun setVolume(value: Float) {
        val target = value.coerceIn(0f, 1f)
        volume = if (target > volume) volume * 0.3f + target * 0.7f else volume * 0.7f + target * 0.3f
        invalidate()
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

    fun playHeardAnimation() {
        ValueAnimator.ofFloat(1f, 1.10f, 1f).apply {
            duration = ChatVoiceInteractionContract.holdOptions.heardPulseMs
            addUpdateListener { animator ->
                val scale = animator.animatedValue as Float
                scaleX = scale
                scaleY = scale
            }
            start()
        }
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
        drawWave(canvas, density, notch)
        if (countdownMaxMs > 0L && countdownProgress < 1f) {
            canvas.drawArc(arcRect, -90f, 360f * (1f - countdownProgress), false, arcPaint)
        }
    }

    private fun drawWave(canvas: Canvas, density: Float, notch: Float) {
        val centerX = width / 2f
        val centerY = (height - notch) / 2f
        val barWidth = 2.2f * density
        val gap = 3.2f * density
        val heights = floatArrayOf(7f, 12f, 18f, 26f, 19f, 30f, 22f, 14f, 20f, 12f, 7f)
        val scale = 0.25f + volume * 0.75f
        val totalWidth = heights.size * barWidth + (heights.size - 1) * gap
        var x = centerX - totalWidth / 2f
        heights.forEach { heightDp ->
            val half = heightDp * scale * density / 2f
            canvas.drawRoundRect(x, centerY - half, x + barWidth, centerY + half, barWidth, barWidth, wavePaint)
            x += barWidth + gap
        }
    }
}
