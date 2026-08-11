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
