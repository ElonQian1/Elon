package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Matrix
import android.graphics.Shader
import android.view.View
import android.view.animation.LinearInterpolator
import com.elon.app.databinding.ActivityMainBinding
import kotlin.math.sin

internal class MainStageHintShimmer(
    private val binding: ActivityMainBinding,
    private val isActiveConversationWorking: () -> Boolean
) {
    private var animator: ValueAnimator? = null
    private var token = 0

    fun update() {
        if (isActiveConversationWorking() && binding.chatPage.visibility == View.VISIBLE) {
            start()
        } else {
            stop()
        }
    }

    fun stop() {
        token += 1
        animator?.cancel()
        animator = null
        binding.stageHintText.paint.shader = null
        binding.stageHintText.alpha = 1f
        binding.stageHintText.setTextColor(Color.parseColor("#B8B8B8"))
        binding.stageHintText.invalidate()
    }

    private fun start() {
        token += 1
        val currentToken = token
        animator?.cancel()
        animator = null

        val text = binding.stageHintText
        text.paint.shader = null
        text.alpha = 1f
        text.post {
            if (currentToken != token || !isActiveConversationWorking() || binding.chatPage.visibility != View.VISIBLE) {
                return@post
            }
            val width = text.width.coerceAtLeast(text.measuredWidth)
            if (width <= 0) return@post

            val shader = LinearGradient(
                0f,
                0f,
                width.toFloat(),
                0f,
                intArrayOf(
                    Color.parseColor("#9A9A9A"),
                    Color.parseColor("#CFCFCF"),
                    Color.parseColor("#F6F6F6"),
                    Color.parseColor("#D8D8D8"),
                    Color.parseColor("#9A9A9A")
                ),
                floatArrayOf(0f, 0.28f, 0.5f, 0.72f, 1f),
                Shader.TileMode.CLAMP
            )
            val matrix = Matrix()
            text.paint.shader = shader

            animator = ValueAnimator.ofFloat(0f, 1f).apply {
                duration = 1350L
                repeatCount = ValueAnimator.INFINITE
                repeatMode = ValueAnimator.RESTART
                interpolator = LinearInterpolator()
                addUpdateListener { valueAnimator ->
                    val fraction = valueAnimator.animatedFraction
                    matrix.setTranslate(width * (fraction * 2f - 1f), 0f)
                    shader.setLocalMatrix(matrix)
                    text.alpha = 0.76f + 0.24f * sin(Math.PI * fraction).toFloat()
                    text.invalidate()
                }
                start()
            }
        }
    }
}
