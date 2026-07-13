package com.elon.app

import android.view.View
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainBottomNavigationController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val selectTab: (TextView) -> Unit,
    private val showCreateProjectDialog: () -> Unit,
    private val showFriendLocalSearch: () -> Unit
) {
    fun setup() {
        binding.tabChat.setOnClickListener { selectTab(binding.tabChat) }
        binding.tabProject.setOnClickListener { selectTab(binding.tabProject) }
        binding.tabProfile.setOnClickListener { selectTab(binding.tabProfile) }
        binding.bottomNewProjectButton.setOnClickListener { showCreateProjectDialog() }
        binding.bottomSearchButton.setOnClickListener {
            if (!binding.tabChat.isSelected) selectTab(binding.tabChat)
            showFriendLocalSearch()
        }
    }

    fun setVisible(visible: Boolean) {
        binding.pageTabs.visibility = if (visible) View.VISIBLE else View.GONE
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
}
