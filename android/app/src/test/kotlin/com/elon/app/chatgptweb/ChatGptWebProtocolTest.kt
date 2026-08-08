package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProtocolTest {
    @Test
    fun parsesBoundedConversationSnapshots() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "providerId":"chatgpt",
              "source":"official_web",
              "sequence":1,
              "emittedAt":"2026-08-08T00:00:00Z",
              "event":{
                "type":"message_snapshot",
                "title":"测试会话",
                "url":"https://chatgpt.com/c/example",
                "authenticated":true,
                "composerReady":true,
                "streaming":false,
                "messages":[
                  {"id":"u1","role":"user","state":"completed","content":[{"type":"text","text":"你好"}]},
                  {"id":"a1","role":"assistant","state":"completed","content":[{"type":"text","text":"你好，需要什么帮助？"}]},
                  {"id":"tool","role":"tool","state":"completed","content":[{"type":"text","text":"not projected"}]}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.Snapshot

        assertEquals("测试会话", event.value.title)
        assertTrue(event.value.authenticated)
        assertTrue(event.value.composerReady)
        assertFalse(event.value.streaming)
        assertEquals(2, event.value.messages.size)
        assertEquals("assistant", event.value.messages.last().role)
    }

    @Test
    fun treatsMissingAuthenticationSignalAsLoggedOut() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"message_snapshot",
                "composerReady":true,
                "messages":[]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.Snapshot

        assertFalse(event.value.authenticated)
        assertTrue(event.value.composerReady)
    }

    @Test
    fun rejectsUnknownOrMalformedPayloads() {
        assertNull(ChatGptWebProtocol.parse("not-json"))
        assertNull(ChatGptWebProtocol.parse("{\"type\":\"credential\",\"value\":\"secret\"}"))
    }
}
