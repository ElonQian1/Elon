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
        val dateStrip = read("android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuDateStrip.kt")
        val controller = read("android/app/src/main/kotlin/com/elon/app/ChatSideMenuController.kt")
        val webChatControl = read("android/app/src/main/kotlin/com/elon/app/ChatSideMenuWebChatControl.kt")
        val mcp = read("android/app/src/main/kotlin/com/elon/app/MainMcpNativeControlActions.kt")
        val navigationJson = read("android/app/src/main/kotlin/com/elon/app/WebChatNavigationJson.kt")

        assertTrue(view.contains("createSocialSidebarDateStrip("))
        assertTrue(view.contains("ChatGptWebConversationIndex.activeOn(state.conversations, selectedDate)"))
        assertTrue(view.contains("ChatGptWebConversationIndex.unassignedExcluding(state.conversations, active)"))
        assertTrue(view.contains("chatgpt_side_menu_daily_active"))
        assertTrue(view.contains("chatgpt_side_menu_unassigned"))
        assertTrue(view.contains("renderProjects(this)"))
        assertTrue(view.contains("ChatGptNativeNavigationSelector::date"))
        assertTrue(dateStrip.contains("isSelected = selected"))
        assertTrue(controller.contains("applyChatSideMenuContentMode("))
        assertTrue(controller.contains("sideMenus.webChat.attach("))
        assertTrue(controller.contains("ChatSideMenuWebChatControl("))
        assertTrue(webChatControl.contains("fun selectTab(tab: ChatGptWebSideMenuTab)"))
        assertTrue(webChatControl.contains("fun selectDate(date: LocalDate)"))
        assertTrue(view.contains("fun selectTab(tab: ChatGptWebSideMenuTab)"))
        assertTrue(view.contains("fun selectDate(date: LocalDate)"))
        assertTrue(mcp.contains("\"get_web_chat_navigation\""))
        assertTrue(mcp.contains("\"set_web_chat_sidebar\""))
        assertTrue(navigationJson.contains("elon.web_chat.navigation.v1"))
        assertTrue(mcp.contains("elon.web_chat.native_sidebar.v1"))
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
