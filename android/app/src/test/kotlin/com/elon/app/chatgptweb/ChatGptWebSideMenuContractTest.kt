package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertEquals
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
        assertTrue(view.contains("ChatGptWebConversationIndex.unassigned(state.conversations)"))
        assertTrue(view.contains("chatgpt_side_menu_daily_active"))
        assertTrue(view.contains("chatgpt_side_menu_unassigned"))
        assertTrue(view.contains("renderProjects(this)"))
        assertTrue(view.contains("renderProjectConversations(this, it)"))
        assertTrue(view.contains("selectedProjectId = project.id"))
        assertTrue(view.contains("ChatGptNativeNavigationSelector.projectBack(project)"))
        assertTrue(view.contains("if (localProjectActions() == null) post { openProject(project.path) }"))
        assertTrue(view.contains("setOnClickListener { closeThen { openConversation(conversation.path) } }"))
        assertTrue(!view.contains("closeThen { openProject(project.path) }"))
        assertTrue(view.contains("conversationActions.button(conversation)"))
        assertTrue(view.contains("conversationActions.show(conversation)"))
        val actions = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuConversationActions.kt",
        )
        assertTrue(actions.contains("ChatGptNativeNavigationSelector.conversationActions(conversation)"))
        assertTrue(actions.contains("LinearLayout.LayoutParams(dp(48)"))
        assertTrue(view.contains("ChatGptNativeNavigationSelector::date"))
        assertTrue(view.contains("WebChatSideMenuContentState.resolve("))
        assertTrue(view.contains("state.projectCollections[project.id]"))
        assertTrue(view.contains("WebChatSideMenuStateViews.create("))
        assertTrue(view.contains("onRetry = ::requestIndexRefresh"))
        val stateView = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/WebChatSideMenuStateViews.kt",
        )
        assertTrue(stateView.contains("ChatGptNativeNavigationSelector.RETRY_CONVERSATIONS"))
        assertTrue(stateView.contains("LinearLayout.LayoutParams(dp(128), dp(48))"))
        assertTrue(stateView.contains("读取失败 · 可重新加载"))
        assertTrue(dateStrip.contains("isSelected = selected"))
        assertTrue(controller.contains("applyChatSideMenuContentMode("))
        assertTrue(controller.contains("sideMenus.webChat.attach("))
        assertTrue(controller.contains("ChatSideMenuWebChatControl("))
        assertTrue(webChatControl.contains("fun selectTab(tab: ChatGptWebSideMenuTab)"))
        assertTrue(webChatControl.contains("fun selectDate(date: LocalDate)"))
        assertTrue(webChatControl.contains("fun selectProject(projectId: String)"))
        assertTrue(view.contains("fun selectTab(tab: ChatGptWebSideMenuTab)"))
        assertTrue(view.contains("fun selectDate(date: LocalDate)"))
        assertTrue(mcp.contains("\"get_web_chat_navigation\""))
        assertTrue(mcp.contains("\"set_web_chat_sidebar\""))
        assertTrue(navigationJson.contains("elon.web_chat.navigation.v1"))
        assertTrue(mcp.contains("elon.web_chat.native_sidebar.v1"))
        assertTrue(mcp.contains("selected_project_id"))
        assertTrue(mcp.contains("selectWebChatProject(requestedProjectId)"))
    }

    @Test
    fun emptyIndexDistinguishesLoadingFailureAndConfirmedEmptyContent() {
        assertEquals(WebChatSideMenuContentStatus.LOADING, resolve(
            collection = ChatGptWebConversationCollection(),
        ))
        assertEquals(WebChatSideMenuContentStatus.LOADING, resolve(
            collection = collection(ChatGptWebConversationCollection.LOAD_LOADING),
        ))
        assertEquals(WebChatSideMenuContentStatus.FAILED, resolve(
            collection = collection(ChatGptWebConversationCollection.LOAD_FAILED),
        ))
        assertEquals(WebChatSideMenuContentStatus.EMPTY, resolve(
            collection = collection(ChatGptWebConversationCollection.LOAD_READY),
        ))
    }

    @Test
    fun cachedRowsStayVisibleWhileRefreshIsLoadingOrFailed() {
        assertEquals(WebChatSideMenuContentStatus.CONTENT, resolve(
            collection = collection(ChatGptWebConversationCollection.LOAD_LOADING),
            availableCount = 4,
            visibleCount = 2,
        ))
        assertEquals(WebChatSideMenuContentStatus.CONTENT, resolve(
            collection = collection(ChatGptWebConversationCollection.LOAD_FAILED),
            availableCount = 4,
            visibleCount = 2,
        ))
        assertEquals(WebChatSideMenuContentStatus.EMPTY, resolve(
            collection = collection(ChatGptWebConversationCollection.LOAD_LOADING),
            availableCount = 4,
        ))
    }

    private fun resolve(
        collection: ChatGptWebConversationCollection,
        availableCount: Int = 0,
        visibleCount: Int = 0,
    ): WebChatSideMenuContentStatus = WebChatSideMenuContentState.resolve(
        collection = collection,
        availableCount = availableCount,
        visibleCount = visibleCount,
    )

    private fun collection(loadState: String) = ChatGptWebConversationCollection(
        source = ChatGptWebConversationCollection.SOURCE_OFFICIAL,
        officialLoadState = loadState,
    )

    private fun read(relative: String): String =
        String(Files.readAllBytes(root().resolve(relative)), StandardCharsets.UTF_8)

    private fun root(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
