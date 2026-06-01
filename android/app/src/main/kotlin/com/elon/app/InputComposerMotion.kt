package com.elon.app

import android.animation.ValueAnimator
import android.view.View
import android.view.ViewGroup
import android.view.animation.PathInterpolator
import android.widget.FrameLayout

internal class InputComposerMotion(
    private val expandedInputContainer: FrameLayout,
    private val collapsedInputContainer: FrameLayout,
    private val collapsedText: View,
    private val modelButton: View,
    private val planModeButton: View,
    private val rightControls: FrameLayout
) {
    private val interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
    private var expandAnimator: ValueAnimator? = null
    private var textHeightAnimator: ValueAnimator? = null
    private var expandedTextHeight = 0
    private val expandedModelWidth: Int
    private val expandedModelEndMargin: Int
    private val expandedPlanWidth: Int
    private val expandedPlanEndMargin: Int

    var isExpanded: Boolean = false
        private set

    init {
        val density = modelButton.resources.displayMetrics.density
        val params = modelButton.layoutParams as? ViewGroup.MarginLayoutParams
        val fallbackWidth = (density * 76f).toInt()
        val fallbackMargin = (density * 8f).toInt()
        expandedModelWidth = params?.width?.takeIf { it > 0 } ?: fallbackWidth
        expandedModelEndMargin = params?.marginEnd ?: fallbackMargin

        val planParams = planModeButton.layoutParams as? ViewGroup.MarginLayoutParams
        expandedPlanWidth = planParams?.width?.takeIf { it > 0 } ?: (density * 64f).toInt()
        expandedPlanEndMargin = planParams?.marginEnd ?: (density * 6f).toInt()
    }

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
        val textVerticalTravel = collapsedInputContainer.resources.displayMetrics.density * 6f
        val textHorizontalTravel = collapsedInputContainer.resources.displayMetrics.density * 10f
        val startTextTranslationY = collapsedText.translationY
        val endTextTranslationY = if (expanded) -textVerticalTravel else 0f
        val startTextTranslationX = collapsedText.translationX
        val endTextTranslationX = if (expanded) textHorizontalTravel else 0f
        val startTextAlpha = collapsedText.alpha
        val endTextAlpha = if (expanded) 0f else 1f
        val startModelAlpha = modelButton.alpha
        val endModelAlpha = if (expanded) 1f else 0f
        val startPlanAlpha = planModeButton.alpha
        val endPlanAlpha = if (expanded) 1f else 0f
        val startModelWidth = currentOptionalWidth(modelButton, expanded)
        val endModelWidth = if (expanded) expandedModelWidth else 0
        val startModelMargin = currentOptionalEndMargin(modelButton, expanded)
        val endModelMargin = if (expanded) expandedModelEndMargin else 0
        val startPlanWidth = currentOptionalWidth(planModeButton, expanded)
        val endPlanWidth = if (expanded) expandedPlanWidth else 0
        val startPlanMargin = currentOptionalEndMargin(planModeButton, expanded)
        val endPlanMargin = if (expanded) expandedPlanEndMargin else 0
        val startRightWidth = rightControls.width.takeIf { it > 0 } ?: rightControls.layoutParams.width
        val endRightWidth = rightControlsTargetWidth()

        collapsedInputContainer.visibility = View.VISIBLE
        if (expanded) {
            setOptionalButtonWidth(modelButton, startModelWidth, startModelMargin)
            setOptionalButtonWidth(planModeButton, startPlanWidth, startPlanMargin)
            modelButton.visibility = View.VISIBLE
            planModeButton.visibility = View.VISIBLE
        }

        if (!animate) {
            setExpandedHeight(endHeight)
            collapsedInputContainer.alpha = endPillAlpha
            collapsedText.translationY = endTextTranslationY
            collapsedText.translationX = endTextTranslationX
            collapsedText.alpha = endTextAlpha
            modelButton.alpha = endModelAlpha
            planModeButton.alpha = endPlanAlpha
            setOptionalButtonWidth(modelButton, endModelWidth, endModelMargin)
            setOptionalButtonWidth(planModeButton, endPlanWidth, endPlanMargin)
            setRightControlsWidth(endRightWidth)
            collapsedInputContainer.visibility = if (expanded) View.INVISIBLE else View.VISIBLE
            if (!expanded) {
                modelButton.visibility = View.GONE
                planModeButton.visibility = View.GONE
            }
            return
        }

        expandAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 240L
            interpolator = this@InputComposerMotion.interpolator
            addUpdateListener { animator ->
                val t = animator.animatedValue as Float
                setExpandedHeight(lerp(startHeight, endHeight, t))
                collapsedInputContainer.alpha = lerp(startPillAlpha, endPillAlpha, t)
                collapsedText.translationY = lerp(startTextTranslationY, endTextTranslationY, t)
                collapsedText.translationX = lerp(startTextTranslationX, endTextTranslationX, t)
                collapsedText.alpha = lerp(startTextAlpha, endTextAlpha, t)
                modelButton.alpha = lerp(startModelAlpha, endModelAlpha, t)
                planModeButton.alpha = lerp(startPlanAlpha, endPlanAlpha, t)
                setOptionalButtonWidth(
                    modelButton,
                    lerp(startModelWidth, endModelWidth, t),
                    lerp(startModelMargin, endModelMargin, t)
                )
                setOptionalButtonWidth(
                    planModeButton,
                    lerp(startPlanWidth, endPlanWidth, t),
                    lerp(startPlanMargin, endPlanMargin, t)
                )
                setRightControlsWidth(lerp(startRightWidth, endRightWidth, t))
            }
            addListener(
                onEnd = {
                    collapsedInputContainer.visibility = if (expanded) View.INVISIBLE else View.VISIBLE
                    if (!expanded) {
                        modelButton.visibility = View.GONE
                        planModeButton.visibility = View.GONE
                    }
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

    private fun rightControlsTargetWidth(): Int {
        val addWidth = rightControls.resources.displayMetrics.density * 42f
        return addWidth.toInt()
    }

    private fun currentOptionalWidth(view: View, expanding: Boolean): Int {
        if (view.visibility == View.GONE && expanding) return 0
        return view.width.takeIf { it > 0 } ?: view.layoutParams.width.coerceAtLeast(0)
    }

    private fun currentOptionalEndMargin(view: View, expanding: Boolean): Int {
        val params = view.layoutParams as? ViewGroup.MarginLayoutParams ?: return 0
        if (view.visibility == View.GONE && expanding) return 0
        return params.marginEnd
    }

    private fun setOptionalButtonWidth(view: View, width: Int, endMargin: Int) {
        val params = view.layoutParams as? ViewGroup.MarginLayoutParams ?: return
        val targetWidth = width.coerceAtLeast(0)
        val targetMargin = endMargin.coerceAtLeast(0)
        if (params.width == targetWidth && params.marginEnd == targetMargin) return
        params.width = targetWidth
        params.marginEnd = targetMargin
        view.layoutParams = params
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
