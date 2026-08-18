package com.elon.app

import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatBackgroundWebViewContractTest {
    @Test
    fun providersShareANonRenderingBackgroundSurface() {
        val presentation = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/WebChatBackgroundWebViewPresentation.kt",
        )
        val chatGpt = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val chatGptWebView = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundWebViewFactory.kt",
        )
        val composerInteraction = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptComposerOptionInteraction.kt",
        )
        val google = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )

        assertTrue(presentation.contains("visibility = View.INVISIBLE"))
        assertTrue(presentation.contains("beginWebChatBackgroundInteraction()"))
        assertTrue(presentation.contains("endWebChatBackgroundInteraction()"))
        assertTrue(presentation.contains("const val MAX_LEASE_MS = 2_500L"))
        assertFalse(presentation.contains("View.GONE"))
        assertFalse(presentation.contains("pauseTimers"))
        assertTrue(chatGptWebView.contains("configureWebChatBackgroundSurface()"))
        assertTrue(chatGpt.contains("composerOptionInteraction::dispatch"))
        assertTrue(composerInteraction.contains("backgroundLease.run(action)"))
        assertTrue(composerInteraction.contains("backgroundLease.release()"))
        assertTrue(google.contains("configureWebChatBackgroundSurface()"))
        assertFalse(chatGpt.contains("alpha = 0.01f"))
        assertFalse(google.contains("alpha = 0.01f"))
    }

    private fun readRepositoryFile(relativePath: String): String =
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
