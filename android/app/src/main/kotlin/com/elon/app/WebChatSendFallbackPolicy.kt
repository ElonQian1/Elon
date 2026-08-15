package com.elon.app

internal object WebChatSendFallbackPolicy {
    enum class Action {
        RETRY_IN_PLACE,
        OPEN_OFFICIAL_AUTHENTICATION,
    }

    fun decide(loginRequired: Boolean): Action =
        if (loginRequired) Action.OPEN_OFFICIAL_AUTHENTICATION else Action.RETRY_IN_PLACE
}
