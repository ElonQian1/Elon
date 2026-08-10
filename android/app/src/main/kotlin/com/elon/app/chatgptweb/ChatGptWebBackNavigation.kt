package com.elon.app.chatgptweb

internal object ChatGptWebBackNavigation {
    enum class Action {
        DISMISS_OFFICIAL_OVERLAY,
        NAVIGATE_WEB_HISTORY,
        EXIT_OFFICIAL_VIEW,
        FINISH_ACTIVITY,
    }

    fun decide(
        manifest: ChatGptWebUiManifest?,
        canGoBack: Boolean,
        officialViewActive: Boolean,
    ): Action = when {
        manifest?.controls.orEmpty().any { it.region == ChatGptWebUiRegion.OVERLAY } ->
            Action.DISMISS_OFFICIAL_OVERLAY
        canGoBack -> Action.NAVIGATE_WEB_HISTORY
        officialViewActive -> Action.EXIT_OFFICIAL_VIEW
        else -> Action.FINISH_ACTIVITY
    }
}
