package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebProductCapabilityCatalogTest {
    @Test
    fun keepsPageAndComposerCapabilitiesUniqueAndProtocolSafe() {
        val pages = ChatGptWebProductCapabilityCatalog.PAGE_FEATURES
        val tools = ChatGptWebProductCapabilityCatalog.COMPOSER_TOOLS

        assertEquals(pages.size, pages.map { it.id }.toSet().size)
        assertEquals(pages.size, pages.map { it.protocolKind }.toSet().size)
        assertEquals(tools.size, tools.map { it.id }.toSet().size)
        assertTrue(pages.all { it.protocolKind in ChatGptWebProductCapabilityCatalog.FEATURE_KINDS })
        assertTrue(tools.all { it.semantic in ChatGptWebComposerOptionSemantics.KNOWN })
        assertTrue(ChatGptWebFeatureBaseline.ids().containsAll(pages.map { it.id }))
        assertTrue(ChatGptWebFeatureBaseline.ids().containsAll(tools.map { it.id }))
        assertTrue("/studymode" in ChatGptWebProductCapabilityCatalog.RESTORABLE_PREFIXES)
    }

    @Test
    fun marksPersonalHealthAndFinancialPagesAsSensitive() {
        assertTrue(ChatGptWebProductCapabilityCatalog.requiresUserConfirmation("health"))
        assertTrue(ChatGptWebProductCapabilityCatalog.requiresUserConfirmation("finances"))
        assertFalse(ChatGptWebProductCapabilityCatalog.requiresUserConfirmation("work"))
        assertFalse(ChatGptWebProductCapabilityCatalog.requiresUserConfirmation("images"))
        assertFalse(ChatGptWebProductCapabilityCatalog.requiresUserConfirmation("library"))
    }

    @Test
    fun requiresConfirmationOnlyForObservedSensitiveFeatureSelection() {
        val health = ChatGptWebFeature("feature_health", "健康", "health", false)
        val images = ChatGptWebFeature("feature_images", "图像", "images", false)

        assertEquals(
            "stale_feature_id",
            ChatGptWebProductCapabilityCatalog.selectionError(null, false),
        )
        assertEquals(
            "user_confirmation_required",
            ChatGptWebProductCapabilityCatalog.selectionError(health, false),
        )
        assertEquals(null, ChatGptWebProductCapabilityCatalog.selectionError(health, true))
        assertEquals(null, ChatGptWebProductCapabilityCatalog.selectionError(images, false))
        assertTrue(
            ChatGptWebProductCapabilityCatalog.navigationJson(health)
                .getBoolean("requires_user_confirmation"),
        )
    }

    @Test
    fun reportsObservedPageAndComposerCapabilitiesWithoutContent() {
        val result = ChatGptWebProductCapabilityCatalog.describe(
            features = listOf(ChatGptWebFeature("feature_health", "Health", "health", false)),
            composerOptions = listOf(
                ChatGptWebComposerOption(
                    "tools_study",
                    "学习",
                    false,
                    "menuitem",
                    ChatGptWebComposerOptionSemantics.STUDY,
                ),
            ),
            controls = emptyList(),
        )

        val pages = result.getJSONArray("page_features")
        val health = (0 until pages.length())
            .map(pages::getJSONObject)
            .first { it.getString("id") == "health" }
        val tools = result.getJSONArray("composer_tools")
        val study = (0 until tools.length())
            .map(tools::getJSONObject)
            .first { it.getString("id") == "study_mode" }

        assertTrue(health.getBoolean("current_page_observed"))
        assertTrue(health.getBoolean("requires_user_confirmation"))
        assertTrue(study.getBoolean("current_page_observed"))
    }
}
