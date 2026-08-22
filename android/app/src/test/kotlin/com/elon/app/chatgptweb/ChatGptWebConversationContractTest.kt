package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebConversationContractTest {
    @Test
    fun newConversationOpensTheMobileSidebarBeforeResolvingItsVisibleEntry() {
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val core = readRepositoryFile("android/app/src/main/assets/chatgpt_web_adapter.js")
        val workflow = conversations.substringAfter("function newConversation")
            .substringBefore("function openConversation")

        assertTrue(workflow.contains("const existing = findNewConversationNode()"))
        assertTrue(workflow.contains("const open = findSidebarButton(true)"))
        assertTrue(workflow.contains("open.click()"))
        assertTrue(workflow.contains("waitForNewConversation("))
        assertTrue(workflow.contains("target.click()"))
        assertTrue(
            workflow.indexOf("const existing = findNewConversationNode()") <
                workflow.indexOf("const open = findSidebarButton(true)"),
        )
        assertTrue(core.contains("conversationAdapter.newConversation(respond)"))
        assertFalse(core.contains("document.querySelectorAll('a[href=\"/\"]"))
    }

    @Test
    fun newConversationPrefersTheCurrentOfficialStableControl() {
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val finder = conversations.substringAfter("function findNewConversationNode()")
            .substringBefore("function waitForNewConversation")

        assertTrue(finder.contains("[data-testid=\"create-new-chat-button\"]"))
        assertTrue(finder.contains("[data-testid=\"new-chat-button\"]"))
        assertTrue(finder.contains("stableControl && isVisible(stableControl)"))
        assertTrue(
            finder.indexOf("stableControl && isVisible(stableControl)") <
                finder.indexOf("/new chat|create chat|new conversation"),
        )
    }

    @Test
    fun conversationAdapterRemainsVisibleDomOnly() {
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val history = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversation_history.js",
        )

        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization")
            .forEach { forbidden ->
                assertFalse("conversation adapter must not contain $forbidden", conversations.contains(forbidden))
                assertFalse("conversation history must not contain $forbidden", history.contains(forbidden))
            }
    }

    @Test
    fun conversationAdapterEmitsDailyActivityAndProjectMembership() {
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val history = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversation_history.js",
        )

        assertTrue(conversations.contains("activityDates:"))
        assertTrue(conversations.contains("projectTitle:"))
        assertTrue(conversations.contains("function collectProjectHistory(initial, onDone)"))
        assertTrue(conversations.contains("timeoutMs: 10000"))
        assertTrue(conversations.contains("projectHints.merge(observedProjects, command && command.projectHints)"))
        assertTrue(conversations.contains("projectHints.missingTitles(observedTitles, values)"))
        assertTrue(conversations.contains("collectProjects(observedProjects, (projects) =>"))
        assertTrue(conversations.contains("projects,"))
        assertTrue(conversations.contains("function openProject"))
        assertTrue(conversations.contains("path.split('/').filter(Boolean).pop()"))
        assertTrue(history.contains("previous.activityDates"))
    }

    @Test
    fun cachedProjectHintsAvoidRepeatedProjectNavigation() {
        val pageAdapter = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )
        val background = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )

        assertTrue(pageAdapter.contains("projectHints.take(MAX_PROJECT_HINTS)"))
        assertTrue(pageAdapter.contains("chatgpt_web_adapter_project_hints.js"))
        assertTrue(background.contains("scopeProjectId = refreshRequest.scopeProjectId"))
        assertTrue(pageAdapter.contains("put(\"projectScopeId\", it)"))
        assertTrue(conversations.contains("scopeProjectId: scopeProjectId || null"))
    }

    @Test
    fun virtualHistoryTicksDoNotRepeatTheExpensiveProjectScan() {
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val reader = conversations.substringAfter("function readConversations()")
            .substringBefore("function findConversationScroller()")

        assertFalse(reader.contains("readProjects()"))
        assertTrue(reader.contains("projectIdFromPath(path)"))
        assertTrue(conversations.contains("enrichProjectConversations(snapshot.conversations, projects)"))
    }

    @Test
    fun slowMobileSidebarCanFinishAfterTheFirstAttemptTimesOut() {
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val backgroundSession = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )

        assertTrue(conversations.contains("Date.now() - started >= 10000"))
        assertTrue(conversations.contains("let sidebarOpenedByAdapter = false"))
        assertTrue(
            conversations.contains(
                "!existing.length && sidebarOpenedByAdapter && findSidebarButton(false)",
            ),
        )
        assertTrue(backgroundSession.contains("ChatGptConversationRefreshCoordinator("))
        assertTrue(backgroundSession.contains("conversationRefresh.onFailed()"))
        assertTrue(backgroundSession.contains("conversationRefresh.onSucceeded()"))
        assertTrue(backgroundSession.contains("conversationCollection.officialLoadState !="))
        assertTrue(backgroundSession.contains("ChatGptWebConversationCollection.LOAD_READY"))
        assertFalse(backgroundSession.contains("conversationListRequested"))
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
