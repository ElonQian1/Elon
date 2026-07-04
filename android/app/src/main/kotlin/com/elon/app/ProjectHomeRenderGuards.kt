package com.elon.app

import android.view.View
import com.elon.app.databinding.ActivityMainBinding

internal fun ActivityMainBinding.renderProjectListIfProjectHomeVisible(renderProjectList: () -> Unit) {
    if (
        projectPage.visibility == View.VISIBLE &&
        pageTabs.visibility == View.VISIBLE &&
        projectSpaceAiMenu.visibility != View.VISIBLE
    ) {
        renderProjectList()
    }
}
