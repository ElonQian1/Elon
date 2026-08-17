package com.elon.app

internal object WebChatSendFallbackPolicy {
    enum class Action {
        RETRY_IN_PLACE,
        RETRY_GUEST_ACCESS,
    }

    fun decide(loginRequired: Boolean): Action =
        if (loginRequired) Action.RETRY_GUEST_ACCESS else Action.RETRY_IN_PLACE
}
