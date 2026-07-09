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
    private val openProjectPlaza: () -> Unit,
    private val openHome: () -> Unit
) {
    private enum class EntryMode {
        PROJECT_PLAZA,
        HOME
    }

    private var mode = EntryMode.PROJECT_PLAZA
    private var expanded = true
    private var animator: ValueAnimator? = null
    private var motionAttached = false

    fun setup() {
        binding.ensureConversationPageScrollable()
        binding.homeProjectCreateMenu.setOnClickListener { handleClick() }
        ensureMotionAttached()
        syncStyle()
        reset()
    }

    fun showProjectPlazaEntry() {
        show(EntryMode.PROJECT_PLAZA, expandedOnShow = true)
    }

    fun showHomeEntry() {
        show(EntryMode.HOME, expandedOnShow = false)
    }

    private fun show(targetMode: EntryMode, expandedOnShow: Boolean) {
        ensureMotionAttached()
        setMode(targetMode)
        binding.homeProjectCreateMenu.visibility = View.VISIBLE
        reset(expandedOnShow)
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

    private fun reset(expandedOnShow: Boolean = true) {
        updateExpanded(expanded = expandedOnShow, animate = false)
    }

    private fun setMode(targetMode: EntryMode) {
        if (mode == targetMode) return
        mode = targetMode
        syncStyle()
        applyFrame(
            width = binding.homeProjectCreateMenu.layoutParams.width.takeIf { it > 0 }
                ?: dp(HOME_PROJECT_FAB_EXPANDED_WIDTH_DP),
            iconGap = if (expanded) dp(HOME_PROJECT_FAB_ICON_GAP_DP) else 0,
            secondaryIconWidth = if (expanded) dp(HOME_PROJECT_FAB_ICON_SIZE_DP) else 0
        )
    }

    private fun handleClick() {
        when (mode) {
            EntryMode.PROJECT_PLAZA -> openProjectPlaza()
            EntryMode.HOME -> openHome()
        }
    }

    private fun updateExpanded(expanded: Boolean, animate: Boolean) {
        val menu = binding.homeProjectCreateMenu
        val collapsedWidth = dp(HOME_PROJECT_FAB_COLLAPSED_SIZE_DP)
        val expandedWidth = dp(HOME_PROJECT_FAB_EXPANDED_WIDTH_DP)
        val expandedSecondaryIconWidth = dp(HOME_PROJECT_FAB_ICON_SIZE_DP)
        val targetWidth = if (expanded) expandedWidth else collapsedWidth
        val targetSecondaryIconWidth = if (expanded) expandedSecondaryIconWidth else 0
        val currentLayoutWidth = menu.layoutParams.width.takeIf { it > 0 } ?: targetWidth
        val currentSecondaryIconWidth = currentSecondaryIconWidth()
        val sameTarget = this.expanded == expanded
        if (sameTarget && animator?.isRunning == true) return

        val alreadyAtTarget = sameTarget &&
            currentLayoutWidth == targetWidth &&
            currentSecondaryIconWidth == targetSecondaryIconWidth &&
            animator == null
        if (alreadyAtTarget) {
            menu.bringToFront()
            return
        }

        this.expanded = expanded
        animator?.cancel()
        animator = null
        syncStyle()

        val targetIconGap = if (expanded) dp(HOME_PROJECT_FAB_ICON_GAP_DP) else 0

        if (!animate || menu.visibility != View.VISIBLE || menu.width <= 0) {
            applyFrame(targetWidth, targetIconGap, targetSecondaryIconWidth)
            menu.bringToFront()
            return
        }

        val startWidth = menu.width.takeIf { it > 0 } ?: currentLayoutWidth
        val startIconGap = currentIconGap()
        val startSecondaryIconWidth = currentSecondaryIconWidth
        val valueAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = HOME_PROJECT_FAB_ANIMATION_MS
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener { valueAnimator ->
                val progress = valueAnimator.animatedValue as Float
                val width = (startWidth + (targetWidth - startWidth) * progress).toInt()
                val iconGap = (startIconGap + (targetIconGap - startIconGap) * progress).toInt()
                val secondaryIconWidth =
                    (startSecondaryIconWidth + (targetSecondaryIconWidth - startSecondaryIconWidth) * progress).toInt()
                applyFrame(width, iconGap, secondaryIconWidth)
            }
            addListener(object : AnimatorListenerAdapter() {
                private var cancelled = false

                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                }

                override fun onAnimationEnd(animation: Animator) {
                    if (cancelled) return
                    applyFrame(targetWidth, targetIconGap, targetSecondaryIconWidth)
                    animator = null
                    menu.bringToFront()
                }
            })
        }
        animator = valueAnimator
        valueAnimator.start()
    }

    private fun applyFrame(width: Int, iconGap: Int, secondaryIconWidth: Int) {
        val menu = binding.homeProjectCreateMenu
        val height = dp(if (expanded) HOME_PROJECT_FAB_EXPANDED_HEIGHT_DP else HOME_PROJECT_FAB_COLLAPSED_SIZE_DP)
        val menuParams = menu.layoutParams
        if (menuParams.width != width || menuParams.height != height) {
            menuParams.width = width
            menuParams.height = height
            menu.layoutParams = menuParams
        }

        val iconSize = dp(HOME_PROJECT_FAB_ICON_SIZE_DP)
        val homeParams = binding.homeProjectHomeIcon.layoutParams as LinearLayout.LayoutParams
        val projectParams = binding.homeProjectCreateIcon.layoutParams as LinearLayout.LayoutParams

        if (mode == EntryMode.PROJECT_PLAZA) {
            homeParams.width = secondaryIconWidth
            homeParams.height = iconSize
            homeParams.marginEnd = if (secondaryIconWidth > 0) iconGap else 0
            projectParams.width = iconSize
            projectParams.height = iconSize
            projectParams.marginEnd = 0
            binding.homeProjectHomeIcon.alpha = if (secondaryIconWidth > 0) 1f else 0f
            binding.homeProjectCreateIcon.alpha = 1f
        } else {
            homeParams.width = iconSize
            homeParams.height = iconSize
            homeParams.marginEnd = if (secondaryIconWidth > 0) iconGap else 0
            projectParams.width = secondaryIconWidth
            projectParams.height = iconSize
            projectParams.marginEnd = 0
            binding.homeProjectHomeIcon.alpha = 1f
            binding.homeProjectCreateIcon.alpha = if (secondaryIconWidth > 0) 1f else 0f
        }
        binding.homeProjectHomeIcon.layoutParams = homeParams
        binding.homeProjectCreateIcon.layoutParams = projectParams
    }

    private fun syncStyle() {
        val context = binding.root.context
        binding.homeProjectCreateMenu.setBackgroundResource(R.drawable.bg_home_floating_nav)
        binding.homeProjectCreateMenu.contentDescription = when (mode) {
            EntryMode.PROJECT_PLAZA -> "项目广场"
            EntryMode.HOME -> "首页"
        }
        val iconColor = context.getColor(R.color.elon_text_primary)
        binding.homeProjectHomeIcon.setColorFilter(iconColor)
        binding.homeProjectCreateIcon.setColorFilter(iconColor)
    }

    private fun currentSecondaryIconWidth(): Int {
        val icon = when (mode) {
            EntryMode.PROJECT_PLAZA -> binding.homeProjectHomeIcon
            EntryMode.HOME -> binding.homeProjectCreateIcon
        }
        return (icon.layoutParams as LinearLayout.LayoutParams).width
            .takeIf { it >= 0 }
            ?: dp(HOME_PROJECT_FAB_ICON_SIZE_DP)
    }

    private fun currentIconGap(): Int {
        val icon = when (mode) {
            EntryMode.PROJECT_PLAZA -> binding.homeProjectHomeIcon
            EntryMode.HOME -> binding.homeProjectHomeIcon
        }
        return (icon.layoutParams as LinearLayout.LayoutParams).marginEnd
    }

    private companion object {
        const val HOME_PROJECT_FAB_ANIMATION_MS = 220L
        const val HOME_PROJECT_FAB_COLLAPSED_SIZE_DP = 60
        const val HOME_PROJECT_FAB_EXPANDED_HEIGHT_DP = 60
        const val HOME_PROJECT_FAB_EXPANDED_WIDTH_DP = 144
        const val HOME_PROJECT_FAB_ICON_SIZE_DP = 28
        const val HOME_PROJECT_FAB_EXPAND_AT_TOP_DP = 4
        const val HOME_PROJECT_FAB_ICON_GAP_DP = 10
    }
}
