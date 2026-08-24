package com.elon.app

internal enum class WebChatBackgroundResumeAction {
    NONE,
    RETRY_DEFERRED_LOAD,
    RETRY_FAILED_PAGE,
    REPAIR_FINISHED_PAGE,
    WATCH_IN_FLIGHT_PAGE,
}

/** Keeps provider switches warm without treating an in-flight page as a failed page. */
internal object WebChatBackgroundResumePolicy {
    fun decide(
        loadDeferred: Boolean,
        pageSupported: Boolean,
        pageFailed: Boolean,
        pageLoading: Boolean,
        pageProgress: Int,
    ): WebChatBackgroundResumeAction = when {
        loadDeferred -> WebChatBackgroundResumeAction.RETRY_DEFERRED_LOAD
        !pageSupported -> WebChatBackgroundResumeAction.NONE
        pageFailed -> WebChatBackgroundResumeAction.RETRY_FAILED_PAGE
        pageLoading && pageProgress >= PAGE_FINISHED_PROGRESS ->
            WebChatBackgroundResumeAction.REPAIR_FINISHED_PAGE
        pageLoading -> WebChatBackgroundResumeAction.WATCH_IN_FLIGHT_PAGE
        else -> WebChatBackgroundResumeAction.NONE
    }

    private const val PAGE_FINISHED_PROGRESS = 100
}
