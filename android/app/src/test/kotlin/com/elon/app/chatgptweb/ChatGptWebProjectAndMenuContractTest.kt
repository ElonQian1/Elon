package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProjectAndMenuContractTest {
    @Test
    fun projectAndContextMenuPoliciesLoadBeforeTheirConsumers() {
        val adapter = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt")
        val bootstrap = read("android/app/src/main/assets/chatgpt_web_adapter_bootstrap.js")

        assertTrue(
            adapter.indexOf("chatgpt_web_adapter_project_policy.js") <
                adapter.indexOf("chatgpt_web_adapter_conversations.js"),
        )
        assertTrue(
            adapter.indexOf("chatgpt_web_adapter_context_menu_policy.js") <
                adapter.indexOf("chatgpt_web_adapter_layout.js"),
        )
        assertTrue(bootstrap.contains("'__elonChatGptProjectPolicy'"))
        assertTrue(bootstrap.contains("'__elonChatGptContextMenuPolicy'"))
    }

    @Test
    fun projectDiscoveryStaysVisibleDomOnlyAndMenuObservationIsBounded() {
        val project = read("android/app/src/main/assets/chatgpt_web_adapter_project_policy.js")
        val conversations = read("android/app/src/main/assets/chatgpt_web_adapter_conversations.js")
        val menu = read("android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js")
        val invocation = read(
            "android/app/src/main/assets/chatgpt_web_adapter_context_menu_invocation.js",
        )
        val layout = read("android/app/src/main/assets/chatgpt_web_adapter_layout.js")

        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization")
            .forEach { forbidden -> assertFalse(project.contains(forbidden)) }
        assertTrue(project.contains("[data-project-id]"))
        assertTrue(project.contains("canonicalPath"))
        assertTrue(project.contains("runtimeProjectId"))
        assertTrue(project.contains("Object.getOwnPropertyDescriptor"))
        assertFalse(project.contains("descriptor.get"))
        assertTrue(conversations.contains("projectPolicy.read"))
        assertTrue(conversations.contains("projectPolicy.findNode"))
        assertTrue(conversations.contains("collectProjects"))
        assertTrue(conversations.contains("history.back()"))
        assertTrue(conversations.contains("enrichProjectConversations"))
        assertTrue(menu.contains("hasNewRoot"))
        assertTrue(menu.contains("function observe(onOpened, onTimedOut)"))
        assertTrue(menu.contains("observe.isOpen = opened"))
        assertTrue(menu.contains("elapsed >= timeout"))
        assertFalse(menu.contains("retry();"))
        assertTrue(invocation.contains("function createCoordinator()"))
        assertTrue(invocation.contains("candidate.observation.isOpen()"))
        assertTrue(invocation.contains("if (!candidate.retried)"))
        assertTrue(layout.contains("overlayPolicy.contextMenuSignature"))
        assertTrue(layout.contains("overlays.forEach"))
        assertTrue(layout.contains("contextMenuPolicy.prepare"))
        assertTrue(layout.contains("contextMenuInvocation?.reconcile()"))
        assertTrue(layout.contains("contextMenuInvocation.start"))
        assertTrue(layout.contains("官网会话设置未打开，请重试。"))
    }

    private fun read(relativePath: String): String =
        String(Files.readAllBytes(root().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun root(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
