package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatConsumerRecoveryPolicyTest {
    @Test
    fun loginPageOffersGuestRetryWithoutPretendingTheSessionIsReady() {
        val state = WebChatConsumerRecoveryPolicy.resolve(
            WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            "login_required",
        )

        assertTrue(state.visible)
        assertTrue(state.retryVisible)
        assertTrue(state.officialVisible)
        assertEquals("访客", state.retryLabel)
        assertEquals("登录", state.officialLabel)
    }

    @Test
    fun healthySessionDoesNotShowRecoveryChrome() {
        assertFalse(WebChatConsumerRecoveryPolicy.resolve(
            WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            "ready",
        ).visible)
    }

    @Test
    fun productionRetryReturnsToOfficialHomeAndWorkModeClearsWebToolbar() {
        val session = read("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt")
        val controller = read("android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt")
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")

        assertTrue(session.contains("view.loadUrl(ChatGptWebNavigationPolicy.START_URL)"))
        assertTrue(controller.contains("override fun retryGuestAccess(): Boolean"))
        assertTrue(feature.contains("webChatState() != \"login_required\""))
        assertTrue(feature.contains("binding.moreButton.visibility = View.GONE"))
    }

    private fun read(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        var current = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        repeat(5) {
            if (Files.exists(current.resolve("android/app/src/main"))) return current
            current = current.parent ?: return@repeat
        }
        error("Repository root not found from ${System.getProperty("user.dir")}")
    }
}
