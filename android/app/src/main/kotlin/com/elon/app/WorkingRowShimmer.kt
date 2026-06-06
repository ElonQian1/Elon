package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Color
import android.view.View
import android.view.animation.LinearInterpolator
import java.util.WeakHashMap
import kotlin.math.sin

internal class WorkingRowShimmer(
    private val baseColor: Int = Color.parseColor("#181B20"),
    private val highlightColor: Int = Color.parseColor("#283140")
) {
    private val animators = WeakHashMap<View, ValueAnimator>()
    private val detachListeners = WeakHashMap<View, View.OnAttachStateChangeListener>()

    fun start(row: View) {
        if (animators[row]?.isRunning == true) return
        stop(row, resetColor = false)
        row.setBackgroundColor(baseColor)

        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1350L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener { valueAnimator ->
                val pulse = sin(Math.PI * valueAnimator.animatedFraction).toFloat()
                row.setBackgroundColor(blendColor(baseColor, highlightColor, pulse))
            }
        }
        val detachListener = object : View.OnAttachStateChangeListener {
            override fun onViewAttachedToWindow(v: View) = Unit
            override fun onViewDetachedFromWindow(v: View) {
                stop(v, resetColor = false)
            }
        }
        animators[row] = animator
        detachListeners[row] = detachListener
        row.addOnAttachStateChangeListener(detachListener)
        animator.start()
    }

    fun stop(row: View, color: Int = baseColor) {
        stop(row, resetColor = true, color = color)
    }

    fun cancelAll() {
        animators.keys.toList().forEach { stop(it, resetColor = false) }
    }

    private fun stop(row: View, resetColor: Boolean, color: Int = baseColor) {
        animators.remove(row)?.cancel()
        detachListeners.remove(row)?.let(row::removeOnAttachStateChangeListener)
        if (resetColor) row.setBackgroundColor(color)
    }
}
