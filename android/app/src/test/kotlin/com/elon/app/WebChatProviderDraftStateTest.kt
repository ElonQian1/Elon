package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatProviderDraftStateTest {
    @Test
    fun keepsIndependentProviderDraftsAndRemovesBlankValues() {
        val state = WebChatProviderDraftState()

        state.remember(WebChatProviderId.CHATGPT_WEB, "ChatGPT draft")
        state.remember(WebChatProviderId.GOOGLE_WEB, "Google draft")

        assertEquals("ChatGPT draft", state.restore(WebChatProviderId.CHATGPT_WEB))
        assertEquals("Google draft", state.restore(WebChatProviderId.GOOGLE_WEB))

        state.remember(WebChatProviderId.GOOGLE_WEB, "   ")
        assertEquals("", state.restore(WebChatProviderId.GOOGLE_WEB))
    }

    @Test
    fun codecRestoresOnlyKnownBoundedProviderDrafts() {
        val raw = WebChatProviderDraftCodec.encode(mapOf(
            WebChatProviderId.CHATGPT_WEB to "ChatGPT draft",
            WebChatProviderId.GOOGLE_WEB to "g".repeat(WebChatProviderDraftState.MAX_DRAFT_LENGTH + 10),
        ))

        val decoded = requireNotNull(WebChatProviderDraftCodec.decode(raw))

        assertEquals("ChatGPT draft", decoded[WebChatProviderId.CHATGPT_WEB])
        assertEquals(
            WebChatProviderDraftState.MAX_DRAFT_LENGTH,
            decoded[WebChatProviderId.GOOGLE_WEB]?.length,
        )
    }

    @Test
    fun codecRejectsMalformedOrUnknownSchemaPayloads() {
        assertNull(WebChatProviderDraftCodec.decode("not-json"))
        assertNull(WebChatProviderDraftCodec.decode("""{"schema":"future","drafts":{}}"""))
    }
}
