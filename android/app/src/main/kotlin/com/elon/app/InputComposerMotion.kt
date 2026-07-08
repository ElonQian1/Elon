package com.elon.app

import android.animation.ValueAnimator
import android.view.View
import android.view.ViewGroup
import android.view.animation.PathInterpolator
import android.widget.FrameLayout

internal class InputComposerMotion(
    private val expandedInputContainer: FrameLayout,
    private val inputPanelContainer: View,
    private val collapsedInputContainer: FrameLayout,
    private val collapsedText: View,
    private val rightControls: FrameLayout
) {
    private val interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
    private var expandAnimator: ValueAnimator? = null
    private var expandAnimatorTarget: Boolean? = null
    private var textHeightAnimator: ValueAnimator? = null
    private var expandedTextHeight = 0
    private var expandedPanelBackgroundApplied = false

    var isExpanded: Boolean = false
        private set

    fun updateExpandedTextHeight(height: Int, animate: Boolean) {
        val target = height.coerceAtLeast(0)
        expandedTextHeight = target
        if (!isExpanded) return
        if (expandAnimator?.isRunning == true && expandAnimatorTarget == true) return
        animateHeight(expandedInputContainer, target, animate)
    }

    fun expandForTextInput(animate: Boolean) {
        setExpanded(expanded = true, animate = animate, animateLayoutHeight = true)
    }

    fun setExpanded(expanded: Boolean, animate: Boolean, animateLayoutHeight: Boolean = true) {
        if (isExpanded == expanded) {
            if (expandAnimator?.isRunning == true && expandAnimatorTarget == expanded) return
            if (!expanded || expandedInputContainer.height == targetExpandedHeight(expanded)) return
            animateHeight(expandedInputContainer, targetExpandedHeight(expanded), animate && animateLayoutHeight)
            return
        }
        isExpanded = expanded
        expandAnimator?.cancel()
        expandAnimatorTarget = null

        val transition = createTransition(expanded)
        prepareTransition(transition)

        if (expanded && !animateLayoutHeight) {
            setExpandedHeight(transition.endHeight)
        }

        if (!animate) {
            applyTransition(transition, 1f, animateLayoutHeight = true)
            completeTransition(transition)
            expandAnimatorTarget = null
            return
        }

        var cancelled = false
        expandAnimatorTarget = expanded
        expandAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 240L
            interpolator = this@InputComposerMotion.interpolator
            addUpdateListener { animator ->
                val t = animator.animatedValue as Float
                applyTransition(transition, t, animateLayoutHeight)
            }
            addListener(object : android.animation.AnimatorListenerAdapter() {
                override fun onAnimationCancel(animation: android.animation.Animator) {
                    cancelled = true
                }

                override fun onAnimationEnd(animation: android.animation.Animator) {
                    if (!cancelled) {
                        setExpandedHeight(targetExpandedHeight(transition.expanded))
                        completeTransition(transition)
                    }
                    if (expandAnimator === animation) {
                        expandAnimatorTarget = null
                        expandAnimator = null
                    }
                }
            })
            start()
        }
    }

    private fun createTransition(expanded: Boolean): ComposerTransition {
        val textVerticalTravel = collapsedInputContainer.resources.displayMetrics.density * 6f
        val textHorizontalTravel = collapsedInputContainer.resources.displayMetrics.density * 10f
        return ComposerTransition(
            expanded = expanded,
            startHeight = expandedInputContainer.height,
            endHeight = targetExpandedHeight(expanded),
            startPillAlpha = collapsedInputContainer.alpha,
            endPillAlpha = if (expanded) 0f else 1f,
            startTextTranslationY = collapsedText.translationY,
            endTextTranslationY = if (expanded) -textVerticalTravel else 0f,
            startTextTranslationX = collapsedText.translationX,
            endTextTranslationX = if (expanded) textHorizontalTravel else 0f,
            startTextAlpha = collapsedText.alpha,
            endTextAlpha = if (expanded) 0f else 1f,
            startRightWidth = rightControls.width.takeIf { it > 0 } ?: rightControls.layoutParams.width,
            endRightWidth = rightControlsTargetWidth()
        )
    }

    private fun prepareTransition(transition: ComposerTransition) {
        applyPanelBackground(transition.expanded)
        collapsedInputContainer.visibility = View.VISIBLE
    }

    private fun applyTransition(
        transition: ComposerTransition,
        progress: Float,
        animateLayoutHeight: Boolean
    ) {
        val t = progress.coerceIn(0f, 1f)
        if (animateLayoutHeight) {
            val endHeight = if (transition.expanded) targetExpandedHeight(expanded = true) else transition.endHeight
            setExpandedHeight(lerp(transition.startHeight, endHeight, t))
        }
        collapsedInputContainer.alpha = lerp(transition.startPillAlpha, transition.endPillAlpha, t)
        collapsedText.translationY = lerp(transition.startTextTranslationY, transition.endTextTranslationY, t)
        collapsedText.translationX = lerp(transition.startTextTranslationX, transition.endTextTranslationX, t)
        collapsedText.alpha = lerp(transition.startTextAlpha, transition.endTextAlpha, t)
        setRightControlsWidth(lerp(transition.startRightWidth, transition.endRightWidth, t))
    }

    private fun completeTransition(transition: ComposerTransition) {
        applyPanelBackground(transition.expanded)
        collapsedInputContainer.visibility = if (transition.expanded) View.INVISIBLE else View.VISIBLE
    }

    private fun applyPanelBackground(expanded: Boolean) {
        if (expandedPanelBackgroundApplied == expanded) return
        inputPanelContainer.setBackgroundResource(
            if (expanded) R.drawable.bg_bottom_panel_expanded else R.drawable.bg_bottom_panel_new
        )
        expandedPanelBackgroundApplied = expanded
    }

    private fun targetExpandedHeight(expanded: Boolean): Int {
        return if (expanded) {
            expandedTextHeight.takeIf { it > 0 } ?: defaultExpandedTextHeight()
        } else {
            0
        }
    }

    private fun defaultExpandedTextHeight(): Int {
        return (expandedInputContainer.resources.displayMetrics.density * 42f).toInt()
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
        val addWidth = rightControls.resources.displayMetrics.density * 38f
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

    private data class ComposerTransition(
        val expanded: Boolean,
        val startHeight: Int,
        val endHeight: Int,
        val startPillAlpha: Float,
        val endPillAlpha: Float,
        val startTextTranslationY: Float,
        val endTextTranslationY: Float,
        val startTextTranslationX: Float,
        val endTextTranslationX: Float,
        val startTextAlpha: Float,
        val endTextAlpha: Float,
        val startRightWidth: Int,
        val endRightWidth: Int
    )
}
