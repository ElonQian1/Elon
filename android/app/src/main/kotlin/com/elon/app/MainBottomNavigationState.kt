package com.elon.app

import android.widget.TextView
import com.elon.app.databinding.ActivityMainBinding

internal enum class MainBottomNavigationDestination {
    CHAT,
    PROJECT,
    PROFILE
}

internal data class MainBottomNavigationRenderState(
    val selectedPage: MainBottomNavigationDestination,
    val isMenuActivated: Boolean
)

internal fun mainBottomNavigationRenderState(
    currentPage: MainBottomNavigationDestination,
    isProjectBrowserOpen: Boolean
) = MainBottomNavigationRenderState(
    selectedPage = currentPage,
    isMenuActivated = isProjectBrowserOpen
)

internal class MainBottomNavigationSelectionState(
    private val binding: ActivityMainBinding,
    private val updateTabVisual: (TextView, Boolean) -> Unit
) {
    private var currentPage = MainBottomNavigationDestination.CHAT
    private var isProjectBrowserOpen = false

    fun selectPage(tab: TextView) {
        currentPage = when (tab) {
            binding.tabProject -> MainBottomNavigationDestination.PROJECT
            binding.tabProfile -> MainBottomNavigationDestination.PROFILE
            else -> MainBottomNavigationDestination.CHAT
        }
        render()
    }

    fun setProjectBrowserOpen(isOpen: Boolean) {
        isProjectBrowserOpen = isOpen
        render()
    }

    private fun render() {
        val state = mainBottomNavigationRenderState(currentPage, isProjectBrowserOpen)
        listOf(
            binding.tabChat to MainBottomNavigationDestination.CHAT,
            binding.tabProject to MainBottomNavigationDestination.PROJECT,
            binding.tabProfile to MainBottomNavigationDestination.PROFILE
        ).forEach { (tab, destination) ->
            updateTabVisual(tab, destination == state.selectedPage)
        }
        binding.bottomMenuButton.isSelected = false
        binding.bottomMenuButton.isActivated = state.isMenuActivated
        binding.bottomMenuIcon.isActivated = state.isMenuActivated
    }
}
