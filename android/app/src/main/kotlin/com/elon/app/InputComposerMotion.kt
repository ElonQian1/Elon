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
    private var keyboardSyncTransition: ComposerTransition? = null
    private var keyboardSyncProgress = 0f

    var isExpanded: Boolean = false
        private set

    val isKeyboardSynchronizedExpansionPending: Boolean
        get() = keyboardSyncTransition != null

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
        if (keyboardSyncTransition != null) {
            keyboardSyncTransition = createTransition(expanded = true)
            keyboardSyncTransition?.let { applyTransition(it, keyboardSyncProgress, animateLayoutHeight = true) }
            return
        }
        animateHeight(expandedInputContainer, target, animate)
    }

    fun prepareKeyboardSynchronizedExpansion() {
        if (isExpanded && keyboardSyncTransition == null) return
        expandAnimator?.cancel()
        textHeightAnimator?.cancel()
        isExpanded = true
        keyboardSyncProgress = 0f
        keyboardSyncTransition = createTransition(expanded = true).also { transition ->
            prepareTransition(transition)
            applyTransition(transition, 0f, animateLayoutHeight = true)
        }
    }

    fun expandForTextInput(animate: Boolean) {
        keyboardSyncTransition = null
        keyboardSyncProgress = 0f
        setExpanded(expanded = true, animate = animate, animateLayoutHeight = true)
    }

    fun applyKeyboardSynchronizedExpansionProgress(progress: Float): Boolean {
        val transition = keyboardSyncTransition ?: return false
        keyboardSyncProgress = progress.coerceIn(0f, 1f)
        applyTransition(transition, keyboardSyncProgress, animateLayoutHeight = true)
        if (keyboardSyncProgress >= 1f) {
            completeTransition(transition)
            keyboardSyncTransition = null
        }
        return true
    }

    fun finishKeyboardSynchronizedExpansion(animate: Boolean): Boolean {
        val transition = keyboardSyncTransition ?: return false
        expandAnimator?.cancel()
        if (!animate) {
            applyTransition(transition, 1f, animateLayoutHeight = true)
            completeTransition(transition)
            keyboardSyncTransition = null
            keyboardSyncProgress = 0f
            return true
        }
        val startProgress = keyboardSyncProgress.coerceIn(0f, 1f)
        expandAnimator = ValueAnimator.ofFloat(startProgress, 1f).apply {
            duration = ((1f - startProgress) * 220L).toLong().coerceAtLeast(80L)
            interpolator = this@InputComposerMotion.interpolator
            addUpdateListener { animator ->
                keyboardSyncProgress = animator.animatedValue as Float
                applyTransition(transition, keyboardSyncProgress, animateLayoutHeight = true)
            }
            addListener(
                onEnd = {
                    applyTransition(transition, 1f, animateLayoutHeight = true)
                    completeTransition(transition)
                    keyboardSyncTransition = null
                    keyboardSyncProgress = 0f
                }
            )
            start()
        }
        return true
    }

    fun setExpanded(expanded: Boolean, animate: Boolean, animateLayoutHeight: Boolean = true) {
        if (expanded && keyboardSyncTransition != null) return
        keyboardSyncTransition = null
        keyboardSyncProgress = 0f
        if (isExpanded == expanded) {
            if (!expanded || expandedInputContainer.height == targetExpandedHeight(expanded)) return
            animateHeight(expandedInputContainer, targetExpandedHeight(expanded), animate && animateLayoutHeight)
            return
        }
        isExpanded = expanded
        expandAnimator?.cancel()

        val transition = createTransition(expanded)
        prepareTransition(transition)

        if (expanded && !animateLayoutHeight) {
            setExpandedHeight(transition.endHeight)
        }

        if (!animate) {
            applyTransition(transition, 1f, animateLayoutHeight = true)
            completeTransition(transition)
            return
        }

        expandAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 240L
            interpolator = this@InputComposerMotion.interpolator
            addUpdateListener { animator ->
                val t = animator.animatedValue as Float
                applyTransition(transition, t, animateLayoutHeight)
            }
            addListener(
                onEnd = {
                    setExpandedHeight(transition.endHeight)
                    completeTransition(transition)
                }
            )
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
            startModelAlpha = modelButton.alpha,
            endModelAlpha = if (expanded) 1f else 0f,
            startPlanAlpha = planModeButton.alpha,
            endPlanAlpha = if (expanded) 1f else 0f,
            startModelWidth = currentOptionalWidth(modelButton, expanded),
            endModelWidth = if (expanded) expandedModelWidth else 0,
            startModelMargin = currentOptionalEndMargin(modelButton, expanded),
            endModelMargin = if (expanded) expandedModelEndMargin else 0,
            startPlanWidth = currentOptionalWidth(planModeButton, expanded),
            endPlanWidth = if (expanded) expandedPlanWidth else 0,
            startPlanMargin = currentOptionalEndMargin(planModeButton, expanded),
            endPlanMargin = if (expanded) expandedPlanEndMargin else 0,
            startRightWidth = rightControls.width.takeIf { it > 0 } ?: rightControls.layoutParams.width,
            endRightWidth = rightControlsTargetWidth()
        )
    }

    private fun prepareTransition(transition: ComposerTransition) {
        collapsedInputContainer.visibility = View.VISIBLE
        if (transition.expanded) {
            setOptionalButtonWidth(modelButton, transition.startModelWidth, transition.startModelMargin)
            setOptionalButtonWidth(planModeButton, transition.startPlanWidth, transition.startPlanMargin)
            modelButton.visibility = View.VISIBLE
            planModeButton.visibility = View.VISIBLE
        }
    }

    private fun applyTransition(
        transition: ComposerTransition,
        progress: Float,
        animateLayoutHeight: Boolean
    ) {
        val t = progress.coerceIn(0f, 1f)
        if (animateLayoutHeight) {
            setExpandedHeight(lerp(transition.startHeight, transition.endHeight, t))
        }
        collapsedInputContainer.alpha = lerp(transition.startPillAlpha, transition.endPillAlpha, t)
        collapsedText.translationY = lerp(transition.startTextTranslationY, transition.endTextTranslationY, t)
        collapsedText.translationX = lerp(transition.startTextTranslationX, transition.endTextTranslationX, t)
        collapsedText.alpha = lerp(transition.startTextAlpha, transition.endTextAlpha, t)
        modelButton.alpha = lerp(transition.startModelAlpha, transition.endModelAlpha, t)
        planModeButton.alpha = lerp(transition.startPlanAlpha, transition.endPlanAlpha, t)
        setOptionalButtonWidth(
            modelButton,
            lerp(transition.startModelWidth, transition.endModelWidth, t),
            lerp(transition.startModelMargin, transition.endModelMargin, t)
        )
        setOptionalButtonWidth(
            planModeButton,
            lerp(transition.startPlanWidth, transition.endPlanWidth, t),
            lerp(transition.startPlanMargin, transition.endPlanMargin, t)
        )
        setRightControlsWidth(lerp(transition.startRightWidth, transition.endRightWidth, t))
    }

    private fun completeTransition(transition: ComposerTransition) {
        collapsedInputContainer.visibility = if (transition.expanded) View.INVISIBLE else View.VISIBLE
        if (!transition.expanded) {
            modelButton.visibility = View.GONE
            planModeButton.visibility = View.GONE
        }
    }

    private fun targetExpandedHeight(expanded: Boolean): Int {
        return if (expanded) {
            expandedTextHeight.takeIf { it > 0 } ?: defaultExpandedTextHeight()
        } else {
            0
        }
    }

    private fun defaultExpandedTextHeight(): Int {
        return (expandedInputContainer.resources.displayMetrics.density * 46f).toInt()
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
        val startModelAlpha: Float,
        val endModelAlpha: Float,
        val startPlanAlpha: Float,
        val endPlanAlpha: Float,
        val startModelWidth: Int,
        val endModelWidth: Int,
        val startModelMargin: Int,
        val endModelMargin: Int,
        val startPlanWidth: Int,
        val endPlanWidth: Int,
        val startPlanMargin: Int,
        val endPlanMargin: Int,
        val startRightWidth: Int,
        val endRightWidth: Int
    )
}

private fun ValueAnimator.addListener(onEnd: () -> Unit) {
    addListener(object : android.animation.AnimatorListenerAdapter() {
        override fun onAnimationEnd(animation: android.animation.Animator) {
            onEnd()
        }
    })
}
