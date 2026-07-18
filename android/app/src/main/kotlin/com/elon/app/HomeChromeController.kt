package com.elon.app

import android.widget.PopupWindow
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
        setNavigationBarColor(R.color.elon_bg_app)
        setBottomMenuVisible(true)
        binding.projectSpaceAiMenu.visibility = android.view.View.GONE
        binding.homeMenuButton.visibility = android.view.View.GONE
        projectCreateFab.hide()
    }

    fun showProjectPlazaEntry() {
        setNavigationBarColor(R.color.elon_bg_app)
        setBottomMenuVisible(true)
        binding.projectSpaceAiMenu.visibility = android.view.View.GONE
        binding.homeMenuButton.visibility = android.view.View.GONE
        projectCreateFab.hide()
    }

    fun showMenuOnly() {
        setNavigationBarColor(R.color.elon_bg_app)
        setBottomMenuVisible(true)
        binding.projectSpaceAiMenu.visibility = android.view.View.GONE
        binding.homeMenuButton.visibility = android.view.View.GONE
        projectCreateFab.hide()
    }

    fun hide() {
        binding.homeMenuButton.visibility = android.view.View.GONE
        projectCreateFab.hide()
    }

    fun clearTranslations() {
        binding.homeProjectCreateMenu.translationX = 0f
    }

}
