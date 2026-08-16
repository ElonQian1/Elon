package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class SocialAiComposerDraftCoordinatorTest {
    @Test
    fun keepsWorkAndEachProviderDraftIndependent() {
        var input = "work draft"
        var providerChanges = 0
        val coordinator = SocialAiComposerDraftCoordinator(
            providerDrafts = WebChatProviderDraftState(),
            readText = { input },
            writeText = { input = it },
            onProviderDraftChanged = { providerChanges += 1 },
        )

        coordinator.activateWorkMode()
        coordinator.onTextChanged("updated work")
        input = "updated work"
        coordinator.activateProvider(WebChatProviderId.CHATGPT_WEB)
        assertEquals("", input)

        input = "ChatGPT draft"
        coordinator.onTextChanged(input)
        coordinator.activateProvider(WebChatProviderId.GOOGLE_WEB)
        assertEquals("", input)

        input = "Google draft"
        coordinator.onTextChanged(input)
        coordinator.activateProvider(WebChatProviderId.CHATGPT_WEB)
        assertEquals("ChatGPT draft", input)

        coordinator.activateWorkMode()
        assertEquals("updated work", input)
        assertEquals(2, providerChanges)
    }

    @Test
    fun releasePreventsAnotherFriendInputFromReplacingProviderDraft() {
        var input = ""
        val state = WebChatProviderDraftState()
        val coordinator = SocialAiComposerDraftCoordinator(
            providerDrafts = state,
            readText = { input },
            writeText = { input = it },
            onProviderDraftChanged = {},
        )

        coordinator.activateProvider(WebChatProviderId.CHATGPT_WEB)
        input = "kept draft"
        coordinator.onTextChanged(input)
        coordinator.release()

        input = "another friend draft"
        coordinator.onTextChanged(input)
        coordinator.activateProvider(WebChatProviderId.CHATGPT_WEB)

        assertEquals("kept draft", input)
        assertEquals("kept draft", state.restore(WebChatProviderId.CHATGPT_WEB))
    }
}
