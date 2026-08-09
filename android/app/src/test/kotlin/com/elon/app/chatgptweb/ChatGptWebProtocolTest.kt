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
                "draft":"继续补充",
                "authenticated":true,
                "composerReady":true,
                "streaming":false,
                "currentModel":"5.6 Sol 轻度",
                "capabilities":["streaming","conversation_list","invalid capability"],
                "messages":[
                  {"id":"u1","role":"user","state":"completed","content":[{"type":"text","text":"你好"}]},
                  {"id":"a1","role":"assistant","state":"streaming","content":[{"type":"markdown","text":"## 你好\n\n需要什么帮助？"}]},
                  {"id":"tool","role":"tool","state":"completed","content":[{"type":"text","text":"not projected"}]}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.Snapshot

        assertEquals("测试会话", event.value.title)
        assertEquals("继续补充", event.value.draft)
        assertTrue(event.value.authenticated)
        assertTrue(event.value.composerReady)
        assertFalse(event.value.streaming)
        assertEquals("5.6 Sol 轻度", event.value.currentModel)
        assertEquals(2, event.value.messages.size)
        assertEquals("assistant", event.value.messages.last().role)
        assertEquals("streaming", event.value.messages.last().state)
        assertTrue(event.value.messages.last().content.startsWith("## 你好"))
        assertTrue(event.value.capabilities.supports(ChatGptWebCapabilityId.CONVERSATION_LIST))
        assertFalse(event.value.capabilities.supports("invalid capability"))
    }

    @Test
    fun parsesComposerControlsAndRejectsUnsafeOptionIds() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"composer_controls_snapshot",
                "section":"model",
                "currentModel":"5.6 Sol 轻度",
                "options":[
                  {"id":"model_ab12","label":"轻度","selected":true,"kind":"menuitemradio"},
                  {"id":"../unsafe","label":"错误选项","selected":false,"kind":"menuitem"},
                  {"id":"model_blank","label":"  ","selected":false,"kind":"menuitem"}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.ComposerControls

        assertEquals("model", event.section)
        assertEquals("5.6 Sol 轻度", event.currentModel)
        assertEquals(1, event.options.size)
        assertEquals("轻度", event.options.single().label)
        assertTrue(event.options.single().selected)
    }

    @Test
    fun acceptsOnlyBoundedWhitelistedWebTouchRequests() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"web_touch_request",
                "purpose":"list_composer_tools",
                "xRatio":0.15,
                "yRatio":0.91
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.WebTouchRequest

        assertEquals("list_composer_tools", event.purpose)
        assertEquals(0.15, event.xRatio, 0.0)
        assertNull(
            ChatGptWebProtocol.parse(
                """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"tap_anywhere","xRatio":0.5,"yRatio":0.5}}""",
            ),
        )
        assertNull(
            ChatGptWebProtocol.parse(
                """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"list_composer_tools","xRatio":1.5,"yRatio":0.5}}""",
            ),
        )
    }

    @Test
    fun parsesBoundedConversationListsAndRejectsUnsafePaths() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"conversation_snapshot",
                "conversations":[
                  {"id":"one","title":"第一场会话","path":"/c/one","active":true},
                  {"id":"bad","title":"越界地址","path":"https://example.com/c/bad"},
                  {"id":"blank","title":"  ","path":"/c/blank"}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.ConversationList

        assertEquals(1, event.conversations.size)
        assertEquals("第一场会话", event.conversations.single().title)
        assertTrue(event.conversations.single().active)
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
