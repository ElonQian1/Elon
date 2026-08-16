package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatConsumerRetryContractTest {
    @Test
    fun retryReloadsBothProviderSessionsWithoutClearingCookiesOrData() {
        val chatGpt = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val google = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")

        listOf(chatGpt, google).forEach { source ->
            assertTrue(source.contains("fun retryConnection(): Boolean"))
            assertTrue(source.contains("view.reload()"))
            assertTrue(source.contains("updateState(State.LOADING)"))
        }
        assertTrue(feature.contains("controller.retryGuestAccess()"))
        assertTrue(feature.contains("controller.retryConnection()"))
        assertTrue(feature.contains("if (!retried) controller.onHostResumed()"))
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
