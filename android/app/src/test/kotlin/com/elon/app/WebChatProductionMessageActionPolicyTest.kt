package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProductionMessageActionPolicyTest {
    @Test
    fun keepsNativeChatGptActionsWhenTheOfficialControlScanIsEmpty() {
        val message = message(
            role = "friend",
            provider = WebChatProviderId.CHATGPT_WEB,
            actions = emptySet(),
        )

        assertEquals(
            setOf(WebChatMessageAction.COPY, WebChatMessageAction.MORE),
            WebChatProductionMessageActionPolicy.resolve(message),
        )
    }

    @Test
    fun preservesOfficialActionsWithoutInventingProviderSpecificOnes() {
        val message = message(
            role = "friend",
            provider = WebChatProviderId.GOOGLE_WEB,
            actions = setOf(WebChatMessageAction.REGENERATE),
        )

        assertEquals(
            setOf(WebChatMessageAction.COPY, WebChatMessageAction.REGENERATE),
            WebChatProductionMessageActionPolicy.resolve(message),
        )
    }

    @Test
    fun doesNotOfferNativeActionsWithoutSourceMetadata() {
        assertEquals(
            emptySet<WebChatMessageAction>(),
            WebChatProductionMessageActionPolicy.resolve(ChatMessage("friend", "answer")),
        )
    }

    private fun message(
        role: String,
        provider: WebChatProviderId,
        actions: Set<WebChatMessageAction>,
    ) = ChatMessage(
        role = role,
        content = "answer",
        webChatMessage = WebChatProductionMessage(
            providerWireValue = provider.wireValue,
            sourceMessageId = "source",
            actions = actions,
        ),
    )
}
