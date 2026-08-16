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
        val activity = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt")
        val layout = read("android/app/src/main/res/layout/activity_chatgpt_web_test.xml")
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
        assertTrue(layout.contains("android:id=\"@+id/chatGptWebToolbar\""))
        assertTrue(activity.contains("binding.chatGptWebToolbar.visibility = View.GONE"))
        assertTrue(activity.contains("modeController.select(ChatGptWebModeController.Mode.WEB)"))
        assertTrue(activity.contains("ChatGptWebAccessPolicy.canChat(snapshot)"))
        assertTrue(chatController.contains("WebChatSendFallbackPolicy.decide"))
        assertTrue(chatController.contains("session.onHostResumed()"))
        assertTrue(googleController.contains("WebChatSendFallbackPolicy.decide(loginRequired = false)"))
        assertTrue(googleController.contains("session.onHostResumed()"))
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
    fun conversationRowsDoNotPresentPinnedHeadingsAsPerConversationState() {
        val adapter = read(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val sideMenu = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt",
        )

        assertFalse(adapter.substringBefore("function cleanText").contains("pinned", ignoreCase = true))
        assertFalse(adapter.substringBefore("function cleanText").contains("已置顶"))
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
        val activity = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt")
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
        assertTrue(layoutAdapter.contains("scrollIntoView"))
        val baseAdapter = read("android/app/src/main/assets/chatgpt_web_adapter.js")
        assertTrue(baseAdapter.contains("if (fingerprint !== lastSnapshot)"))
        assertTrue(baseAdapter.indexOf("layoutAdapter.emitSnapshot") > baseAdapter.indexOf("emitEvent(event)"))
        assertTrue(mcp.contains("adb_content_description"))
        assertTrue(mcp.contains("context_id"))
        assertTrue(mcp.contains("ChatGptWebMessageJson.encode"))
        assertTrue(messageJson.contains("message.content.take"))
        assertTrue(messageJson.contains("messagePartSelector"))
        assertTrue(mcp.contains("chatgpt_get_context"))
        assertTrue(activity.contains("mcpNativeControlBinding.register()"))
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

        assertTrue(main.contains("private val chatGptWebLifecycle = MainChatGptWebLifecycle(this)"))
        assertTrue(
            main.indexOf("private val chatGptWebLifecycle =") <
                main.indexOf("override fun onCreate("),
        )
        assertTrue(lifecycle.contains("ChatGptWebAudioPermissionController(activity)"))
        assertTrue(feature.contains("audioPermissionController = chatGptWebLifecycle.audioPermissionController"))
        assertTrue(feature.contains("chatGptWebLifecycle.dispose()"))
        assertTrue(controller.contains("audioPermissionController = audioPermissionController"))
        assertTrue(session.contains("override fun onPermissionRequest(request: PermissionRequest)"))
        assertTrue(session.contains("audioPermissionController.handle(request)"))
        assertTrue(session.contains("audioPermissionController.cancel(request)"))
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
