package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionInteractionLatencyContractTest {
    @Test
    fun primaryWebChatMenusRenderBeforeTheirSilentOfficialRefresh() {
        val tools = read("WebChatProductionComposerTools.kt")
        val features = read("WebChatProductionFeatureNavigation.kt")
        val actions = read("WebChatProductionPageActions.kt")
        val model = read("ChatGptSocialChatController.kt")
        val conversationActions = read("WebChatProductionConversationActions.kt")

        assertTrue(tools.contains("WebChatProductionQuickComposerActionCatalog.availableFor"))
        assertBefore(features, "showFeatureDialog(", "port.requestFeatures()")
        assertBefore(actions, "val actions = readActions(", "port.requestControls()")
        assertBefore(model, "presentModelOptions(", "session.requestModelOptions()")
        assertBefore(
            features,
            "if (!interactionCache.needsFeatureRefresh(provider.id)) return",
            "port.requestFeatures()",
        )
        assertBefore(
            model,
            "interactionCache.needsComposerRefresh(provider.id, MODEL_SECTION)",
            "session.requestModelOptions()",
        )

        listOf(tools, features, actions).forEach { source ->
            assertTrue(source.contains("WebChatActionSheet.showUpdatable"))
        }
        assertTrue(model.contains("WebChatModelControlPopup.show("))
        assertTrue(conversationActions.contains("WebChatActionSheet.showUpdatable"))
        assertFalse(conversationActions.contains("正在打开会话操作"))
    }

    @Test
    fun clickHandlersDoNotUseBlockingOfficialReadToasts() {
        val combined = listOf(
            "WebChatProductionComposerTools.kt",
            "WebChatProductionFeatureNavigation.kt",
            "WebChatProductionPageActions.kt",
            "ChatGptSocialChatController.kt",
        ).joinToString("\n", transform = ::read)

        assertFalse(combined.contains("正在读取网页工具"))
        assertFalse(combined.contains("正在读取官网功能"))
        assertFalse(combined.contains("正在读取当前网页操作"))
        assertFalse(combined.contains("正在同步官网档位"))
        assertTrue(combined.contains("interactionCache"))
    }

    @Test
    fun productionChatPrewarmsDeclaredCapabilitiesBeforeTheUserOpensMenus() {
        val feature = read("MainSocialAiChatFeature.kt")
        val prewarmer = read("WebChatProductionCapabilityPrewarmer.kt")

        assertTrue(feature.contains("productionCapabilityPrewarmer.schedule(provider)"))
        assertTrue(prewarmer.contains("WebChatProviderCapability.MODEL_SELECTOR"))
        assertTrue(prewarmer.contains("WebChatProviderCapability.COMPOSER_TOOLS"))
        assertTrue(prewarmer.contains("WebChatProviderCapability.FEATURE_NAVIGATION"))
        assertTrue(prewarmer.contains("WebChatProviderCapability.PAGE_ACTIONS"))
        assertTrue(prewarmer.contains("SUCCESS_COOLDOWN_MS"))
        assertTrue(prewarmer.contains("RETRY_DELAYS_MS"))
        assertTrue(prewarmer.contains("needsComposerRefresh"))
        assertTrue(prewarmer.contains("needsFeatureRefresh"))
        assertTrue(feature.contains("productionCapabilityPrewarmer.cancel()"))
        assertTrue(feature.contains("prioritizeConsumerInteraction()"))
        assertTrue(prewarmer.contains("cache.needsControlRefresh"))
    }

    private fun assertBefore(source: String, first: String, second: String) {
        val firstIndex = source.indexOf(first)
        val secondIndex = source.indexOf(second, startIndex = firstIndex.coerceAtLeast(0))
        assertTrue("missing first marker: $first", firstIndex >= 0)
        assertTrue("$first must run before $second", secondIndex > firstIndex)
    }

    private fun read(fileName: String): String = String(
        Files.readAllBytes(repositoryRoot().resolve(
            "android/app/src/main/kotlin/com/elon/app/$fileName",
        )),
        StandardCharsets.UTF_8,
    )

    private fun repositoryRoot(): Path {
        var current = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        repeat(5) {
            if (Files.exists(current.resolve("android/app/src/main"))) return current
            current = current.parent ?: return@repeat
        }
        error("Repository root not found from ${System.getProperty("user.dir")}")
    }
}
