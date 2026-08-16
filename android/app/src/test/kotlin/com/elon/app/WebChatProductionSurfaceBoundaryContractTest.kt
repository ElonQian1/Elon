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
        val sendVisual = read("android/app/src/main/kotlin/com/elon/app/MainSendButtonVisualActions.kt")
        val consumerComposer = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatConsumerComposerState.kt",
        )

        assertTrue(feature.contains("ChatGptSocialChatController"))
        assertTrue(feature.contains("GoogleWebSocialChatController"))
        assertTrue(feature.contains("WebChatProductionComposerToolsCoordinator"))
        assertTrue(feature.contains("WebChatProductionFeatureNavigationCoordinator"))
        assertTrue(feature.contains("WebChatProductionPageActionsCoordinator"))
        assertTrue(feature.contains("openFeatureNavigation = ::openProductionFeatureNavigation"))
        assertTrue(feature.contains("WebChatProductionSelectors.pageActions"))
        assertTrue(feature.contains("views.attachmentButton.visibility"))
        assertTrue(feature.contains("WebChatConsumerComposerStateResolver.resolve"))
        assertTrue(consumerComposer.contains("WebChatProviderCapability.ATTACHMENT_UPLOAD"))
        assertTrue(feature.contains("WebChatProductionSelectors.composerInput"))
        assertTrue(feature.contains("WebChatProductionSelectors.attachment"))
        assertTrue(composer.contains("webToolsButton"))
        assertTrue(composer.contains("web-chat-composer-tools:unavailable"))
        assertTrue(sendVisual.contains("WebChatProductionSelectors.composerAction"))
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
            "android/app/src/main/kotlin/com/elon/app/MainSendButtonVisualActions.kt",
            "android/app/src/main/kotlin/com/elon/app/ChatAdapter.kt",
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
            "android/app/src/main/kotlin/com/elon/app/GoogleWebSocialChatController.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionMessageActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionRichContent.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerCommands.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerVisualMode.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureNavigation.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureCompletion.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionPageActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionAdaptiveControls.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionSelectors.kt",
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuCoordinator.kt",
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt",
        )
    }
}
