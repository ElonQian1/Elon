package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.drawable.Drawable
import android.view.Gravity
import android.view.View
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

internal class ChatSideMenuRefreshButton(
    context: Context,
    private val dp: (Int) -> Int,
    selectableForeground: () -> Drawable?,
    private val onRefresh: () -> Unit
) : FrameLayout(context) {
    private val ringSizePx = dp(RING_SIZE_DP)
    private val dotSizePx = dp(DOT_SIZE_DP)
    private val pathRadiusPx = ringSizePx * DOT_PATH_RADIUS_RATIO
    private var dotAnimator: ValueAnimator? = null

    private val dotView = ImageView(context).apply {
        setImageResource(R.drawable.ic_chat_side_menu_refresh_dot)
        scaleType = ImageView.ScaleType.FIT_CENTER
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    init {
        clipChildren = false
        clipToPadding = false
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "刷新会话列表"
        minimumWidth = dp(BUTTON_SIZE_DP)
        minimumHeight = dp(BUTTON_SIZE_DP)

        addView(
            ImageView(context).apply {
                setImageResource(R.drawable.ic_chat_side_menu_refresh_ring)
                scaleType = ImageView.ScaleType.FIT_CENTER
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            },
            LayoutParams(ringSizePx, ringSizePx, Gravity.CENTER)
        )
        addView(dotView, LayoutParams(dotSizePx, dotSizePx, Gravity.CENTER))
        updateDotPosition(0f)

        setOnClickListener {
            startDotOrbit()
            onRefresh()
        }
        addOnAttachStateChangeListener(object : OnAttachStateChangeListener {
            override fun onViewAttachedToWindow(v: View) = Unit
            override fun onViewDetachedFromWindow(v: View) {
                dotAnimator?.cancel()
                dotAnimator = null
                updateDotPosition(0f)
            }
        })
    }

    private fun startDotOrbit() {
        dotAnimator?.cancel()
        dotAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = DOT_ORBIT_DURATION_MS
            interpolator = LinearInterpolator()
            addUpdateListener { animator ->
                updateDotPosition(animator.animatedValue as Float)
            }
            start()
        }
    }

    private fun updateDotPosition(progress: Float) {
        val angle = DOT_START_ANGLE_RAD + progress * TWO_PI
        val radius = pathRadiusPx.toDouble()
        dotView.translationX = (cos(angle.toDouble()) * radius).roundToInt().toFloat()
        dotView.translationY = (sin(angle.toDouble()) * radius).roundToInt().toFloat()
    }

    private companion object {
        const val BUTTON_SIZE_DP = 38
        const val RING_SIZE_DP = 28
        const val DOT_SIZE_DP = 9
        const val DOT_ORBIT_DURATION_MS = 900L
        const val DOT_PATH_RADIUS_RATIO = 0.27731958f
        const val DOT_START_ANGLE_RAD = 0.38050637f
        const val TWO_PI = 6.2831855f
    }
}
