package com.elon.app

import android.widget.TextView
import com.elon.app.databinding.ActivityMainBinding

internal enum class MainBottomNavigationDestination {
    CHAT,
    PROJECT,
    PROFILE,
    MENU
}

internal fun selectedMainBottomNavigationDestination(
    currentPage: MainBottomNavigationDestination,
    isProjectBrowserOpen: Boolean
): MainBottomNavigationDestination {
    return if (isProjectBrowserOpen) {
        MainBottomNavigationDestination.MENU
    } else {
        currentPage
    }
}

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
        val selected = selectedMainBottomNavigationDestination(currentPage, isProjectBrowserOpen)
        listOf(
            binding.tabChat to MainBottomNavigationDestination.CHAT,
            binding.tabProject to MainBottomNavigationDestination.PROJECT,
            binding.tabProfile to MainBottomNavigationDestination.PROFILE
        ).forEach { (tab, destination) ->
            updateTabVisual(tab, destination == selected)
        }
        val menuSelected = selected == MainBottomNavigationDestination.MENU
        binding.bottomMenuButton.isSelected = menuSelected
        binding.bottomMenuSelection.isSelected = menuSelected
        binding.bottomMenuIcon.isSelected = menuSelected
    }
}
