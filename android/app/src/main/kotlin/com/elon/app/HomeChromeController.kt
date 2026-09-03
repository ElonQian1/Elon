package com.elon.app

import android.widget.PopupWindow
import android.widget.FrameLayout
import android.view.Gravity
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class HomeChromeController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val actionPopupProvider: () -> PopupWindow?,
    private val dp: (Int) -> Int,
    private val setNavigationBarColor: (Int) -> Unit,
    private val setBottomMenuVisible: (Boolean) -> Unit,
    private val showFriendsHome: () -> Unit,
    private val showProjectPlaza: () -> Unit,
    private val toggleProjectBrowser: () -> Unit
) {
    private val projectCreateFab = HomeProjectCreateFabController(
        binding = binding,
        dp = dp,
        openProjectPlaza = showProjectPlaza,
        openHome = showFriendsHome
    )

    fun setup() {
        projectCreateFab.setup()
        binding.homeMenuButton.setOnClickListener {
            actionPopupProvider()?.dismiss()
            toggleProjectBrowser()
        }
        binding.bottomMenuButton.setOnClickListener {
            actionPopupProvider()?.dismiss()
            toggleProjectBrowser()
        }
    }

    fun showHome() {
        setNavigationBarColor(R.color.elon_home_bg)
        binding.toolbar.setBackgroundColor(activity.elonColor(R.color.elon_home_bg))
        binding.contentContainer.setBackgroundColor(activity.elonColor(R.color.elon_home_bg))
        setBottomMenuVisible(true)
        binding.projectSpaceAiMenu.visibility = android.view.View.GONE
        binding.homeMenuButton.visibility = android.view.View.VISIBLE
        binding.topTitleText.apply {
            text = "消息"
            textSize = 24f
            typeface = android.graphics.Typeface.create("sans-serif", android.graphics.Typeface.BOLD)
            gravity = Gravity.CENTER_VERTICAL
            layoutParams = (layoutParams as FrameLayout.LayoutParams).apply {
                gravity = Gravity.START or Gravity.CENTER_VERTICAL
                marginStart = dp(52)
            }
        }
        projectCreateFab.hide()
    }

    fun showProjectPlazaEntry() {
        binding.toolbar.setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        binding.contentContainer.setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        setNavigationBarColor(R.color.elon_bg_app)
        setBottomMenuVisible(true)
        binding.projectSpaceAiMenu.visibility = android.view.View.GONE
        binding.homeMenuButton.visibility = android.view.View.GONE
        projectCreateFab.hide()
    }

    fun showMenuOnly() {
        binding.toolbar.setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        binding.contentContainer.setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        setNavigationBarColor(R.color.elon_bg_app)
        setBottomMenuVisible(true)
        binding.projectSpaceAiMenu.visibility = android.view.View.GONE
        binding.homeMenuButton.visibility = android.view.View.GONE
        projectCreateFab.hide()
    }

    fun hide() {
        binding.homeMenuButton.visibility = android.view.View.GONE
        binding.topTitleText.apply {
            textSize = 16f
            gravity = Gravity.CENTER
            layoutParams = (layoutParams as FrameLayout.LayoutParams).apply {
                gravity = Gravity.CENTER
                marginStart = 0
            }
        }
        projectCreateFab.hide()
    }

    fun clearTranslations() {
        binding.homeProjectCreateMenu.translationX = 0f
    }

}
