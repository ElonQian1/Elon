package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebSideMenuContractTest {
    @Test
    fun socialAiSidebarKeepsDateStripAndShowsDailyWebConversations() {
        val view = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt")
        val controller = read("android/app/src/main/kotlin/com/elon/app/ChatSideMenuController.kt")

        assertTrue(view.contains("createSocialSidebarDateStrip("))
        assertTrue(view.contains("ChatGptWebConversationIndex.activeOn(state.conversations, selectedDate)"))
        assertTrue(view.contains("val countLabel = \"\$activeCount 个活跃 · 共 \${state.conversations.size} 个会话\""))
        assertTrue(view.contains("renderProjects(this)"))
        assertTrue(view.contains("ChatGptNativeNavigationSelector::date"))
        assertTrue(controller.contains("applyChatSideMenuContentMode("))
        assertTrue(controller.contains("sideMenus.webChat.attach("))
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
