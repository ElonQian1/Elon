package com.elon.app

import android.animation.ValueAnimator
import android.view.View
import android.view.ViewGroup
import android.view.animation.DecelerateInterpolator
import android.widget.FrameLayout

internal class InputComposerMotion(
    private val expandedInputContainer: FrameLayout,
    private val collapsedInputContainer: FrameLayout,
    private val modelButton: View,
    private val rightControls: FrameLayout
) {
    private val interpolator = DecelerateInterpolator(1.4f)
    private var expandAnimator: ValueAnimator? = null
    private var textHeightAnimator: ValueAnimator? = null
    private var expandedTextHeight = 0

    var isExpanded: Boolean = false
        private set

    fun updateExpandedTextHeight(height: Int, animate: Boolean) {
        val target = height.coerceAtLeast(0)
        expandedTextHeight = target
        if (!isExpanded) return
        animateHeight(expandedInputContainer, target, animate)
    }

    fun setExpanded(expanded: Boolean, animate: Boolean) {
        if (isExpanded == expanded && expandedInputContainer.height == targetExpandedHeight(expanded)) return
        isExpanded = expanded
        expandAnimator?.cancel()

        val startHeight = expandedInputContainer.height
        val endHeight = targetExpandedHeight(expanded)
        val startPillAlpha = collapsedInputContainer.alpha
        val endPillAlpha = if (expanded) 0f else 1f
        val startModelAlpha = modelButton.alpha
        val endModelAlpha = if (expanded) 1f else 0f
        val startRightWidth = rightControls.width.takeIf { it > 0 } ?: rightControls.layoutParams.width
        val endRightWidth = rightControlsTargetWidth(expanded)

        collapsedInputContainer.visibility = View.VISIBLE
        if (expanded) modelButton.visibility = View.VISIBLE

        if (!animate) {
            setExpandedHeight(endHeight)
            collapsedInputContainer.alpha = endPillAlpha
            modelButton.alpha = endModelAlpha
            setRightControlsWidth(endRightWidth)
            collapsedInputContainer.visibility = if (expanded) View.INVISIBLE else View.VISIBLE
            if (!expanded) modelButton.visibility = View.GONE
            return
        }

        expandAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 220L
            interpolator = this@InputComposerMotion.interpolator
            addUpdateListener { animator ->
                val t = animator.animatedValue as Float
                setExpandedHeight(lerp(startHeight, endHeight, t))
                collapsedInputContainer.alpha = lerp(startPillAlpha, endPillAlpha, t)
                modelButton.alpha = lerp(startModelAlpha, endModelAlpha, t)
                setRightControlsWidth(lerp(startRightWidth, endRightWidth, t))
            }
            addListener(
                onEnd = {
                    collapsedInputContainer.visibility = if (expanded) View.INVISIBLE else View.VISIBLE
                    if (!expanded) modelButton.visibility = View.GONE
                }
            )
            start()
        }
    }

    private fun targetExpandedHeight(expanded: Boolean): Int {
        return if (expanded) expandedTextHeight else 0
    }

    private fun animateHeight(view: View, target: Int, animate: Boolean) {
        textHeightAnimator?.cancel()
        if (!animate) {
            setExpandedHeight(target)
            return
        }
        val start = view.height
        if (start == target) return
        textHeightAnimator = ValueAnimator.ofInt(start, target).apply {
            duration = 140L
            interpolator = this@InputComposerMotion.interpolator
            addUpdateListener { setExpandedHeight(it.animatedValue as Int) }
            start()
        }
    }

    private fun setExpandedHeight(height: Int) {
        val params = expandedInputContainer.layoutParams
        if (params.height == height) return
        params.height = height
        expandedInputContainer.layoutParams = params
    }

    private fun rightControlsTargetWidth(expanded: Boolean): Int {
        val addWidth = rightControls.resources.displayMetrics.density * if (expanded) 94f else 46f
        return addWidth.toInt()
    }

    private fun setRightControlsWidth(width: Int) {
        val params = rightControls.layoutParams as? ViewGroup.MarginLayoutParams ?: return
        if (params.width == width) return
        params.width = width
        rightControls.layoutParams = params
    }

    private fun lerp(start: Float, end: Float, t: Float): Float {
        return start + (end - start) * t
    }

    private fun lerp(start: Int, end: Int, t: Float): Int {
        return (start + (end - start) * t).toInt()
    }
}

private fun ValueAnimator.addListener(onEnd: () -> Unit) {
    addListener(object : android.animation.AnimatorListenerAdapter() {
        override fun onAnimationEnd(animation: android.animation.Animator) {
            onEnd()
        }
    })
}
