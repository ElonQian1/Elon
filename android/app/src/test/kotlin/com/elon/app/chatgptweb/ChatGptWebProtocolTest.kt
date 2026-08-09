package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProtocolTest {
    @Test
    fun rejectsEventsFromStalePageAdaptersWhenMinimumVersionIsRequired() {
        val current =
            """{"type":"command_result","adapterVersion":4,"action":"snapshot","ok":true}"""
        val stale =
            """{"type":"command_result","adapterVersion":3,"action":"snapshot","ok":true}"""
        val unversioned = """{"type":"command_result","action":"snapshot","ok":true}"""

        assertTrue(
            ChatGptWebProtocol.parse(current, minimumAdapterVersion = 4) is
                ChatGptWebEvent.CommandResult
        )
        assertNull(ChatGptWebProtocol.parse(stale, minimumAdapterVersion = 4))
        assertNull(ChatGptWebProtocol.parse(unversioned, minimumAdapterVersion = 4))
        assertTrue(ChatGptWebProtocol.parse(unversioned) is ChatGptWebEvent.CommandResult)
    }

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
                "dictationActive":true,
                "attachments":[
                  {"id":"attachment_ab12","name":"需求.txt","state":"ready","removable":true},
                  {"id":"../unsafe","name":"忽略.txt","state":"ready","removable":true}
                ],
                "capabilities":["streaming","conversation_list","invalid capability"],
                "messages":[
                  {"id":"u1","role":"user","state":"completed","content":[{"type":"text","text":"你好"}]},
                  {"id":"a1","role":"assistant","state":"streaming","content":[
                    {"type":"markdown","text":"## 你好\n\n需要什么帮助？"},
                    {"type":"image","text":"生成的图片"},
                    {"type":"file","text":"分析结果.csv"},
                    {"type":"script","text":"忽略"}
                  ]},
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
        assertTrue(event.value.dictationActive)
        assertEquals("需求.txt", event.value.attachments.single().name)
        assertTrue(event.value.attachments.single().removable)
        assertEquals(2, event.value.messages.size)
        assertEquals("assistant", event.value.messages.last().role)
        assertEquals("streaming", event.value.messages.last().state)
        assertTrue(event.value.messages.last().content.startsWith("## 你好"))
        assertEquals(listOf("image", "file"), event.value.messages.last().parts.map { it.type })
        assertEquals("生成的图片", event.value.messages.last().parts.first().label)
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
        assertTrue(
            ChatGptWebProtocol.parse(
                """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"remove_attachment","xRatio":0.5,"yRatio":0.5}}""",
            ) is ChatGptWebEvent.WebTouchRequest,
        )
        assertTrue(
            ChatGptWebProtocol.parse(
                """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"list_navigation","xRatio":0.5,"yRatio":0.5}}""",
            ) is ChatGptWebEvent.WebTouchRequest,
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
    fun parsesBoundedDynamicFeatureNavigation() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"navigation_snapshot",
                "features":[
                  {"id":"feature_ab12","label":"文件库","kind":"library","selected":true},
                  {"id":"../bad","label":"忽略","kind":"settings","selected":false},
                  {"id":"feature_cd34","label":"自定义入口","kind":"unknown","selected":false}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.FeatureNavigation

        assertEquals(2, event.features.size)
        assertEquals("library", event.features.first().kind)
        assertTrue(event.features.first().selected)
        assertEquals("navigation", event.features.last().kind)
    }

    @Test
    fun parsesVersionedUiManifestAndRejectsUnsafeControls() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"ui_manifest_snapshot",
                "version":3,
                "pageKind":"home",
                "title":"工作",
                "compatibility":"healthy",
                "controls":[
                  {"id":"control_navigation","semantic":"navigation","label":"打开导航","region":"header","role":"button","enabled":true},
                  {"id":"control_suggestion_ab12","semantic":"suggestion","label":"帮我整理待办","region":"suggestions","role":"button","enabled":true,"inViewport":false},
                  {"id":"control_conversation_ab12","semantic":"conversation","label":"桥接验证","region":"overlay","role":"link","enabled":true,"contextId":"demo"},
                  {"id":"control_message_ab12_share_cd34","semantic":"share","label":"分享","region":"message","role":"button","enabled":true,"contextId":"conversation-turn-4","xRatio":0.8,"yRatio":0.6},
                  {"id":"../unsafe","semantic":"action","label":"忽略","region":"header","role":"button","enabled":true},
                  {"id":"control_unknown","semantic":"future_kind","label":"未来功能","region":"overlay","role":"menuitem","enabled":false}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.UiManifest

        assertEquals("工作", event.value.title)
        assertEquals("healthy", event.value.compatibility)
        assertEquals(5, event.value.controls.size)
        assertEquals("suggestion", event.value.controls[1].semantic)
        assertFalse(event.value.controls[1].inViewport)
        assertEquals("conversation", event.value.controls[2].semantic)
        assertEquals("demo", event.value.controls[2].contextId)
        assertEquals("conversation-turn-4", event.value.controls[3].contextId)
        assertEquals("message", event.value.controls[3].region)
        assertEquals(0.8, event.value.controls[3].webXRatio ?: -1.0, 0.0)
        assertEquals(0.6, event.value.controls[3].webYRatio ?: -1.0, 0.0)
        assertEquals("action", event.value.controls.last().semantic)
        assertEquals("chatgpt-control:control_navigation:打开导航", event.value.controls.first().accessibilityLabel)
    }

    @Test
    fun genericUiTouchRequiresAValidCurrentControlId() {
        val valid = ChatGptWebProtocol.parse(
            """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"invoke_ui_control","controlId":"control_suggestion_ab12","xRatio":0.5,"yRatio":0.8}}""",
        ) as ChatGptWebEvent.WebTouchRequest

        assertEquals("control_suggestion_ab12", valid.controlId)
        assertNull(
            ChatGptWebProtocol.parse(
                """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"invoke_ui_control","xRatio":0.5,"yRatio":0.8}}""",
            ),
        )
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
