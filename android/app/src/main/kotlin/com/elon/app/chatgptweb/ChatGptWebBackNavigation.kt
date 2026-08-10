package com.elon.app.chatgptweb

internal object ChatGptWebBackNavigation {
    enum class Action {
        DISMISS_OFFICIAL_OVERLAY,
        NAVIGATE_WEB_HISTORY,
        FINISH_ACTIVITY,
    }

    fun decide(manifest: ChatGptWebUiManifest?, canGoBack: Boolean): Action = when {
        manifest?.controls.orEmpty().any { it.region == ChatGptWebUiRegion.OVERLAY } ->
            Action.DISMISS_OFFICIAL_OVERLAY
        canGoBack -> Action.NAVIGATE_WEB_HISTORY
        else -> Action.FINISH_ACTIVITY
    }
}
