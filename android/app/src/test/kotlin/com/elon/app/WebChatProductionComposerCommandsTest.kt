package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionComposerCommandsTest {
    private val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)

    @Test
    fun offersDictationAndRealtimeVoiceWhenIdle() {
        val commands = WebChatProductionComposerCommandCatalog.resolve(chatGpt, JSONObject())

        assertEquals(
            listOf("chatgpt_start_dictation", "chatgpt_start_realtime_voice"),
            commands.map { it.action },
        )
        assertEquals(
            "web-chat-composer-command:chatgpt_web:start-dictation",
            commands.first().nativeSelector,
        )
    }

    @Test
    fun replacesVoiceActionsWithStopWhileStreaming() {
        val commands = WebChatProductionComposerCommandCatalog.resolve(
            chatGpt,
            JSONObject().put("streaming", true),
        )

        assertEquals(listOf("chatgpt_stop_generation"), commands.map { it.action })
    }

    @Test
    fun offersSubmitInsteadOfStartingAnotherDictation() {
        val commands = WebChatProductionComposerCommandCatalog.resolve(
            chatGpt,
            JSONObject().put("dictation_active", true),
        )

        assertEquals(listOf("chatgpt_submit_dictation"), commands.map { it.action })
    }

    @Test
    fun doesNotExposeUnsupportedCommandsForGoogle() {
        val google = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)

        assertTrue(WebChatProductionComposerCommandCatalog.resolve(google, JSONObject()).isEmpty())
    }
}
