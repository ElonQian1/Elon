package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import kotlin.math.hypot
import kotlin.math.max

internal fun showChatProjectDropRipple(
    overlay: View,
    contentContainer: ViewGroup,
    share: ChatProjectShare,
    overlayX: Float,
    overlayY: Float
) {
    val overlayLocation = IntArray(2)
    val contentLocation = IntArray(2)
    overlay.getLocationOnScreen(overlayLocation)
    contentContainer.getLocationOnScreen(contentLocation)
    val localX = overlayLocation[0] + overlayX - contentLocation[0]
    val localY = overlayLocation[1] + overlayY - contentLocation[1]
    val ripple = ChatProjectDropRippleView(contentContainer.context, projectCardPaletteFor(share.id).first())
    contentContainer.addView(
        ripple,
        FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT
        )
    )
    ripple.start(localX, localY) {
        (ripple.parent as? ViewGroup)?.removeView(ripple)
    }
}

private class ChatProjectDropRippleView(
    context: Context,
    private val baseColor: Int
) : View(context) {
    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = context.resources.displayMetrics.density * 1.4f
    }
    private var centerX = 0f
    private var centerY = 0f
    private var fraction = 0f

    fun start(x: Float, y: Float, onEnd: () -> Unit) {
        centerX = x
        centerY = y
        ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 620L
            interpolator = LinearInterpolator()
            addUpdateListener {
                fraction = it.animatedFraction
                invalidate()
            }
            addListener(object : android.animation.AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: android.animation.Animator) {
                    onEnd()
                }
            })
            start()
        }
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val maxRadius = max(
            hypot(centerX.toDouble(), centerY.toDouble()),
            hypot((width - centerX).toDouble(), (height - centerY).toDouble())
        ).toFloat()
        val radius = maxRadius * fraction
        val alpha = ((1f - fraction) * 78).toInt().coerceIn(0, 78)
        fillPaint.color = withAlpha(baseColor, alpha)
        strokePaint.color = withAlpha(baseColor, (alpha * 1.4f).toInt().coerceIn(0, 110))
        canvas.drawCircle(centerX, centerY, radius, fillPaint)
        canvas.drawCircle(centerX, centerY, radius * 0.72f, strokePaint)
    }

    private fun withAlpha(color: Int, alpha: Int): Int {
        return Color.argb(alpha, Color.red(color), Color.green(color), Color.blue(color))
    }
}
