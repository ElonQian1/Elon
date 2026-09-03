package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPageAdapterExecutionContractTest {
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
        val posted = command.indexOf("webView.post {")
        val execute = command.indexOf("webView.evaluateJavascript(")
        assertTrue(resume >= 0)
        assertTrue(posted > resume)
        assertTrue(execute > posted)
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
