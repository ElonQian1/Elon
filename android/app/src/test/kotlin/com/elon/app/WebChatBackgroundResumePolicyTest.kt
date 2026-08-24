package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatBackgroundResumePolicyTest {
    @Test
    fun aDeferredInitialLoadRetriesBeforeInspectingTheCurrentDocument() {
        assertEquals(
            WebChatBackgroundResumeAction.RETRY_DEFERRED_LOAD,
            decide(loadDeferred = true, pageSupported = false),
        )
    }

    @Test
    fun anUnsupportedPageDoesNotStartRecovery() {
        assertEquals(
            WebChatBackgroundResumeAction.NONE,
            decide(pageSupported = false, pageFailed = true, pageLoading = true),
        )
    }

    @Test
    fun aFailedSupportedPageUsesTheBoundedReloadBudget() {
        assertEquals(
            WebChatBackgroundResumeAction.RETRY_FAILED_PAGE,
            decide(pageFailed = true),
        )
    }

    @Test
    fun aFinishedDocumentRepairsTheAdapterBeforeReloading() {
        assertEquals(
            WebChatBackgroundResumeAction.REPAIR_FINISHED_PAGE,
            decide(pageLoading = true, pageProgress = 100),
        )
    }

    @Test
    fun anInFlightDocumentKeepsLoadingAndOnlyRestartsTheWatchdog() {
        assertEquals(
            WebChatBackgroundResumeAction.WATCH_IN_FLIGHT_PAGE,
            decide(pageLoading = true, pageProgress = 63),
        )
    }

    private fun decide(
        loadDeferred: Boolean = false,
        pageSupported: Boolean = true,
        pageFailed: Boolean = false,
        pageLoading: Boolean = false,
        pageProgress: Int = 0,
    ) = WebChatBackgroundResumePolicy.decide(
        loadDeferred = loadDeferred,
        pageSupported = pageSupported,
        pageFailed = pageFailed,
        pageLoading = pageLoading,
        pageProgress = pageProgress,
    )
}
