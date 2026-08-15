package com.elon.app

import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.charset.StandardCharsets
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionSurfaceBoundaryContractTest {
    @Test
    fun productionSurfaceNeverDependsOnTheDiagnosticActivityOrItsViews() {
        val forbidden = listOf(
            "ChatGptWebTestActivity",
            "ActivityChatgptWebTestBinding",
            "activity_chatgpt_web_test",
            "chatGptNative",
            "chatGptWebTest",
        )

        productionSources.forEach { path ->
            val source = read(path)
            forbidden.forEach { token ->
                assertFalse("$path must not reference diagnostic token $token", source.contains(token))
            }
        }
    }

    @Test
    fun productionFeaturesAreWiredThroughTheFriendChatSurface() {
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val composer = read("android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt")

        assertTrue(feature.contains("ChatGptSocialChatController"))
        assertTrue(feature.contains("GoogleWebSocialChatController"))
        assertTrue(feature.contains("WebChatProductionComposerToolsCoordinator"))
        assertTrue(composer.contains("webToolsButton"))
        assertTrue(composer.contains("web-chat-composer-tools:unavailable"))
    }

    @Test
    fun diagnosticActivityKeepsItsOwnExplicitTestBinding() {
        val diagnostic = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt",
        )

        assertTrue(diagnostic.contains("class ChatGptWebTestActivity"))
        assertTrue(diagnostic.contains("ActivityChatgptWebTestBinding"))
        assertTrue(diagnostic.contains("binding.chatGptNativeComposer"))
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

    private companion object {
        val productionSources = listOf(
            "android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt",
            "android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt",
            "android/app/src/main/kotlin/com/elon/app/ChatAdapter.kt",
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
            "android/app/src/main/kotlin/com/elon/app/GoogleWebSocialChatController.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionMessageActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionRichContent.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerCommands.kt",
        )
    }
}
