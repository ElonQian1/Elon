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
        assertTrue(presentation.contains("beginWebChatRealtimeVoiceInteraction()"))
        assertTrue(presentation.contains("isFocusableInTouchMode = true"))
        assertTrue(presentation.contains("endWebChatBackgroundInteraction()"))
        assertTrue(presentation.contains("const val MAX_LEASE_MS = 2_500L"))
        assertFalse(presentation.contains("View.GONE"))
        assertFalse(presentation.contains("pauseTimers"))
        assertTrue(chatGptWebView.contains("configureWebChatBackgroundSurface()"))
        assertTrue(chatGpt.contains("composerOptionInteraction::dispatch"))
        assertTrue(chatGpt.contains("backgroundInteractionLease::run"))
        assertTrue(chatGpt.contains("surfaceMode.isSkin() || realtimeVoiceBacking.isActive()"))
        val voiceBacking = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptRealtimeVoiceBackingController.kt",
        )
        assertTrue(voiceBacking.contains("view.beginWebChatRealtimeVoiceInteraction()"))
        assertTrue(voiceBacking.contains("if (surfaceMode.isSkin())"))
        assertTrue(voiceBacking.contains("surfaceMode.apply()"))
        assertTrue(composerInteraction.contains("backgroundLease.run(action)"))
        assertTrue(composerInteraction.contains("backgroundLease.release()"))
        assertTrue(google.contains("configureWebChatBackgroundSurface()"))
        val googlePause = google.substringAfter("private fun pauseSession()")
            .substringBefore("private fun resumeRecovery()")
        assertFalse(googlePause.contains("stopLoading()"))
        assertFalse(googlePause.contains("loadPendingAfterPause = true"))
        assertTrue(google.contains("WebChatBackgroundResumePolicy.decide("))
        assertFalse(chatGpt.contains("alpha = 0.01f"))
        assertFalse(google.contains("alpha = 0.01f"))
    }

    @Test
    fun providerSwitchKeepsInFlightProviderNavigationAlive() {
        val chatGpt = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val google = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )
        val chatGptPause = chatGpt.substringAfter("private fun pauseSession()")
            .substringBefore("private fun resumeRecovery()")
        val googlePause = google.substringAfter("private fun pauseSession()")
            .substringBefore("private fun resumeRecovery()")

        assertFalse(chatGptPause.contains("stopLoading()"))
        assertFalse(googlePause.contains("stopLoading()"))
        assertTrue(chatGpt.contains("if (view.progress >= 100)"))
        assertTrue(chatGpt.contains("recovery.onNavigationStarted()"))
        assertTrue(google.contains("WebChatBackgroundResumePolicy.decide("))
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
