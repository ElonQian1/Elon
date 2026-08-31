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
        val modeController = read("android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt")
        val composer = read("android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt")
        val attachmentMenu = read(
            "android/app/src/main/kotlin/com/elon/app/MainAttachmentPanelActions.kt",
        )
        val sendVisual = read("android/app/src/main/kotlin/com/elon/app/MainSendButtonVisualActions.kt")
        val consumerComposer = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatConsumerComposerState.kt",
        )
        val consumerStatusBanner = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatConsumerStatusBanner.kt",
        )
        val composerTools = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt",
        )
        val chatGptBackground = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val chatGptNavigationActions = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptSessionNavigationActions.kt",
        )
        val chatGptTouchRequests = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTouchRequestHandler.kt",
        )

        assertTrue(feature.contains("ChatGptSocialChatController"))
        assertFalse(modeController.contains("ChatGptWebTestActivity"))
        assertTrue(modeController.contains("ChatGptWebOfficialFallbackIntent"))
        assertTrue(feature.contains("GoogleWebSocialChatController"))
        assertTrue(feature.contains("WebChatProductionComposerToolsCoordinator"))
        assertTrue(feature.contains("WebChatProductionFeatureNavigationCoordinator"))
        assertTrue(feature.contains("WebChatProductionPageActionsCoordinator"))
        assertTrue(feature.contains("WebChatProductionHeaderActionsCoordinator"))
        assertTrue(feature.contains("productionHeaderActions.render"))
        assertTrue(feature.contains("WebChatProductionConversationActionsCoordinator"))
        assertTrue(feature.contains("WebChatProductionSuggestionsCoordinator"))
        assertTrue(feature.contains("productionSuggestions.render(provider, controller.consumerPort())"))
        assertTrue(feature.contains("openRemoteConversationActions = productionConversationActions::show"))
        assertTrue(feature.contains("openFeatureNavigation = ::openProductionFeatureNavigation"))
        assertTrue(feature.contains("WebChatProductionSelectors.pageActions"))
        assertTrue(feature.contains("views.attachmentButton.visibility"))
        assertTrue(feature.contains("WebChatConsumerComposerStateResolver.resolve"))
        assertTrue(feature.contains("WebChatConsumerStatusBanner"))
        assertTrue(feature.contains("controller.consumerRecoveryState(provider)"))
        assertTrue(consumerStatusBanner.contains("WebChatConsumerRecoveryPolicy.resolve"))
        val sideMenu = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt",
        )
        assertTrue(sideMenu.contains("WebChatSideMenuRefreshPolicy.shouldRefreshOnOpen"))
        assertTrue(sideMenu.contains("ChatGptNativeNavigationSelector.REFRESH_CONVERSATIONS"))
        assertTrue(chatGptBackground.contains("ChatGptConversationNavigationCoordinator"))
        assertTrue(chatGptBackground.contains("observedMcpState::beginOpenConversationCommand"))
        assertTrue(chatGptNavigationActions.contains("conversationNavigation.beginOpen"))
        assertTrue(chatGptNavigationActions.contains("conversationNavigation.beginNew"))
        assertTrue(chatGptBackground.contains("conversationNavigation.save"))
        assertTrue(chatGptTouchRequests.contains("ChatGptWebInteractionTimings"))
        assertTrue(consumerComposer.contains("WebChatProviderCapability.ATTACHMENT_UPLOAD"))
        assertTrue(feature.contains("WebChatProductionSelectors.composerInput"))
        assertTrue(feature.contains("WebChatProductionSelectors.attachment"))
        assertTrue(composer.contains("webToolsButton"))
        assertTrue(composer.contains("web-chat-composer-tools:not-bound"))
        assertTrue(attachmentMenu.contains("attachment-action-camera"))
        assertTrue(attachmentMenu.contains("attachment-action-photos"))
        assertTrue(attachmentMenu.contains("attachment-action-files"))
        assertFalse(attachmentMenu.contains("web-chat-quick-action:"))
        assertTrue(composerTools.contains("title = \"工具\""))
        assertTrue(composerTools.contains("quick:${'$'}{it.semantic}"))
        assertTrue(feature.contains("productionComposerTools.show(provider)"))
        assertTrue(feature.contains("views.activeWebToolChip.render"))
        assertTrue(feature.contains("WebChatProductionComposerContext.inputHint"))
        assertTrue(feature.contains("productionComposerTools.quickActions(provider).isEmpty()"))
        assertTrue(sendVisual.contains("WebChatProductionSelectors.composerAction"))
    }

    @Test
    fun activeProviderReentryKeepsTheWarmIdentityTransport() {
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val activation = feature.substringAfter("private fun activateChatProvider")
            .substringBefore("private fun renderToolbarVoiceAction")
        val controllerIndex = activation.indexOf("val controller = controllerFor(provider.id)")
        val activeGuardIndex = activation.indexOf("if (!controller.isActive())")
        val deactivateIndex = activation.indexOf("chatGptController.deactivate()")
        val activateIndex = activation.indexOf("controller.activate(provider)")
        val presentationIndex = activation.indexOf("ensureConsumerEnhancementsAttached()")

        assertTrue(controllerIndex >= 0)
        assertTrue(activeGuardIndex > controllerIndex)
        assertTrue(deactivateIndex > activeGuardIndex)
        assertTrue(activateIndex > deactivateIndex)
        assertTrue(presentationIndex > activateIndex)
    }

    @Test
    fun officialFallbackIsSeparateAndTheDiagnosticSurfaceIsGone() {
        val official = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebOfficialActivity.kt",
        )
        val root = repositoryRoot()

        assertTrue(official.contains("class ChatGptWebOfficialActivity"))
        assertFalse(official.contains("McpNativeControlBinding"))
        assertFalse(official.contains("ChatGptWebPageAdapter"))
        assertFalse(
            Files.exists(
                root.resolve(
                    "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt",
                ),
            ),
        )
        assertFalse(Files.exists(root.resolve("android/app/src/main/res/layout/activity_chatgpt_web_test.xml")))
    }

    @Test
    fun productionMessageActionsUseTheTypedConsumerPortWithoutAJsonRoundTrip() {
        val actions = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionMessageActions.kt",
        )
        val controller = read(
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
        )

        assertTrue(actions.contains("WebChatConsumerPort"))
        assertTrue(actions.contains("executeSessionCommand"))
        assertTrue(actions.contains("invokeControl"))
        assertFalse(actions.contains("WebChatSocialMcpPort"))
        assertFalse(actions.contains("JSONObject"))
        assertFalse(actions.contains("MessageActionJson"))
        assertTrue(controller.contains("socialConsumerPort.state().controls"))
        assertFalse(controller.contains("socialMcpPort.uiState()"))
    }

    @Test
    fun webProvidersShareOneProductionTranscriptInsteadOfDuplicatingListState() {
        val chatGpt = read(
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
        )
        val google = read(
            "android/app/src/main/kotlin/com/elon/app/GoogleWebSocialChatController.kt",
        )

        listOf(chatGpt, google).forEach { controller ->
            assertTrue(controller.contains("WebChatProductionTranscript("))
            assertFalse(controller.contains("mutableListOf<ChatMessage>()"))
            assertFalse(controller.contains("linkedMapOf<String, Long>()"))
            assertFalse(controller.contains("WebChatProductionMessageListUpdater("))
            assertTrue(controller.contains("transcript.activate()"))
            assertTrue(controller.contains("transcript.submit(presented, active)"))
        }
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
            "android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt",
            "android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt",
            "android/app/src/main/kotlin/com/elon/app/MainAttachmentPanelActions.kt",
            "android/app/src/main/kotlin/com/elon/app/MainSendButtonVisualActions.kt",
            "android/app/src/main/kotlin/com/elon/app/ChatAdapter.kt",
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
            "android/app/src/main/kotlin/com/elon/app/GoogleWebSocialChatController.kt",
            "android/app/src/main/kotlin/com/elon/app/ProfileChatGptWebEntry.kt",
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionMessageActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionRichContent.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionActiveToolChip.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerContext.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerCommands.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerVisualMode.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureNavigation.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureCompletion.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionHeaderActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionPageActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionPageActionPolicy.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionConversationActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionSuggestions.kt",
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuConversationActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionAdaptiveControls.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionSelectors.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatConsumerStatusBanner.kt",
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuCoordinator.kt",
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt",
        )
    }
}
