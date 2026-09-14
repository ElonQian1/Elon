package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class ChatGptPrivateTextInputReadinessTest {
    private val page = ChatGptWebSnapshot(
        title = "", url = "https://chatgpt.com/c/11111111-1111-4111-8111-111111111111",
        draft = "synthetic draft", messages = emptyList(), authenticated = true,
        composerReady = false, streaming = false, currentModel = "auto", attachments = emptyList(),
        dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY, privateSendReady = true,
    )

    @Test fun privateInputDoesNotPretendTheDomComposerExists() {
        assertTrue(ChatGptWebAccessPolicy.canSendText(page))
        assertFalse(ChatGptWebAccessPolicy.canChat(page))
        assertFalse(page.composerReady)
        assertTrue(allows(page))
        assertFalse(WebChatSendContextPolicy.allows(true, page, false, null, null))
        assertFalse(allows(page.copy(streaming = true)))
        assertFalse(WebChatSendContextPolicy.allows(true, page, true, null, null, allowPrivateText = true))
        assertFalse(WebChatSendContextPolicy.allows(true, page, false, "/c/other", "/c/current", allowPrivateText = true))
    }

    @Test fun staleOrUnauthenticatedPrivateHintsCannotAdmitASend() {
        listOf(page.copy(privateSendReady = false), page.copy(authenticated = false),
            page.copy(loginRequired = true), page.copy(accessReason = "rate_limited"),
            page.copy(contentOnly = true), page.copy(url = "https://example.com/"),
            page.copy(url = "https://chatgpt.com/auth/login")).forEach {
            assertFalse(ChatGptWebAccessPolicy.canSendText(it))
            assertFalse(allows(it))
        }
    }

    @Test fun onlyTextCommandsCanUseIndependentReadiness() {
        listOf("set_input_text", "send_input", "chatgpt_set_page_input_text", "chatgpt_send_page_input").forEach {
            assertNull(ChatGptWebOperationReadiness.rejection(it, page, true, false))
            assertEquals("adapter_generation_not_ready", ChatGptWebOperationReadiness.rejection(it, page, false, false))
            assertEquals("bridge_not_ready", ChatGptWebOperationReadiness.rejection(it, page.copy(privateSendReady = false), true, false))
        }
        listOf("chatgpt_start_dictation", "chatgpt_start_realtime_voice", "chatgpt_attach_library_file").forEach {
            assertEquals("bridge_not_ready", ChatGptWebOperationReadiness.rejection(it, page, true, false))
        }
        assertFalse(ChatGptWebTransientComposerReadiness.reconcile(page, page, true).composerReady)
    }

    @Test fun previousDomReadinessCannotRelabelTheVerifiedPrivateEditorDuringMenusOrDictation() {
        val previous = page.copy(composerReady = true, privateSendReady = false, currentModel = "Fast")
        for (dictation in listOf(false, true)) {
            val incoming = page.copy(dictationActive = dictation, currentModel = "")
            val merged = ChatGptWebTransientComposerReadiness.reconcile(
                previous, incoming, composerInteractionActive = !dictation,
            )
            assertFalse(merged.composerReady)
            assertTrue(merged.privateSendReady)
            assertTrue(ChatGptWebAccessPolicy.canSendText(merged))
            assertFalse(ChatGptWebAccessPolicy.canChat(merged))
            assertEquals("Fast", merged.currentModel)
        }
    }

    @Test fun protocolKeepsDomAndPrivateStateSeparateAndDropsReadinessOnContentOnlyEvents() {
        fun parse(extra: String) = (ChatGptWebProtocol.parse(
            """{"schema":"yilong.ai.ui.v1","event":{"type":"message_snapshot","composerReady":false,"authenticated":true,$extra}}""",
        ) as ChatGptWebEvent.Snapshot).value
        assertTrue(parse("\"privateSendReady\":true").privateSendReady)
        assertFalse(parse("\"privateSendReady\":true").composerReady)
        assertFalse(parse("\"privateSendReady\":true,\"snapshotScope\":\"content\"").privateSendReady)
        assertFalse(parse("\"draft\":\"\"").privateSendReady)
    }

    private fun allows(snapshot: ChatGptWebSnapshot) =
        WebChatSendContextPolicy.allows(true, snapshot, false, null, null, allowPrivateText = true)
}
