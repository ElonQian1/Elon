package com.elon.app

import android.view.View
import android.view.ViewOutlineProvider
import android.widget.ScrollView
import android.widget.TextView
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainBottomNavigationController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val selectTab: (TextView) -> Unit,
    private val showHomeActions: (View, TextView) -> Unit
) {
    fun setup() {
        binding.bottomNavContent.outlineProvider = ViewOutlineProvider.BACKGROUND
        binding.bottomNavContent.clipToOutline = true
        binding.pageTabs.post { applyStitchScale() }

        binding.tabChat.setOnClickListener { selectTab(binding.tabChat) }
        binding.tabProject.setOnClickListener { selectTab(binding.tabProject) }
        binding.tabProfile.setOnClickListener { selectTab(binding.tabProfile) }
        binding.bottomComposeButton.setOnClickListener {
            val tab = if (
                binding.projectPage.visibility == View.VISIBLE ||
                binding.marketplacePage.visibility == View.VISIBLE
            ) binding.tabProject else binding.tabChat
            showHomeActions(binding.bottomComposeButton, tab)
        }
    }

    private fun applyStitchScale() {
        val designViewport = activity.resources.getDimensionPixelSize(
            R.dimen.main_bottom_menu_design_viewport_width
        ).toFloat()
        val scale = (binding.pageTabs.width / designViewport).coerceAtMost(1f)
        binding.bottomNavContent.apply {
            pivotX = width / 2f
            pivotY = height.toFloat()
            scaleX = scale
            scaleY = scale
            (layoutParams as? FrameLayout.LayoutParams)?.let { params ->
                params.bottomMargin = (
                    activity.resources.getDimensionPixelSize(R.dimen.main_bottom_menu_edge_gap) * scale
                ).toInt()
                layoutParams = params
            }
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
