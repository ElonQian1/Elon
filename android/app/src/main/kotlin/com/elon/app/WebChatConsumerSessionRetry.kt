package com.elon.app

internal fun retryWebChatConsumerSession(controller: WebChatSocialController) {
    val retried = if (controller.stateWireValue() == "login_required") {
        controller.retryGuestAccess()
    } else {
        controller.retryConnection()
    }
    if (!retried) controller.onHostResumed()
}
