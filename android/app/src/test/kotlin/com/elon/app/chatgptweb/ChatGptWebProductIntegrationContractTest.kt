package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProductIntegrationContractTest {
    @Test
    fun socialAiChatKeepsWorkModeAndAddsNativeWebChatProviders() {
        val controller = read("android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt")
        val chatController = read("android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt")
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val provider = read("android/app/src/main/kotlin/com/elon/app/WebChatProvider.kt")
        val friendChat = read("android/app/src/main/kotlin/com/elon/app/MainFriendChatActions.kt")
        val main = read("android/app/src/main/kotlin/com/elon/app/MainActivity.kt")
        val mainMcp = read("android/app/src/main/kotlin/com/elon/app/MainMcpNativeControlActions.kt")
        val activity = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt")
        val layout = read("android/app/src/main/res/layout/activity_chatgpt_web_test.xml")
        val strings = read("android/app/src/main/res/values/strings.xml")

        assertTrue(strings.contains("ChatGPT 网页 AI"))
        assertTrue(strings.contains("工作模式"))
        assertTrue(strings.contains("聊天模式"))
        assertTrue(controller.contains("SocialAiInteractionMode.WORK"))
        assertTrue(controller.contains("SocialAiInteractionMode.CHAT"))
        assertTrue(controller.contains("social_ai_mode_chat_provider"))
        assertTrue(!controller.contains("showProviderSelector"))
        assertTrue(controller.contains("openOfficialFallback"))
        assertTrue(provider.contains("GOOGLE_WEB"))
        assertTrue(provider.contains("available = false"))
        assertTrue(chatController.contains("ChatGptFriendMessageMapper.map"))
        assertTrue(chatController.contains("session.sendAttachments"))
        assertTrue(chatController.contains("chatAttachmentsFromPending"))
        assertTrue(friendChat.contains("onActiveFriendChanged(friend)"))
        assertTrue(friendChat.contains("suspendForExternalChat"))
        assertTrue(main.contains("socialAiChatFeature.trySendMessage"))
        assertTrue(feature.contains("activateWorkMode"))
        assertTrue(feature.contains("activateChatProvider"))
        assertTrue(feature.contains("deactivateChatProvider"))
        assertTrue(feature.contains("WEB_CHAT_MODEL_BUTTON_OWNER"))
        assertTrue(mainMcp.contains("set_social_ai_interaction_mode"))
        assertTrue(mainMcp.contains("select_web_chat_provider"))
        assertTrue(mainMcp.contains("open_chatgpt_official_fallback"))
        assertTrue(mainMcp.contains("web_chat_attachment_phase"))
        assertTrue(layout.contains("android:id=\"@+id/chatGptWebToolbar\""))
        assertTrue(activity.contains("binding.chatGptWebToolbar.visibility = View.GONE"))
        assertTrue(activity.contains("ChatGptWebAccessPolicy.canChat(snapshot)"))
    }

    @Test
    fun adaptiveMirrorAndMcpShareStableSemanticControlIds() {
        val layoutAdapter = read("android/app/src/main/assets/chatgpt_web_adapter_layout.js")
        val mcp = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpActions.kt")
        val messageJson = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMessageJson.kt")
        val activity = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt")
        val mcpBinding = read("android/app/src/main/kotlin/com/elon/app/mcp/McpNativeControlBinding.kt")

        assertTrue(layoutAdapter.contains("ui_manifest_snapshot"))
        assertTrue(layoutAdapter.contains("invoke_ui_control"))
        assertTrue(layoutAdapter.contains("compatibility:"))
        assertTrue(layoutAdapter.contains("addMessageControls"))
        assertTrue(layoutAdapter.contains("return 'conversation';"))
        assertTrue(layoutAdapter.contains("pageSemantic === 'conversation_options'"))
        assertTrue(layoutAdapter.contains("return 'conversation_options';"))
        assertTrue(layoutAdapter.contains("contextId: resolvedContextId"))
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

    private fun read(relative: String): String =
        String(Files.readAllBytes(root().resolve(relative)), StandardCharsets.UTF_8)

    private fun root(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
