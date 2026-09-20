package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPageAdapterExecutionContractTest {
    @Test
    fun groupReusesTemporarySenderWithoutChangingPersonalDefaultsOrPersistingItsPrompt() {
        val adapter = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt")
        val group = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/GroupWebAiSession.kt")
        val executor = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/GroupWebAiExecutor.kt")
        assertTrue(adapter.contains("allowTemporaryTextDispatch: Boolean = false"))
        assertTrue(adapter.contains("!allowTemporaryTextDispatch"))
        assertTrue(adapter.contains("if (\$allowTemporaryTextDispatch) window.__elonChatGptFreshTextTemporaryEnabled = true"))
        assertTrue(group.contains("allowTemporaryTextDispatch = true"))
        assertTrue(executor.indexOf("if (!prepared") < executor.indexOf("authorize { permitted"))
        assertTrue(executor.contains("session?.isReady(it)"))
        assertTrue(executor.contains("sendPreparation?.close()"))
    }

    @Test
    fun commandsRunOnTheUiTurnAfterTheBackgroundWebViewIsResumed() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )
        val start = source.indexOf("private fun runCommand(")
        val end = source.indexOf("private fun isAllowedOrigin", start)
        assertTrue(start >= 0 && end > start)
        val command = source.substring(start, end)

        val resume = command.indexOf("onWebExecutionRequested()")
        val delivery = command.indexOf("commandDelivery.send(command)")
        assertTrue(resume >= 0)
        assertTrue(delivery > resume)
        val invokeStart = source.indexOf("invoke = { command, owner, allowed, result ->")
        val invokeEnd = source.indexOf("repair =", invokeStart)
        assertTrue(invokeStart >= 0 && invokeEnd > invokeStart)
        val invoke = source.substring(invokeStart, invokeEnd)
        val posted = invoke.indexOf("webView.post {")
        val execute = invoke.indexOf("webView.evaluateJavascript(")
        assertTrue(posted >= 0 && execute > posted)
        assertTrue(invoke.contains("if (!allowed())"))
        assertTrue(command.contains("if (!listenerInstalled ||"))
        assertTrue(command.contains("ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)"))
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
