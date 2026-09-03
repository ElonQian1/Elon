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
        assertTrue(
            adapter.indexOf("chatgpt_web_adapter_control_labels.js") <
                adapter.indexOf("chatgpt_web_adapter_layout.js"),
        )
        assertTrue(
            adapter.indexOf("chatgpt_web_adapter_project_choice_reveal.js") <
                adapter.indexOf("chatgpt_web_adapter_layout.js"),
        )
        assertTrue(bootstrap.contains("'__elonChatGptProjectPolicy'"))
        assertTrue(bootstrap.contains("'__elonChatGptContextMenuPolicy'"))
        assertTrue(bootstrap.contains("'__elonChatGptControlLabels'"))
        assertTrue(bootstrap.contains("'__elonChatGptProjectChoiceReveal'"))
    }

    @Test
    fun projectDiscoveryAndScopedContextActivationStayDomOnly() {
        val project = read("android/app/src/main/assets/chatgpt_web_adapter_project_policy.js")
        val conversations = read("android/app/src/main/assets/chatgpt_web_adapter_conversations.js")
        val menu = read("android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js")
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
        assertTrue(menu.contains("function activate(control, node)"))
        assertTrue(menu.contains("node.click()"))
        assertTrue(menu.contains("return false"))
        assertTrue(layout.contains("overlayPolicy.contextMenuSignature"))
        assertTrue(layout.contains("overlays.forEach"))
        assertTrue(layout.contains("contextMenuPolicy.activate(control, node)"))
        assertTrue(menu.contains("requiresNativeTouch(control)"))
        assertTrue(menu.contains("if (requiresNativeTouch(control) || !isProjectMoveStep(control)"))
    }

    @Test
    fun webCommandsRemainBoundToTheObservedDocumentGeneration() {
        val adapter = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt")

        assertTrue(adapter.contains(".put(\"documentToken\", documentSession.snapshot().documentToken)"))
        assertTrue(adapter.contains("window.__elonChatGptBridge.command(\$encoded)"))
        assertFalse(adapter.contains("command.documentToken=window.__elonChatGptDocumentToken"))
    }

    @Test
    fun explicitMessageActionsWinOverTheOwningProjectRoute() {
        val layout = read("android/app/src/main/assets/chatgpt_web_adapter_layout.js")
        val readAloud = layout.indexOf("if (/read.aloud|朗读/.test(signal)) return 'read_aloud';")
        val project = layout.indexOf("if (/project|项目/.test(signal + ' ' + path)) return 'project';")

        assertTrue(readAloud >= 0)
        assertTrue(project >= 0)
        assertTrue(readAloud < project)
    }

    @Test
    fun projectChoiceRevealIsReadOnlyAndWiredThroughTheExistingCommandPort() {
        val reveal = read(
            "android/app/src/main/assets/chatgpt_web_adapter_project_choice_reveal.js",
        )
        val adapter = read("android/app/src/main/assets/chatgpt_web_adapter.js")
        val actions = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpActions.kt",
        )

        assertTrue(reveal.contains("scrollPositions(container)"))
        assertTrue(reveal.contains("project_choice_revealed"))
        assertFalse(reveal.contains(".click()"))
        assertFalse(reveal.contains("fetch("))
        assertTrue(adapter.contains("action === 'reveal_project_choice'"))
        assertTrue(actions.contains("\"chatgpt_reveal_project_choice\""))
        assertTrue(actions.contains("commands.revealProjectChoice(title, requestId)"))
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
