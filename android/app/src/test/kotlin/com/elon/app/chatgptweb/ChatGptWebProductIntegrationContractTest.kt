package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProductIntegrationContractTest {
    @Test
    fun socialAiChatKeepsWorkModeAndAddsNativeWebChatProviders() {
        val controller = read("android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt")
        val chatController = read("android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt")
        val googleController = read("android/app/src/main/kotlin/com/elon/app/GoogleWebSocialChatController.kt")
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val provider = read("android/app/src/main/kotlin/com/elon/app/WebChatProvider.kt")
        val navigationSession = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatNavigationSession.kt",
        )
        val friendChat = read("android/app/src/main/kotlin/com/elon/app/MainFriendChatActions.kt")
        val main = read("android/app/src/main/kotlin/com/elon/app/MainActivity.kt")
        val mainMcp = read("android/app/src/main/kotlin/com/elon/app/MainMcpNativeControlActions.kt")
        val fixtureActions = read(
            "android/app/src/main/kotlin/com/elon/app/ChatGptWebAcceptanceAttachmentNativeActions.kt",
        )
        val official = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebOfficialActivity.kt",
        )
        val strings = read("android/app/src/main/res/values/strings.xml")

        assertTrue(strings.contains("ChatGPT 网页 AI"))
        assertTrue(strings.contains("工作模式"))
        assertTrue(strings.contains("聊天模式"))
        assertTrue(controller.contains("SocialAiInteractionMode.WORK"))
        assertTrue(controller.contains("SocialAiInteractionMode.CHAT"))
        assertTrue(controller.contains("SocialAiModeSegmentedControl"))
        assertTrue(!controller.contains("showProviderSelector"))
        assertTrue(controller.contains("openOfficialFallback"))
        assertTrue(provider.contains("GOOGLE_WEB"))
        assertTrue(provider.contains("id = WebChatProviderId.GOOGLE_WEB"))
        assertTrue(provider.contains("available = true"))
        assertTrue(navigationSession.contains("WebChatNavigationSessionRegistry"))
        assertTrue(navigationSession.contains("session.capabilities.all(provider.capabilities::contains)"))
        assertTrue(navigationSession.contains("REQUIRED_NATIVE_NAVIGATION.all(session.capabilities::contains)"))
        assertTrue(chatController.contains("ChatGptFriendMessageMapper.map"))
        assertTrue(chatController.contains("WebChatSendCoordinator("))
        assertTrue(chatController.contains("OfficialPageWebChatSendTransport("))
        assertTrue(chatController.contains("sendCoordinator.observeSnapshot"))
        assertTrue(chatController.contains("sendCoordinator.acceptCommandResult"))
        assertTrue(chatController.contains("WebChatPendingSendSnapshotPresentation.resolve"))
        assertTrue(googleController.contains("GoogleWebBackgroundSession"))
        assertTrue(googleController.contains("ChatGptFriendMessageMapper.map"))
        assertTrue(chatController.contains("session.sendAttachments"))
        assertTrue(chatController.contains("chatAttachmentsFromPending"))
        assertTrue(friendChat.contains("onActiveFriendChanged(friend)"))
        assertTrue(friendChat.contains("suspendForExternalChat"))
        assertTrue(main.contains("socialAiChatFeature.trySendMessage"))
        assertTrue(feature.contains("activateWorkMode"))
        assertTrue(feature.contains("activateChatProvider"))
        assertTrue(feature.contains("deactivateChatProvider"))
        assertTrue(feature.contains("webChatNavigationSessions.session(providerId())"))
        assertTrue(feature.contains("providerId = WebChatProviderId.GOOGLE_WEB"))
        assertTrue(!feature.contains("providerId() != WebChatProviderId.CHATGPT_WEB"))
        assertTrue(feature.contains("WEB_CHAT_MODEL_BUTTON_OWNER"))
        assertTrue(mainMcp.contains("set_social_ai_interaction_mode"))
        assertTrue(mainMcp.contains("select_web_chat_provider"))
        assertTrue(mainMcp.contains("open_chatgpt_official_fallback"))
        assertTrue(mainMcp.contains("open_web_chat_official_fallback"))
        assertTrue(
            mainMcp.substringAfter("\"open_chatgpt_official_fallback\"")
                .substringBefore("\"open_web_chat_official_fallback\"")
                .contains("openChatGptWeb()"),
        )
        assertTrue(mainMcp.contains("web_chat_attachment_phase"))
        assertTrue(mainMcp.contains("web_chat_last_command"))
        assertTrue(mainMcp.contains("web_chat_streaming"))
        assertTrue(feature.contains("webChatLastCommandStatus"))
        assertTrue(mainMcp.contains("start_new_web_chat_conversation"))
        assertTrue(mainMcp.contains("open_web_chat_conversation"))
        assertTrue(mainMcp.contains("open_web_chat_project"))
        assertTrue(mainMcp.contains("refresh_web_chat_conversations"))
        assertTrue(mainMcp.contains("get_web_chat_navigation"))
        assertTrue(mainMcp.contains("feature.webChatNavigationAvailable()"))
        assertTrue(mainMcp.contains("set_web_chat_sidebar"))
        assertTrue(mainMcp.contains("open_chat_side_menu"))
        assertTrue(mainMcp.contains("close_chat_side_menu"))
        assertTrue(mainMcp.contains("chat_side_menu_open"))
        assertTrue(mainMcp.contains("chatgpt_web_acceptance_attachment"))
        assertTrue(fixtureActions.contains("stage_chatgpt_web_acceptance_attachment"))
        assertTrue(fixtureActions.contains("remove_chatgpt_web_acceptance_attachment"))
        assertTrue(official.contains("class ChatGptWebOfficialActivity"))
        assertTrue(official.contains("ChatGptWebOfficialFallbackIntent.startUrl(intent)"))
        assertTrue(official.contains("fileChooserController.show("))
        assertFalse(official.contains("ChatGptWebPageAdapter"))
        assertTrue(chatController.contains("WebChatSendFallbackPolicy.decide"))
        assertTrue(chatController.contains("session.onHostResumed()"))
        assertTrue(chatController.contains("session.deactivate()"))
        assertTrue(googleController.contains("WebChatSendFallbackPolicy.decide(loginRequired = false)"))
        assertTrue(googleController.contains("session.onHostResumed()"))
        assertTrue(googleController.contains("sendCoordinator.pauseWatchdog()"))
        assertTrue(googleController.contains("session.deactivate()"))
        val chatPageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )
        val googlePageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt",
        )
        assertTrue(chatPageAdapter.contains("fun onHostPaused()"))
        assertTrue(googlePageAdapter.contains("fun onHostPaused()"))
    }

    @Test
    fun webChatProvidersReplayPrivateCachesBeforeStartingTheirWebViews() {
        val chatGpt = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val google = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )
        val store = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/WebChatSnapshotStore.kt",
        )

        assertTrue(chatGpt.contains("WebChatSnapshotStore(activity, \"chatgpt\")"))
        assertTrue(google.contains("WebChatSnapshotStore(activity, \"google\")"))
        assertTrue(chatGpt.indexOf("latestSnapshot?.let(onSnapshot)") < chatGpt.indexOf("ensureInitialized()"))
        assertTrue(google.indexOf("latestSnapshot?.let(onSnapshot)") < google.indexOf("ensureInitialized()"))
        assertTrue(
            chatGpt.indexOf("onConversationIndexChanged(conversationIndex())") <
                chatGpt.indexOf("ensureInitialized()"),
        )
        assertTrue(
            google.indexOf("onConversationIndexChanged(conversationIndex())") <
                google.indexOf("ensureInitialized()"),
        )
        assertTrue(store.contains("context.noBackupFilesDir"))
        assertTrue(store.contains("AtomicFile"))
        assertFalse(store.contains("CookieManager"))
    }

    @Test
    fun pinnedStateIsDetectedButNeverPresentedAsAConversationDateGroup() {
        val adapter = read(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val index = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebConversationIndex.kt",
        )
        val sideMenu = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt",
        )

        assertTrue(adapter.contains("pinned: pinnedFor(node)"))
        assertTrue(index.contains("NON_TEMPORAL_GROUP_LABELS"))
        assertTrue(index.contains("setOf(\"pinned\", \"已置顶\", \"置顶\")"))
        assertFalse(sideMenu.contains("conversation.groupLabel.takeIf"))
    }

    @Test
    fun adaptiveMirrorAndMcpShareStableSemanticControlIds() {
        val layoutAdapter = read("android/app/src/main/assets/chatgpt_web_adapter_layout.js")
        val portalPolicy = read(
            "android/app/src/main/assets/chatgpt_web_adapter_message_portal_policy.js",
        )
        val pageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )
        val mcp = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpActions.kt")
        val messageJson = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMessageJson.kt")
        val background = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val mcpBinding = read("android/app/src/main/kotlin/com/elon/app/mcp/McpNativeControlBinding.kt")

        assertTrue(layoutAdapter.contains("ui_manifest_snapshot"))
        assertTrue(layoutAdapter.contains("invoke_ui_control"))
        assertTrue(layoutAdapter.contains("compatibility:"))
        assertTrue(layoutAdapter.contains("addMessageControls"))
        assertTrue(layoutAdapter.contains("return 'conversation';"))
        assertTrue(
            layoutAdapter.contains(
                "['temporary_chat', 'conversation_options', 'save_to_project'].includes(pageSemantic)",
            ),
        )
        assertTrue(layoutAdapter.contains("return 'conversation_options';"))
        assertTrue(layoutAdapter.contains("contextId: resolvedContextId"))
        assertTrue(layoutAdapter.contains("messagePortalPolicy.inferMessageContext"))
        assertTrue(portalPolicy.contains("function inferMessageIndex"))
        assertTrue(portalPolicy.contains("role: input && input.role"))
        assertTrue(
            pageAdapter.indexOf("chatgpt_web_adapter_message_portal_policy.js") <
                pageAdapter.indexOf("chatgpt_web_adapter_layout.js"),
        )
        assertTrue(
            pageAdapter.indexOf("chatgpt_web_adapter_streaming_policy.js") <
                pageAdapter.indexOf("chatgpt_web_stream_watchdog_probe.js"),
        )
        assertTrue(
            pageAdapter.indexOf("chatgpt_web_stream_watchdog_probe.js") <
                pageAdapter.indexOf("chatgpt_web_stream_watchdog_acceptance.js"),
        )
        assertTrue(
            pageAdapter.indexOf("chatgpt_web_stream_watchdog_acceptance.js") <
                pageAdapter.indexOf("chatgpt_web_adapter.js"),
        )
        assertTrue(layoutAdapter.contains("scrollIntoView"))
        val baseAdapter = read("android/app/src/main/assets/chatgpt_web_adapter.js")
        val watchdogProbe = read(
            "android/app/src/main/assets/chatgpt_web_stream_watchdog_probe.js",
        )
        assertTrue(baseAdapter.contains("if (fingerprint !== lastSnapshot)"))
        assertTrue(baseAdapter.indexOf("layoutAdapter.emitSnapshot") > baseAdapter.indexOf("emitEvent(event)"))
        assertTrue(baseAdapter.contains("privateStreamingSnapshotMode"))
        assertTrue(baseAdapter.contains("if (!forced && privateStreamingSnapshotMode) return"))
        assertTrue(baseAdapter.contains("privateStreamObserved: privateStreamingSnapshotMode"))
        assertTrue(baseAdapter.contains("privateStreamRevision += 1"))
        assertTrue(baseAdapter.contains("privateStreamObserved: privateStreamRevision > 0"))
        assertTrue(baseAdapter.contains("privateStreamState,"))
        assertTrue(baseAdapter.contains("streamWatchdogAcceptance.run"))
        val watchdogAcceptance = read(
            "android/app/src/main/assets/chatgpt_web_stream_watchdog_acceptance.js",
        )
        assertTrue(watchdogAcceptance.contains("probe.observePrivateUpdate('streaming')"))
        assertTrue(watchdogAcceptance.contains("{ privateStreamObserved: true }"))
        assertTrue(watchdogAcceptance.contains("probe.watchdogFired(schedule)"))
        assertTrue(watchdogProbe.contains("private_stream_watchdog_timeout"))
        assertTrue(watchdogProbe.contains("minimumStallMs"))
        assertTrue(mcp.contains("chatgpt_set_page_input_text"))
        assertTrue(mcp.contains("chatgpt_send_page_input"))
        assertTrue(mcp.contains("official_draft_length"))
        assertTrue(mcp.contains("adb_content_description"))
        assertTrue(mcp.contains("context_id"))
        assertTrue(mcp.contains("ChatGptWebMessageJson.encode"))
        assertTrue(messageJson.contains("message.content.take"))
        assertTrue(messageJson.contains("messagePartSelector"))
        assertTrue(mcp.contains("chatgpt_get_context"))
        assertTrue(background.contains("observedMcpState.accept(event)"))
        assertTrue(mcpBinding.contains("controlHandler(args)"))
        assertTrue(!mcpBinding.contains("private val control: (JSONObject)"))
    }

    @Test
    fun ordinaryFriendChatReusesTheCanonicalChatGptMcpEndpoint() {
        val port = read("android/app/src/main/kotlin/com/elon/app/WebChatSocialMcpPort.kt")
        val actions = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpActions.kt")
        val session = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt")
        val controller = read("android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt")
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val mainMcp = read("android/app/src/main/kotlin/com/elon/app/MainMcpNativeControlActions.kt")

        assertTrue(port.contains("interface WebChatSocialMcpPort"))
        assertTrue(actions.contains(": WebChatSocialMcpPort"))
        assertTrue(session.contains("fun createMcpPort("))
        assertTrue(session.contains("observedMcpState.accept(event)"))
        assertTrue(session.contains("onDocumentChanged = ::handleDocumentChanged"))
        assertTrue(session.contains("uiManifest = { latestUiManifest }"))
        assertTrue(controller.contains("override fun mcpPort(): WebChatSocialMcpPort = socialMcpPort"))
        assertTrue(controller.contains("revealMessage = ::revealMessageFromMcp"))
        assertTrue(controller.contains("requestChildRectangleOnScreen"))
        assertTrue(controller.contains("R.id.webChatMessageMore"))
        assertTrue(controller.contains("MAX_REVEAL_ATTEMPTS"))
        assertTrue(feature.contains("providerId() == WebChatProviderId.CHATGPT_WEB"))
        assertTrue(mainMcp.contains("action.startsWith(\"chatgpt_\")"))
        assertTrue(mainMcp.contains("return port.control(args)"))
        assertTrue(mainMcp.contains("\"chatgpt_web_mcp\""))
    }

    @Test
    fun ordinaryFriendChatOwnsARealMicrophonePermissionChain() {
        val main = read("android/app/src/main/kotlin/com/elon/app/MainActivity.kt")
        val lifecycle = read("android/app/src/main/kotlin/com/elon/app/MainChatGptWebLifecycle.kt")
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val controller = read("android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt")
        val session = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt")
        val backgroundWebView = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundWebViewFactory.kt",
        )

        assertTrue(main.contains("private val chatGptWebLifecycle = MainChatGptWebLifecycle(this)"))
        assertTrue(
            main.indexOf("private val chatGptWebLifecycle =") <
                main.indexOf("override fun onCreate("),
        )
        assertTrue(lifecycle.contains("ChatGptWebAudioPermissionController(activity)"))
        assertTrue(feature.contains("audioPermissionController = chatGptWebLifecycle.audioPermissionController"))
        assertTrue(feature.contains("chatGptWebLifecycle.dispose()"))
        assertTrue(controller.contains("audioPermissionController = audioPermissionController"))
        assertTrue(backgroundWebView.contains("override fun onPermissionRequest(request: PermissionRequest)"))
        assertTrue(backgroundWebView.contains("audioPermissionController.handle(request)"))
        assertTrue(backgroundWebView.contains("audioPermissionController.cancel(request)"))
        assertTrue(session.contains("audioPermissionState = audioPermissionController::snapshot"))
        assertTrue(session.contains("microphone_permission_denied"))
    }

    private fun read(relative: String): String =
        String(Files.readAllBytes(root().resolve(relative)), StandardCharsets.UTF_8)

    private fun root(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
