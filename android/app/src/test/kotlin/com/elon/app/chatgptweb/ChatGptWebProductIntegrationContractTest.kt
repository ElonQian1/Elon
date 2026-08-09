package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProductIntegrationContractTest {
    @Test
    fun socialAiChatOffersTheChatGptWebProductMode() {
        val controller = read("android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt")
        val friendChat = read("android/app/src/main/kotlin/com/elon/app/MainFriendChatActions.kt")
        val main = read("android/app/src/main/kotlin/com/elon/app/MainActivity.kt")
        val activity = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt")
        val layout = read("android/app/src/main/res/layout/activity_chatgpt_web_test.xml")
        val strings = read("android/app/src/main/res/values/strings.xml")

        assertTrue(strings.contains("ChatGPT 网页"))
        assertTrue(controller.contains("ChatGptWebTestActivity.createProductIntent"))
        assertTrue(friendChat.contains("onActiveFriendChanged(friend)"))
        assertTrue(main.contains("openChatGptWeb = socialAiChatModeController::openChatGptWeb"))
        assertTrue(layout.contains("android:id=\"@+id/chatGptWebToolbar\""))
        assertTrue(activity.contains("binding.chatGptWebToolbar.visibility = View.GONE"))
    }

    @Test
    fun adaptiveMirrorAndMcpShareStableSemanticControlIds() {
        val layoutAdapter = read("android/app/src/main/assets/chatgpt_web_adapter_layout.js")
        val mcp = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpActions.kt")
        val activity = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt")
        val mcpBinding = read("android/app/src/main/kotlin/com/elon/app/mcp/McpNativeControlBinding.kt")

        assertTrue(layoutAdapter.contains("ui_manifest_snapshot"))
        assertTrue(layoutAdapter.contains("invoke_ui_control"))
        assertTrue(layoutAdapter.contains("compatibility:"))
        assertTrue(layoutAdapter.contains("addMessageControls"))
        assertTrue(layoutAdapter.contains("semantic === 'conversation'"))
        assertTrue(layoutAdapter.contains("contextId: resolvedContextId"))
        assertTrue(layoutAdapter.contains("scrollIntoView"))
        val baseAdapter = read("android/app/src/main/assets/chatgpt_web_adapter.js")
        assertTrue(baseAdapter.contains("if (fingerprint !== lastSnapshot)"))
        assertTrue(baseAdapter.indexOf("layoutAdapter.emitSnapshot") > baseAdapter.indexOf("emitEvent(event)"))
        assertTrue(mcp.contains("adb_content_description"))
        assertTrue(mcp.contains("context_id"))
        assertTrue(mcp.contains("message.content.take"))
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
