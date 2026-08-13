package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProtocolTest {
    @Test
    fun exposesBoundedDocumentTokensAndAdapterReadyEvents() {
        val parsed = ChatGptWebProtocol.parseMessage(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "adapterVersion":35,
              "documentToken":"doc_page_7",
              "event":{"type":"adapter_ready","capabilities":["draft_sync"]}
            }
            """.trimIndent(),
            minimumAdapterVersion = 35,
        )

        assertEquals("doc_page_7", parsed?.documentToken)
        val ready = parsed?.event as ChatGptWebEvent.AdapterReady
        assertTrue(ready.capabilities.supports(ChatGptWebCapabilityId.DRAFT_SYNC))
        assertNull(
            ChatGptWebProtocol.parseMessage(
                """{"type":"command_result","adapterVersion":35,"documentToken":"../old","action":"snapshot","ok":true}""",
                minimumAdapterVersion = 35,
            )?.documentToken,
        )
    }

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
    fun parsesOnlyBoundedMcpRequestIdsFromCommandResults() {
        val correlated = ChatGptWebProtocol.parse(
            """{"type":"command_result","adapterVersion":28,"action":"send_prompt","ok":true,"requestId":"mcp_a9"}""",
        ) as ChatGptWebEvent.CommandResult
        val malformed = ChatGptWebProtocol.parse(
            """{"type":"command_result","adapterVersion":28,"action":"send_prompt","ok":true,"requestId":"../../other"}""",
        ) as ChatGptWebEvent.CommandResult

        assertEquals("mcp_a9", correlated.requestId)
        assertNull(malformed.requestId)
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
                "pageKind":"conversation",
                "loginRequired":false,
                "composerReady":true,
                "streaming":false,
                "currentModel":"5.6 Sol 轻度",
                "dictationActive":true,
                "observedMessageCount":43,
                "messageWindowStart":40,
                "attachments":[
                  {"id":"attachment_ab12","name":"需求.txt","state":"ready","removable":true},
                  {"id":"../unsafe","name":"忽略.txt","state":"ready","removable":true}
                ],
                "capabilities":["streaming","conversation_list","invalid capability"],
                "messages":[
                  {"id":"u1","role":"user","state":"completed","content":[{"type":"text","text":"你好"}]},
                  {"id":"a1","role":"assistant","state":"streaming","content":[
                    {"type":"markdown","text":"## 你好\n\n需要什么帮助？"},
                    {"type":"image","text":"生成的图片","kind":"image","mediaType":"image/png"},
                    {"type":"file","text":"分析结果.csv","kind":"download","mediaType":"text/csv","targetKind":"external","targetHost":"files.example.com"},
                    {"type":"code","text":"Kotlin 代码","kind":"code_block","language":"kotlin","lineCount":12},
                    {"type":"table","text":"表格","kind":"table","rowCount":4,"columnCount":3},
                    {"type":"math","text":"E = mc^2"},
                    {"type":"chart","text":"季度趋势图"},
                    {"type":"interactive","text":"可展开结果"},
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
        assertEquals("conversation", event.value.pageKind)
        assertFalse(event.value.loginRequired)
        assertTrue(event.value.composerReady)
        assertFalse(event.value.streaming)
        assertEquals("5.6 Sol 轻度", event.value.currentModel)
        assertTrue(event.value.dictationActive)
        assertEquals("需求.txt", event.value.attachments.single().name)
        assertTrue(event.value.attachments.single().removable)
        assertEquals(2, event.value.messages.size)
        assertEquals(40, event.value.messageWindowStart)
        assertEquals(43, event.value.observedMessageCount)
        assertEquals("assistant", event.value.messages.last().role)
        assertEquals("streaming", event.value.messages.last().state)
        assertTrue(event.value.messages.last().content.startsWith("## 你好"))
        assertEquals(
            listOf("image", "file", "code", "table", "math", "chart", "interactive"),
            event.value.messages.last().parts.map { it.type },
        )
        assertEquals("生成的图片", event.value.messages.last().parts.first().label)
        assertEquals("image/png", event.value.messages.last().parts.first().metadata?.mediaType)
        assertEquals("files.example.com", event.value.messages.last().parts[1].metadata?.targetHost)
        assertEquals("kotlin", event.value.messages.last().parts[2].metadata?.language)
        assertEquals(12, event.value.messages.last().parts[2].metadata?.lineCount)
        assertEquals(4, event.value.messages.last().parts[3].metadata?.rowCount)
        assertEquals(3, event.value.messages.last().parts[3].metadata?.columnCount)
        assertTrue(event.value.capabilities.supports(ChatGptWebCapabilityId.CONVERSATION_LIST))
        assertFalse(event.value.capabilities.supports("invalid capability"))
    }

    @Test
    fun rejectsUnboundedOrCredentialLikeStructuredMetadata() {
        val event = ChatGptWebProtocol.parse(
            """{"schema":"yilong.ai.ui.v1","event":{"type":"message_snapshot","messages":[{"id":"a1","role":"assistant","state":"completed","content":[{"type":"markdown","text":"answer"},{"type":"citation","text":"source","kind":"reference","language":"bad language","mediaType":"text/html;token=secret","targetKind":"credential","targetHost":"user:password@example.com","targetPath":"/private?token=secret","lineCount":-1,"rowCount":999999}]}]}}""",
        ) as ChatGptWebEvent.Snapshot

        val metadata = event.value.messages.single().parts.single().metadata
        assertEquals("reference", metadata?.kind)
        assertEquals(null, metadata?.language)
        assertEquals(null, metadata?.mediaType)
        assertEquals(null, metadata?.targetKind)
        assertEquals(null, metadata?.targetHost)
        assertEquals(null, metadata?.lineCount)
        assertEquals(null, metadata?.rowCount)
    }

    @Test
    fun boundsAuthenticationEvidenceFromSnapshots() {
        val event = ChatGptWebProtocol.parse(
            """{"schema":"yilong.ai.ui.v1","event":{"type":"message_snapshot","url":"https://chatgpt.com/tasks","pageKind":"unsupported","loginRequired":true}}""",
        ) as ChatGptWebEvent.Snapshot

        assertEquals("unknown", event.value.pageKind)
        assertTrue(event.value.loginRequired)
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
                  {"id":"model_ab12","label":"模型 GPT-5.6 Sol","selected":false,"kind":"menuitem","semantic":"model","opensSubmenu":true},
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
        assertEquals("模型 GPT-5.6 Sol", event.options.single().label)
        assertFalse(event.options.single().selected)
        assertEquals("model", event.options.single().semantic)
        assertTrue(event.options.single().opensSubmenu)
    }

    @Test
    fun parsesStableToolSemanticsAndFallsBackForFutureOptions() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"composer_controls_snapshot",
                "section":"tools",
                "currentModel":"5.6 Sol 轻度",
                "options":[
                  {"id":"tools_camera","label":"拍摄新照片","kind":"menuitem","semantic":"attachment_camera"},
                  {"id":"tools_search","label":"搜索互联网","kind":"menuitem","semantic":"web_search"},
                  {"id":"tools_future","label":"未来工具","kind":"menuitem","semantic":"future_tool"}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.ComposerControls

        assertEquals(
            listOf("attachment_camera", "web_search", "tool"),
            event.options.map(ChatGptWebComposerOption::semantic),
        )
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
        assertTrue(
            ChatGptWebProtocol.parse(
                """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"open_model_submenu","xRatio":0.5,"yRatio":0.5}}""",
            ) is ChatGptWebEvent.WebTouchRequest,
        )
        listOf("regenerate_open_menu", "regenerate_retry").forEach { purpose ->
            val regenerateTouch = ChatGptWebProtocol.parse(
                """{"schema":"yilong.ai.ui.v1","event":{"type":"web_touch_request","purpose":"$purpose","xRatio":0.5,"yRatio":0.5}}""",
            ) as ChatGptWebEvent.WebTouchRequest
            assertEquals(purpose, regenerateTouch.purpose)
        }
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
                "collection":{
                  "scrollerFound":true,
                  "scrolled":true,
                  "scrollRestored":true,
                  "reachedEnd":true,
                  "truncated":false,
                  "timedOut":false,
                  "observedCount":3,
                  "steps":4
                },
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
        assertTrue(event.collection.scrollerFound)
        assertTrue(event.collection.scrolled)
        assertTrue(event.collection.scrollRestored)
        assertTrue(event.collection.reachedEnd)
        assertFalse(event.collection.truncated)
        assertFalse(event.collection.timedOut)
        assertEquals(1, event.collection.observedCount)
        assertEquals(4, event.collection.steps)
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
                  {"id":"feature_health","label":"健康","kind":"health","selected":false},
                  {"id":"feature_cd34","label":"自定义入口","kind":"unknown","selected":false}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.FeatureNavigation

        assertEquals(3, event.features.size)
        assertEquals("library", event.features.first().kind)
        assertTrue(event.features.first().selected)
        assertEquals("health", event.features[1].kind)
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
                "version":4,
                "pageKind":"home",
                "title":"工作",
                "compatibility":"healthy",
                "discoveredControlCount":10,
                "controlsTruncated":false,
                "controls":[
                  {"id":"control_navigation","semantic":"navigation","label":"打开导航","region":"header","role":"button","enabled":true},
                  {"id":"control_web_search","semantic":"action","label":"搜索","region":"composer","role":"button","enabled":true,"selected":true},
                  {"id":"control_suggestion_ab12","semantic":"suggestion","label":"帮我整理待办","region":"suggestions","role":"button","enabled":true,"inViewport":false},
                  {"id":"control_conversation_ab12","semantic":"conversation","label":"桥接验证","region":"overlay","role":"link","enabled":true,"contextId":"demo"},
                  {"id":"control_message_ab12_share_cd34","semantic":"share","label":"分享","region":"message","role":"button","enabled":true,"contextId":"conversation-turn-4","xRatio":0.8,"yRatio":0.6},
                  {"id":"control_sources_ab12","semantic":"sources","label":"文件和来源","region":"header","role":"button","enabled":true},
                  {"id":"control_more_ab12","semantic":"more","label":"更多操作","region":"message","role":"button","enabled":true,"contextId":"conversation-turn-4"},
                  {"id":"control_create_asset_ab12","semantic":"create_asset","label":"创建图片","region":"content","role":"button","enabled":true},
                  {"id":"../unsafe","semantic":"action","label":"忽略","region":"header","role":"button","enabled":true},
                  {"id":"control_unknown","semantic":"future_kind","label":"未来功能","region":"overlay","role":"menuitem","enabled":false}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.UiManifest

        assertEquals("工作", event.value.title)
        assertEquals("healthy", event.value.compatibility)
        assertEquals(10, event.value.discoveredControlCount)
        assertFalse(event.value.controlsTruncated)
        assertEquals(9, event.value.controls.size)
        assertEquals(ChatGptWebUiSemantics.WEB_SEARCH, event.value.controls[1].semantic)
        assertTrue(event.value.controls[1].selected)
        assertEquals("suggestion", event.value.controls[2].semantic)
        assertFalse(event.value.controls[2].inViewport)
        assertEquals("conversation", event.value.controls[3].semantic)
        assertEquals("demo", event.value.controls[3].contextId)
        assertEquals("conversation-turn-4", event.value.controls[4].contextId)
        assertEquals("message", event.value.controls[4].region)
        assertEquals(0.8, event.value.controls[4].webXRatio ?: -1.0, 0.0)
        assertEquals(0.6, event.value.controls[4].webYRatio ?: -1.0, 0.0)
        assertEquals("sources", event.value.controls[5].semantic)
        assertEquals("more", event.value.controls[6].semantic)
        assertEquals(ChatGptWebUiRegion.CONTENT, event.value.controls[7].region)
        assertEquals("create_asset", event.value.controls[7].semantic)
        assertEquals("action", event.value.controls.last().semantic)
        assertEquals("chatgpt-control:control_navigation:打开导航", event.value.controls.first().accessibilityLabel)
    }

    @Test
    fun parsesWritableFormControlsWithoutAllowingCredentialInjection() {
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"ui_manifest_snapshot",
                "version":8,
                "pageKind":"feature",
                "title":"设置",
                "compatibility":"healthy",
                "controls":[
                  {"id":"control_search_ab12","semantic":"search","label":"搜索","region":"content","role":"textbox","inputKind":"search","writable":true},
                  {"id":"control_password_cd34","semantic":"text_input","label":"密码","region":"content","role":"textbox","inputKind":"password","writable":true},
                  {"id":"control_toggle_ef56","semantic":"toggle","label":"启用","region":"content","role":"checkbox","inputKind":"checkbox","writable":true,"stateSettable":true,"selected":true},
                  {"id":"control_model_gh78","semantic":"selection","label":"模型","region":"content","role":"combobox","inputKind":"select","choiceLabels":["快速","思考"],"selectedChoiceIndex":1},
                  {"id":"control_effort_ij90","semantic":"slider","label":"思考强度","region":"content","role":"slider","inputKind":"range","sliderSettable":true,"sliderMin":0,"sliderMax":2,"sliderStep":0.5,"sliderValue":1.5},
                  {"id":"control_menu_kl12","semantic":"toggle","label":"快速","region":"overlay","role":"menuitemradio","inputKind":"radio","stateSettable":true,"selected":true},
                  {"id":"control_tab_op56","semantic":"selection","label":"常规","region":"overlay","role":"tab","inputKind":"tab","stateSettable":true,"selected":true},
                  {"id":"control_temporary_chat","semantic":"temporary_chat","label":"关闭临时聊天","region":"header","role":"button","stateSettable":true,"selected":true},
                  {"id":"control_tree_mn34","semantic":"navigation","label":"项目","region":"content","role":"treeitem","expanded":false,"expandable":true}
                ]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.UiManifest

        val search = event.value.controls[0]
        val password = event.value.controls[1]
        val toggle = event.value.controls[2]
        val selection = event.value.controls[3]
        val slider = event.value.controls[4]
        val menuRadio = event.value.controls[5]
        val tab = event.value.controls[6]
        val temporaryChat = event.value.controls[7]
        val disclosure = event.value.controls[8]
        assertEquals("search", search.inputKind)
        assertTrue(search.supportsTextEntry)
        assertFalse(password.writable)
        assertFalse(password.supportsTextEntry)
        assertEquals("checkbox", toggle.role)
        assertTrue(toggle.selected)
        assertFalse(toggle.writable)
        assertTrue(toggle.supportsSelectedState)
        assertEquals(listOf("快速", "思考"), selection.choiceLabels)
        assertEquals(1, selection.selectedChoiceIndex)
        assertTrue(selection.supportsChoiceSelection)
        assertTrue(slider.supportsSliderValue)
        assertEquals(0.0, slider.slider?.min ?: -1.0, 0.0)
        assertEquals(2.0, slider.slider?.max ?: -1.0, 0.0)
        assertEquals(0.5, slider.slider?.step ?: -1.0, 0.0)
        assertEquals(1.5, slider.slider?.value ?: -1.0, 0.0)
        assertTrue(menuRadio.supportsSelectedState)
        assertEquals("tab", tab.inputKind)
        assertTrue(tab.selected)
        assertTrue(tab.supportsSelectedState)
        assertTrue(temporaryChat.selected)
        assertTrue(temporaryChat.supportsSelectedState)
        assertEquals(false, disclosure.expanded)
        assertTrue(disclosure.supportsExpandedState)
    }

    @Test
    fun boundsLargeUiManifestsAndReportsDiscoveryTruncation() {
        val controls = (1..512).joinToString(",") { index ->
            """{"id":"control_action_$index","semantic":"action","label":"操作 $index","region":"content","role":"button"}"""
        }
        val event = ChatGptWebProtocol.parse(
            """
            {
              "schema":"yilong.ai.ui.v1",
              "event":{
                "type":"ui_manifest_snapshot",
                "version":4,
                "pageKind":"feature",
                "title":"应用",
                "compatibility":"healthy",
                "discoveredControlCount":620,
                "controlsTruncated":true,
                "controls":[$controls]
              }
            }
            """.trimIndent(),
        ) as ChatGptWebEvent.UiManifest

        assertEquals(512, event.value.controls.size)
        assertEquals(620, event.value.discoveredControlCount)
        assertTrue(event.value.controlsTruncated)
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
