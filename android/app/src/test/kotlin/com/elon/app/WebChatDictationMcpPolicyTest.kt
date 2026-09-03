package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatDictationMcpPolicyTest {
    @Test
    fun routesProductionStartSubmitAndCancelSelectors() {
        val idle = WebChatDictationMcpPolicy.snapshot(PRIVATE_START, null)
        assertEquals("idle", idle.phase)
        assertEquals("private", idle.transport)
        assertEquals(WebChatDictationMcpTarget.START_OR_SUBMIT, route("start_web_chat_dictation", idle))
        assertEquals(
            WebChatDictationMcpTarget.TOGGLE_MODE,
            route("toggle_web_chat_dictation_mode", idle),
        )

        val sharedIdle = WebChatDictationMcpPolicy.snapshot(SHARED_START, null)
        assertEquals("shared", sharedIdle.transport)
        assertEquals(WebChatDictationMcpTarget.START_OR_SUBMIT, route("start_web_chat_dictation", sharedIdle))

        val privateActive = WebChatDictationMcpPolicy.snapshot(PRIVATE_SUBMIT, PRIVATE_CANCEL)
        assertEquals("active", privateActive.phase)
        assertEquals("private", privateActive.transport)
        assertEquals(WebChatDictationMcpTarget.START_OR_SUBMIT, route("submit_web_chat_dictation", privateActive))
        assertEquals(WebChatDictationMcpTarget.CANCEL, route("cancel_web_chat_dictation", privateActive))

        val sharedActive = WebChatDictationMcpPolicy.snapshot(SHARED_SUBMIT, SHARED_CANCEL)
        assertEquals("shared", sharedActive.transport)
        assertNull(route("start_web_chat_dictation", sharedActive))
    }

    @Test
    fun identifiesDomStartupWithoutTreatingItAsSubmit() {
        val starting = WebChatDictationMcpPolicy.snapshot(DOM_STARTING, null)
        assertEquals("starting", starting.phase)
        assertEquals("official_dom", starting.transport)
        assertNull(route("submit_web_chat_dictation", starting))
    }

    @Test
    fun startupAndFailureTakePriorityOverOptimisticOfficialSessionControls() {
        val starting = WebChatDictationMcpPolicy.snapshot(DOM_STARTING, DOM_CANCEL)
        assertEquals("starting", starting.phase)
        assertNull(route("submit_web_chat_dictation", starting))
        assertEquals(
            WebChatDictationMcpTarget.CANCEL,
            route("cancel_web_chat_dictation", starting),
        )

        val failed = WebChatDictationMcpPolicy.snapshot(DOM_START_FAILED, DOM_CANCEL)
        assertEquals("failed", failed.phase)
        assertNull(route("submit_web_chat_dictation", failed))
        assertEquals(
            WebChatDictationMcpTarget.CANCEL,
            route("cancel_web_chat_dictation", failed),
        )
    }

    @Test
    fun ignoresRealtimeVoiceSelectorWhileDictationIsIdle() {
        val idle = WebChatDictationMcpPolicy.snapshot(PRIVATE_START, REALTIME_VOICE)

        assertEquals("idle", idle.phase)
        assertEquals("private", idle.transport)
        assertNull(idle.cancelSelector)
    }

    private fun route(action: String, state: WebChatDictationMcpSnapshot) =
        WebChatDictationMcpPolicy.target(action, state)

    private companion object {
        const val PRIVATE_START = "web-chat-composer-command:private:start-dictation"
        const val SHARED_START = "web-chat-composer-command:shared:start-dictation"
        const val PRIVATE_SUBMIT = "web-chat-composer-command:private:submit-dictation"
        const val PRIVATE_CANCEL = "web-chat-composer-command:private:cancel-dictation"
        const val SHARED_SUBMIT = "web-chat-composer-command:shared:submit-dictation"
        const val SHARED_CANCEL = "web-chat-composer-command:shared:cancel-dictation"
        const val DOM_STARTING = "web-chat-composer-command:dom:starting-dictation"
        const val DOM_START_FAILED = "web-chat-composer-command:dom:start-failed-dictation"
        const val DOM_CANCEL = "web-chat-composer-command:dom:cancel-dictation"
        const val REALTIME_VOICE = "web-chat-composer-command:chatgpt_web:start-realtime-voice"
    }
}
