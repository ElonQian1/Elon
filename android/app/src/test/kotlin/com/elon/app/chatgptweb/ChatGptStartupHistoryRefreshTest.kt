package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptStartupHistoryRefreshTest {
    private val snapshot = ChatGptWebSnapshot(
        title = "Fixture", url = "https://chatgpt.com/c/fixture", draft = "",
        messages = emptyList(), authenticated = true, composerReady = false, streaming = false,
        currentModel = "", attachments = emptyList(), dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY, privateSendReady = true,
    )

    @Test
    fun readsOnlyOnceWithoutComposerAndDoesNotRepeatOnFurtherSnapshots() {
        val refresh = ChatGptStartupHistoryRefresh(snapshot.url)
        assertTrue(refresh.take(snapshot))
        repeat(10) { assertFalse(refresh.take(snapshot)) }
    }

    @Test
    fun waitsForLiveIdentityAndIdleReadinessWithoutConsumingTheAttempt() {
        val refresh = ChatGptStartupHistoryRefresh(snapshot.url)
        listOf(
            snapshot.copy(authenticated = false), snapshot.copy(contentOnly = true),
            snapshot.copy(privateSendReady = false), snapshot.copy(streaming = true),
            snapshot.copy(dictationActive = true), snapshot.copy(dictationCaptureActive = true),
            snapshot.copy(dictationCapturePending = true), snapshot.copy(loginRequired = true),
            snapshot.copy(url = snapshot.url + "?temporary-chat=true"),
            snapshot.copy(url = "https://chatgpt.com/"),
        ).forEach { assertFalse(refresh.take(it)) }
        assertTrue(refresh.take(snapshot))
    }

    @Test
    fun confirmedUserNavigationOrClearCancelsTheOldRecovery() {
        val navigated = ChatGptStartupHistoryRefresh(snapshot.url)
        assertFalse(navigated.take(snapshot.copy(url = "https://chatgpt.com/c/other")))
        assertFalse(navigated.take(snapshot))
        val cleared = ChatGptStartupHistoryRefresh(snapshot.url)
        cleared.clear()
        assertFalse(cleared.take(snapshot))
        assertFalse(ChatGptStartupHistoryRefresh("https://chatgpt.com/").take(snapshot))
    }

    @Test
    fun projectRouteKeepsItsOwnScope() {
        val project = snapshot.copy(url = "https://chatgpt.com/g/g-p-fixture/c/fixture")
        val refresh = ChatGptStartupHistoryRefresh(project.url)
        assertTrue(refresh.take(project))
        val moved = ChatGptStartupHistoryRefresh(project.url)
        assertFalse(moved.take(snapshot))
        assertFalse(moved.take(project))
    }
}
