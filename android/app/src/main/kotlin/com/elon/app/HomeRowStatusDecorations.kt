package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.animation.AccelerateDecelerateInterpolator
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal enum class HomeRowBadge {
    AI,
    PROJECT,
}

internal class HomeRowStatusDecorations(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
) {
    fun createTitle(title: String, badge: HomeRowBadge?): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT,
        )
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL

        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            )
            maxWidth = maxOf(
                dp(80),
                activity.resources.displayMetrics.widthPixels - dp(204),
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = title
            setTextColor(Color.parseColor("#F8F7F4"))
            textSize = 16f
            typeface = Typeface.create("sans-serif", Typeface.NORMAL)
        })

        badge?.let { addView(createBadge(it)) }
    }

    fun createWorkingIndicator(): View = FrameLayout(activity).apply {
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        clipChildren = false
        clipToPadding = false

        val dot = View(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(8), dp(8), Gravity.CENTER)
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor("#F8F7F4"))
            }
            scaleX = WORKING_DOT_MIN_SCALE
            scaleY = WORKING_DOT_MIN_SCALE
        }
        addView(dot)

        val animator = ValueAnimator.ofFloat(WORKING_DOT_MIN_SCALE, 1f).apply {
            duration = WORKING_DOT_PULSE_MS
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.REVERSE
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener { valueAnimator ->
                val scale = valueAnimator.animatedValue as Float
                dot.scaleX = scale
                dot.scaleY = scale
            }
        }
        addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
            override fun onViewAttachedToWindow(v: View) {
                if (!animator.isStarted) animator.start()
            }

            override fun onViewDetachedFromWindow(v: View) {
                animator.cancel()
            }
        })
    }

    private fun createBadge(badge: HomeRowBadge): TextView {
        val isAi = badge == HomeRowBadge.AI
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                dp(if (isAi) 34 else 42),
                dp(20),
            ).apply {
                marginStart = dp(7)
            }
            background = GradientDrawable().apply {
                cornerRadius = dp(5).toFloat()
                if (isAi) {
                    setColor(Color.TRANSPARENT)
                    setStroke(dp(1), Color.parseColor("#8EAAC4"))
                } else {
                    setColor(Color.parseColor("#9CBAD5"))
                }
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = if (isAi) "AI" else "项目"
            setTextColor(Color.parseColor(if (isAi) "#B8CDE0" else "#111820"))
            textSize = 11.5f
            typeface = Typeface.create("sans-serif", Typeface.NORMAL)
        }
    }

    private companion object {
        const val WORKING_DOT_MIN_SCALE = 0.62f
        const val WORKING_DOT_PULSE_MS = 900L
    }
}
