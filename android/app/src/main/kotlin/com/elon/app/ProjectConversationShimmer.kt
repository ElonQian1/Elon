package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.view.animation.LinearInterpolator
import kotlin.math.sin

internal fun startProjectConversationShimmer(
    row: View,
    background: GradientDrawable,
    baseColorHex: String = "#222222",
    highlightColorHex: String = "#2A2A2A"
) {
    val baseColor = Color.parseColor(baseColorHex)
    val highlightColor = Color.parseColor(highlightColorHex)
    background.setColor(baseColor)

    val animator = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = PROJECT_CONVERSATION_SHIMMER_DURATION_MS
        repeatCount = ValueAnimator.INFINITE
        repeatMode = ValueAnimator.RESTART
        interpolator = LinearInterpolator()
        addUpdateListener { valueAnimator ->
            val pulse = sin(Math.PI * valueAnimator.animatedFraction).toFloat()
            background.setColor(blendColor(baseColor, highlightColor, pulse))
        }
    }

    row.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
        override fun onViewAttachedToWindow(v: View) = Unit

        override fun onViewDetachedFromWindow(v: View) {
            animator.cancel()
        }
    })
    animator.start()
}

private const val PROJECT_CONVERSATION_SHIMMER_DURATION_MS = 1350L
