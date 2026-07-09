package com.elon.app

import android.view.View
import android.widget.LinearLayout
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

    fun setup() {
        binding.ensureConversationPageScrollable()
        binding.homeProjectCreateMenu.setOnClickListener { handleClick() }
        syncStyle()
        reset()
    }

    fun showProjectPlazaEntry() {
        show(EntryMode.PROJECT_PLAZA)
    }

    fun showHomeEntry() {
        show(EntryMode.HOME)
    }

    private fun show(targetMode: EntryMode) {
        setMode(targetMode)
        binding.homeProjectCreateMenu.visibility = View.VISIBLE
        reset()
        binding.homeProjectCreateMenu.bringToFront()
    }

    fun hide() {
        binding.homeProjectCreateMenu.visibility = View.GONE
    }

    private fun reset() {
        applyFrame()
    }

    private fun setMode(targetMode: EntryMode) {
        if (mode == targetMode) return
        mode = targetMode
        syncStyle()
        applyFrame()
    }

    private fun handleClick() {
        when (mode) {
            EntryMode.PROJECT_PLAZA -> openProjectPlaza()
            EntryMode.HOME -> openHome()
        }
    }

    private fun applyFrame() {
        val menu = binding.homeProjectCreateMenu
        val size = dp(HOME_PROJECT_FAB_SIZE_DP)
        val menuParams = menu.layoutParams
        if (menuParams.width != size || menuParams.height != size) {
            menuParams.width = size
            menuParams.height = size
            menu.layoutParams = menuParams
        }

        val iconSize = dp(HOME_PROJECT_FAB_ICON_SIZE_DP)
        val homeParams = binding.homeProjectHomeIcon.layoutParams as LinearLayout.LayoutParams
        val projectParams = binding.homeProjectCreateIcon.layoutParams as LinearLayout.LayoutParams

        if (mode == EntryMode.PROJECT_PLAZA) {
            homeParams.width = 0
            homeParams.height = iconSize
            homeParams.marginEnd = 0
            projectParams.width = iconSize
            projectParams.height = iconSize
            projectParams.marginEnd = 0
            binding.homeProjectHomeIcon.alpha = 0f
            binding.homeProjectCreateIcon.alpha = 1f
        } else {
            homeParams.width = iconSize
            homeParams.height = iconSize
            homeParams.marginEnd = 0
            projectParams.width = 0
            projectParams.height = iconSize
            projectParams.marginEnd = 0
            binding.homeProjectHomeIcon.alpha = 1f
            binding.homeProjectCreateIcon.alpha = 0f
        }
        binding.homeProjectHomeIcon.layoutParams = homeParams
        binding.homeProjectCreateIcon.layoutParams = projectParams
        menu.bringToFront()
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

    private companion object {
        const val HOME_PROJECT_FAB_SIZE_DP = 60
        const val HOME_PROJECT_FAB_ICON_SIZE_DP = 28
    }
}
