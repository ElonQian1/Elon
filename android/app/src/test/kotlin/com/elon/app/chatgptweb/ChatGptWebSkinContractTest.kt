package com.elon.app.chatgptweb

import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebSkinContractTest {
    @Test
    fun skinUsesTheExistingOfficialWebViewAndKeepsBothFallbacks() {
        val session = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt")
        val portFactory = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPortFactory.kt")
        val surfaceMode = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSurfaceModeController.kt")
        val controller = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSkinPresentationController.kt")
        val pageAdapter = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt")
        val bridge = read("android/app/src/main/assets/chatgpt_web_adapter.js")
        val skin = read("android/app/src/main/assets/chatgpt_web_adapter_skin.js")
        val picker = read("android/app/src/main/kotlin/com/elon/app/WebChatProviderPickerSheet.kt")
        val mcpActions = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpActions.kt")

        assertTrue(surfaceMode.contains("showWebChatSkinSurface()"))
        assertTrue(surfaceMode.contains("showWebChatBackgroundSurface()"))
        assertTrue(session.contains("presentationMode = ::presentationMode"))
        assertTrue(portFactory.contains("mode = presentationMode"))
        assertTrue(session.contains("surfaceMode.isSkin() || realtimeVoiceBacking.isActive()"))
        assertTrue(session.contains("latestSnapshot.keepsBackgroundExecutionActive()"))
        assertTrue(session.contains("conversationNavigation.hasPending() || sendOwner.hasAttachmentSend()"))
        assertTrue(session.contains("{ webExecution.interactionRequested() }"))
        val executionInit = session.indexOf("private val webExecution:")
        val dictationInit = session.indexOf("private val privateDictation =")
        assertTrue(executionInit >= 0 && dictationInit > executionInit)
        assertTrue(session.contains("private val touchRequestHandler by lazy("))
        assertTrue(controller.contains("binding.chatList.visibility = View.GONE"))
        assertTrue(controller.contains("binding.inputLayout.visibility = View.GONE"))
        assertTrue(controller.contains("web-chat-skin-exit:chatgpt"))
        assertTrue(ChatGptWebAdapterAssets.names.contains("chatgpt_web_adapter_skin.js"))
        assertTrue(pageAdapter.contains("action = \"set_skin_mode\""))
        assertTrue(bridge.contains("action === 'set_skin_mode'"))
        assertTrue(bridge.contains("if (observer) observer.disconnect()"))
        assertTrue(bridge.contains("snapshotScheduler.cancelPending()"))
        assertTrue(bridge.contains("if (disposed || skinMode || !snapshotScheduler) return"))
        assertTrue(picker.contains("web-chat-provider-skin"))
        assertTrue(mcpActions.contains("\"skin\", \"web_skin\" -> ChatGptWebPresentationMode.SKIN"))
        assertTrue(skin.contains("https://chatgpt.com"))
        assertTrue(skin.contains("data-elon-chatgpt-skin"))
        listOf(pageAdapter, bridge, skin).forEach { source ->
            listOf("document.cookie", "getCookie(", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
                assertFalse("skin path must not access $it", source.contains(it))
            }
        }
    }

    private fun read(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)))

    private fun repositoryRoot(): Path {
        var current = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        repeat(6) {
            if (Files.isRegularFile(current.resolve("android/app/build.gradle"))) return current
            current = current.parent ?: return@repeat
        }
        error("Repository root not found")
    }
}
