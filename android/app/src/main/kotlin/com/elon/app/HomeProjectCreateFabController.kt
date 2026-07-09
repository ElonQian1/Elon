package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.view.View
import android.view.ViewTreeObserver
import android.view.animation.AccelerateDecelerateInterpolator
import android.widget.LinearLayout
import android.widget.ScrollView
import com.elon.app.databinding.ActivityMainBinding

internal class HomeProjectCreateFabController(
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val showCreateProjectDialog: () -> Unit
) {
    private var expanded = true
    private var animator: ValueAnimator? = null
    private var motionAttached = false

    fun setup() {
        binding.ensureConversationPageScrollable()
        binding.homeProjectCreateMenu.setOnClickListener { showCreateProjectDialog() }
        ensureMotionAttached()
        syncStyle()
        reset()
    }

    fun show() {
        ensureMotionAttached()
        binding.homeProjectCreateMenu.visibility = View.VISIBLE
        reset()
        binding.homeProjectCreateMenu.bringToFront()
    }

    fun hide() {
        animator?.cancel()
        animator = null
        binding.homeProjectCreateMenu.visibility = View.GONE
    }

    private fun ensureMotionAttached() {
        if (motionAttached) return
        val scrollView = binding.conversationPage.parent as? ScrollView ?: return
        val listener = ViewTreeObserver.OnScrollChangedListener {
            updateExpanded(scrollView.scrollY <= dp(HOME_PROJECT_FAB_EXPAND_AT_TOP_DP), animate = true)
        }
        scrollView.viewTreeObserver.addOnScrollChangedListener(listener)
        motionAttached = true
    }

    private fun reset() {
        updateExpanded(expanded = true, animate = false)
    }

    private fun updateExpanded(expanded: Boolean, animate: Boolean) {
        val menu = binding.homeProjectCreateMenu
        val label = binding.homeProjectCreateLabel
        val collapsedWidth = dp(HOME_PROJECT_FAB_COLLAPSED_SIZE_DP)
        val expandedWidth = dp(HOME_PROJECT_FAB_EXPANDED_WIDTH_DP)
        val expandedLabelWidth = measureLabelWidth()
        val targetWidth = if (expanded) expandedWidth else collapsedWidth
        val targetLabelWidth = if (expanded) expandedLabelWidth else 0
        val currentLayoutWidth = menu.layoutParams.width.takeIf { it > 0 } ?: targetWidth
        val currentLabelWidth = (label.layoutParams as LinearLayout.LayoutParams)
            .width
            .takeIf { it >= 0 } ?: expandedLabelWidth
        val sameTarget = this.expanded == expanded
        if (sameTarget && animator?.isRunning == true) return

        val alreadyAtTarget = sameTarget &&
            currentLayoutWidth == targetWidth &&
            currentLabelWidth == targetLabelWidth &&
            animator == null
        if (alreadyAtTarget) {
            menu.bringToFront()
            return
        }

        this.expanded = expanded
        animator?.cancel()
        animator = null
        syncStyle()

        val targetIconMargin = if (expanded) dp(HOME_PROJECT_FAB_ICON_MARGIN_END_DP) else 0
        label.visibility = View.VISIBLE
        label.alpha = 1f

        if (!animate || menu.visibility != View.VISIBLE || menu.width <= 0) {
            applyFrame(targetWidth, targetIconMargin, targetLabelWidth)
            menu.bringToFront()
            return
        }

        val startWidth = menu.width.takeIf { it > 0 } ?: currentLayoutWidth
        val startIconMargin = (binding.homeProjectCreateIcon.layoutParams as LinearLayout.LayoutParams).marginEnd
        val startLabelWidth = (label.layoutParams as LinearLayout.LayoutParams)
            .width
            .takeIf { it >= 0 } ?: expandedLabelWidth
        val valueAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = HOME_PROJECT_FAB_ANIMATION_MS
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener { valueAnimator ->
                val progress = valueAnimator.animatedValue as Float
                val width = (startWidth + (targetWidth - startWidth) * progress).toInt()
                val iconMargin = (startIconMargin + (targetIconMargin - startIconMargin) * progress).toInt()
                val labelWidth = (startLabelWidth + (targetLabelWidth - startLabelWidth) * progress).toInt()
                applyFrame(width, iconMargin, labelWidth)
            }
            addListener(object : AnimatorListenerAdapter() {
                private var cancelled = false

                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                }

                override fun onAnimationEnd(animation: Animator) {
                    if (cancelled) return
                    applyFrame(targetWidth, targetIconMargin, targetLabelWidth)
                    animator = null
                    menu.bringToFront()
                }
            })
        }
        animator = valueAnimator
        valueAnimator.start()
    }

    private fun applyFrame(width: Int, iconMarginEnd: Int, labelWidth: Int) {
        val menu = binding.homeProjectCreateMenu
        val height = dp(if (expanded) HOME_PROJECT_FAB_EXPANDED_HEIGHT_DP else HOME_PROJECT_FAB_COLLAPSED_SIZE_DP)
        val menuParams = menu.layoutParams
        if (menuParams.width != width || menuParams.height != height) {
            menuParams.width = width
            menuParams.height = height
            menu.layoutParams = menuParams
        }

        val iconParams = binding.homeProjectCreateIcon.layoutParams as LinearLayout.LayoutParams
        val iconSize = dp(HOME_PROJECT_FAB_ICON_SIZE_DP)
        if (iconParams.width != iconSize || iconParams.height != iconSize) {
            iconParams.width = iconSize
            iconParams.height = iconSize
        }
        if (iconParams.marginEnd != iconMarginEnd) {
            iconParams.marginEnd = iconMarginEnd
        }
        binding.homeProjectCreateIcon.layoutParams = iconParams

        val label = binding.homeProjectCreateLabel
        val labelParams = label.layoutParams as LinearLayout.LayoutParams
        if (labelParams.width != labelWidth) {
            labelParams.width = labelWidth
            label.layoutParams = labelParams
        }
        label.alpha = 1f
        label.translationX = 0f
    }

    private fun syncStyle() {
        val context = binding.root.context
        binding.homeProjectCreateMenu.setBackgroundResource(R.drawable.bg_project_space_ai_menu_item)
        val iconColor = context.getColor(R.color.elon_button_primary_text)
        binding.homeProjectCreateIcon.setColorFilter(iconColor)
        binding.homeProjectCreateLabel.setTextColor(iconColor)
    }

    private fun measureLabelWidth(): Int {
        val label = binding.homeProjectCreateLabel
        label.measure(
            View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED),
            View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED)
        )
        return label.measuredWidth
    }

    private companion object {
        const val HOME_PROJECT_FAB_ANIMATION_MS = 220L
        const val HOME_PROJECT_FAB_COLLAPSED_SIZE_DP = 60
        const val HOME_PROJECT_FAB_EXPANDED_HEIGHT_DP = 60
        const val HOME_PROJECT_FAB_EXPANDED_WIDTH_DP = 144
        const val HOME_PROJECT_FAB_ICON_SIZE_DP = 28
        const val HOME_PROJECT_FAB_EXPAND_AT_TOP_DP = 4
        const val HOME_PROJECT_FAB_ICON_MARGIN_END_DP = 10
    }
}
