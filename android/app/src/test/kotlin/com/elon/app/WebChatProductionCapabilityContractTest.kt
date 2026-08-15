package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionCapabilityContractTest {
    @Test
    fun everyProviderCapabilityHasAnExplicitProductionDeliveryDefinition() {
        assertEquals(
            WebChatProviderCapability.entries.toSet(),
            WebChatProductionCapabilityContract.knownCapabilities(),
        )
    }

    @Test
    fun everySelectableProviderHasCompleteProductionCoverage() {
        WebChatProviderRegistry.available().forEach { provider ->
            val description = WebChatProductionCapabilityContract.describe(provider)

            assertTrue(provider.selectable)
            assertTrue(description.getBoolean("ready"))
            assertEquals(provider.capabilities.size, description.getInt("covered_capability_count"))
            assertEquals(provider.capabilities.size, description.getJSONArray("capabilities").length())
            assertEquals(0, description.getJSONArray("missing").length())
        }
    }

    @Test
    fun providerCannotClaimAChatGptOnlyCapabilityWithoutAProductionDelivery() {
        val invalidGoogle = WebChatProviderIdentity(
            id = WebChatProviderId.GOOGLE_WEB,
            displayName = "Google 搜索网页 AI",
            avatarResId = 0,
            available = true,
            capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION +
                WebChatProviderCapability.MESSAGE_REGENERATE,
        )

        assertFalse(invalidGoogle.selectable)
        assertEquals(
            setOf(WebChatProviderCapability.MESSAGE_REGENERATE),
            WebChatProductionCapabilityContract.missing(
                invalidGoogle.id,
                invalidGoogle.capabilities,
            ),
        )
    }

    @Test
    fun contractExposesStableControlsMcpChannelsAndOfficialFallback() {
        val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val description = WebChatProductionCapabilityContract.describe(chatGpt)
        val rows = description.getJSONArray("capabilities")
        val byCapability = (0 until rows.length())
            .map(rows::getJSONObject)
            .associateBy { it.getString("capability") }

        assertEquals(
            "chatgpt-native:conversation-list:会话历史",
            byCapability.getValue("conversation_list").getString("adb_selector"),
        )
        assertEquals(
            "main_native_ui",
            byCapability.getValue("project_list").getString("mcp_channel"),
        )
        assertEquals(
            "chatgpt_web",
            byCapability.getValue("feature_navigation").getString("mcp_channel"),
        )
        assertEquals(
            "open_web_chat_official_fallback",
            byCapability.getValue("realtime_voice").getString("official_fallback_action"),
        )
    }

    @Test
    fun realFriendChatExportsTheContractThroughMainMcpState() {
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val mcp = read("android/app/src/main/kotlin/com/elon/app/MainMcpNativeControlActions.kt")

        assertTrue(feature.contains("WebChatProductionCapabilityContract.describe"))
        assertTrue(feature.contains("fun webChatProductionCapabilities"))
        assertTrue(mcp.contains("web_chat_production_capabilities"))
        assertFalse(mcp.contains("ChatGptWebTestActivity"))
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
