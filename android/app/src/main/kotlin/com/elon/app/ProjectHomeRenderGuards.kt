package com.elon.app

import android.view.View
import com.elon.app.databinding.ActivityMainBinding

internal fun ActivityMainBinding.renderProjectListIfProjectHomeVisible(renderProjectList: () -> Unit) {
    if (isProjectHomeSurfaceVisible()) {
        renderProjectList()
    }
}

internal fun ActivityMainBinding.isProjectHomeSurfaceVisible(): Boolean =
    projectPage.visibility == View.VISIBLE &&
        projectTopTabs.visibility == View.VISIBLE &&
        projectSpaceAiMenu.visibility != View.VISIBLE

internal fun ActivityMainBinding.isProjectSpaceSurfaceVisible(): Boolean =
    projectPage.visibility == View.VISIBLE &&
        projectTopTabs.visibility != View.VISIBLE

internal fun ActivityMainBinding.canShowProjectSpaceAiMenu(): Boolean =
    isProjectSpaceSurfaceVisible() &&
        chatPage.visibility != View.VISIBLE &&
        inputLayout.visibility != View.VISIBLE
