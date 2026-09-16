package com.elon.app

import com.elon.app.chatgptweb.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class GroupGoogleAiTest {
    private val google = GroupAiConfiguration(GroupAiEngine.GOOGLE,
        listOf(GroupAiModelChoice("高")), GroupWorkAiConfiguration("work-a", "Model A", false))

    @Test fun providerMenuIncludesGoogleAndExactlyOneSelection() {
        GroupAiEngine.entries.forEach { selected ->
            val options = GroupAiProviderChoices.options(google.copy(engine = selected))
            assertEquals(listOf("CHATGPT", "GOOGLE", "WORK"), options.map { it.id })
            assertEquals(listOf(selected.name), options.filter { it.selected }.map { it.id })
            assertTrue(options.all { it.enabled })
            assertEquals("group-ai-provider:GOOGLE", options[1].selector)
            assertEquals(WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB).avatarResId, options[1].avatarResId)
        }
    }

    @Test fun switchingAndRestartKeepIndependentGroupSettings() {
        val restored = GroupAiConfigurationStore.decode(GroupAiConfigurationStore.encode(google))
        assertEquals(google, restored)
        assertTrue(restored.usesWebAi)
        assertEquals("Google AI", restored.label)
        assertEquals("高", restored.copy(engine = GroupAiEngine.CHATGPT).label)
        assertEquals("Model A", restored.copy(engine = GroupAiEngine.WORK).label)
        assertFalse(restored.copy(engine = GroupAiEngine.WORK).usesWebAi)
    }

    @Test fun googleDoesNotInheritChatGptDispatchFromOldServer() {
        val legacy = JSONObject().put("dispatch_permit", true)
        assertFalse(GroupWebAiRequestPolicy.permitted(google, legacy))
        assertTrue(GroupWebAiRequestPolicy.permitted(GroupAiConfiguration(), legacy))
        val receipt = JSONObject().put("dispatch_permit", true).put("web_provider", "google_web")
        assertTrue(GroupWebAiRequestPolicy.permitted(google, receipt))
        assertFalse(GroupWebAiRequestPolicy.permitted(GroupAiConfiguration(), receipt))
        assertFalse(GroupWebAiRequestPolicy.permitted(google, receipt.put("dispatch_permit", false)))
    }

    @Test fun googleConsentDoesNotClaimTemporaryHistory() {
        val text = GroupWebAiRequestPolicy.consent(google)
        assertTrue(text.contains("Google"))
        assertTrue(text.contains("保存"))
        assertFalse(text.contains("临时"))
        assertTrue(GroupWebAiRequestPolicy.consent(GroupAiConfiguration()).contains("ChatGPT 临时会话"))
    }

    @Test fun onlyEmptyGoogleAiDocumentCanReceiveGroupPrompt() {
        val provider = WebChatProviderId.GOOGLE_WEB
        assertEquals("https://www.google.com/aimode", GroupWebAiSessionPolicy.startUrl(provider))
        assertTrue(GroupWebAiSessionPolicy.ready(snapshot(), provider))
        assertTrue(GroupWebAiSessionPolicy.ready(snapshot().copy(url = "https://www.google.com/webhp?aep=11&hl=zh"), provider))
        listOf("https://www.google.com/search?q=personal&udm=50", "https://www.google.com/search?udm=50&csuir=old",
            "https://www.google.com/aimode?csuir=old", "https://google.com.evil.test/aimode",
            "https://www.google.com:444/aimode", "https://user@www.google.com/aimode", "https://www.google.com/search?q=hello",
            "https://chatgpt.com/?temporary-chat=true").forEach {
            assertFalse(it, GroupWebAiSessionPolicy.ready(snapshot().copy(url = it), provider))
        }
        assertFalse(GroupWebAiSessionPolicy.ready(snapshot().copy(draft = "personal"), provider))
        assertFalse(GroupWebAiSessionPolicy.ready(snapshot().copy(messages = listOf(message("user", "old"))), provider))
        assertFalse(GroupWebAiSessionPolicy.ready(snapshot().copy(streaming = true), provider))
        assertFalse(GroupWebAiSessionPolicy.ready(snapshot().copy(loginRequired = true), provider))
        assertFalse(GroupWebAiSessionPolicy.ready(snapshot().copy(composerReady = false, privateSendReady = true), provider))
    }

    @Test fun chatGptStillRequiresItsTemporaryDocument() {
        val value = snapshot().copy(url = "https://chatgpt.com/?temporary-chat=true", composerReady = false, privateSendReady = true)
        assertTrue(GroupWebAiSessionPolicy.ready(value, WebChatProviderId.CHATGPT_WEB))
        assertFalse(GroupWebAiSessionPolicy.ready(value.copy(url = "https://chatgpt.com/c/personal"), WebChatProviderId.CHATGPT_WEB))
    }

    @Test fun googleCompletedReplyRequiresMatchingGroupPromptAndFinishedAnswer() {
        val messages = listOf(message("user", "group\n prompt"), message("assistant", "answer"))
        assertEquals("answer", GroupWebAiExecutor.completedReply(messages, "group prompt", false, "idle"))
        assertNull(GroupWebAiExecutor.completedReply(messages, "other", false, "idle"))
        assertNull(GroupWebAiExecutor.completedReply(messages, "group prompt", true, "idle"))
        assertNull(GroupWebAiExecutor.completedReply(messages.map { it.copy(state = "streaming") }, "group prompt", false, "idle"))
    }

    private fun message(role: String, text: String) = ChatGptWebMessage(role, role, text, "completed", emptyList())
    private fun snapshot() = ChatGptWebSnapshot(
        title = "Google", url = "https://www.google.com/aimode", draft = "", messages = emptyList(),
        authenticated = false, composerReady = true, streaming = false, currentModel = "Google AI 模式",
        attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY,
    )
}
