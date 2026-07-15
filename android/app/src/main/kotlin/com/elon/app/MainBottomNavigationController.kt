package com.elon.app

import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainBottomNavigationController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val selectTab: (TextView) -> Unit,
    private val createConversationAndOpen: () -> Unit
) {
    fun setup() {
        binding.tabChat.setOnClickListener { selectTab(binding.tabChat) }
        binding.tabProject.setOnClickListener { selectTab(binding.tabProject) }
        binding.tabProfile.setOnClickListener { selectTab(binding.tabProfile) }
        binding.bottomComposeButton.setOnClickListener {
            createConversationAndOpen()
        }
    }

    fun setVisible(visible: Boolean) {
        binding.pageTabs.visibility = if (visible) View.VISIBLE else View.GONE
        applyHomeDashboardMode(
            visible &&
                binding.conversationPage.visibility == View.VISIBLE &&
                binding.chatPage.visibility != View.VISIBLE
        )
        val inset = if (visible) {
            activity.resources.getDimensionPixelSize(R.dimen.main_bottom_menu_outer_height)
        } else {
            0
        }
        listOfNotNull(
            binding.conversationPage.parent as? ScrollView,
            binding.projectScrollView,
            binding.profilePage,
            binding.marketplacePage
        ).forEach { view ->
            view.setPadding(view.paddingLeft, view.paddingTop, view.paddingRight, inset)
            view.clipToPadding = false
        }
    }

    private fun applyHomeDashboardMode(enabled: Boolean) {
        binding.bottomComposeButton.visibility = if (enabled) View.GONE else View.VISIBLE
        binding.bottomComposeGap.visibility = if (enabled) View.GONE else View.VISIBLE
        binding.bottomNavPrimaryBackground.setImageResource(
            if (enabled) R.drawable.bg_home_workspace_bottom_nav
            else R.drawable.bg_bottom_nav_primary_panel
        )
        (binding.bottomNavPrimaryPanel.layoutParams as? LinearLayout.LayoutParams)?.let { params ->
            params.marginStart = if (enabled) dp(10) else 0
            params.marginEnd = if (enabled) dp(10) else 0
            binding.bottomNavPrimaryPanel.layoutParams = params
        }
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density + 0.5f).toInt()
}
