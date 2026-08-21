package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionInteractionSnapshotCodecTest {
    @Test
    fun roundTripsOnlyStableCatalogMetadata() {
        val source = WebChatProductionInteractionSnapshot(
            composer = listOf(WebChatProductionComposerSnapshot(
                providerId = WebChatProviderId.CHATGPT_WEB,
                section = "model",
                updatedAtMs = 42L,
                options = listOf(option("fast", selected = true)),
            )),
            features = listOf(WebChatProductionFeatureSnapshot(
                providerId = WebChatProviderId.CHATGPT_WEB,
                updatedAtMs = 43L,
                features = listOf(feature("images", selected = true)),
            )),
        )

        val raw = WebChatProductionInteractionSnapshotCodec.encode(source)
        val decoded = WebChatProductionInteractionSnapshotCodec.decode(raw)!!

        assertFalse(raw.contains("cookie", ignoreCase = true))
        assertFalse(raw.contains("message", ignoreCase = true))
        assertEquals("fast", decoded.composer.single().options.single().id)
        assertFalse(decoded.composer.single().options.single().selected)
        assertEquals("images", decoded.features.single().features.single().id)
        assertFalse(decoded.features.single().features.single().selected)
    }

    @Test
    fun rejectsUnknownSchemasAndUnknownProviders() {
        assertNull(WebChatProductionInteractionSnapshotCodec.decode("{\"schema\":\"old\"}"))
        val decoded = WebChatProductionInteractionSnapshotCodec.decode(
            """{"schema":"elon.web_chat.production_interactions.v1","composer":[{"provider":"other","section":"model","updated_at_ms":1,"options":[{"id":"x","label":"X"}]}],"features":[]}""",
        )!!
        assertTrue(decoded.composer.isEmpty())
    }

    private fun option(id: String, selected: Boolean) = WebChatConsumerOption(
        id = id,
        label = "快速",
        selected = selected,
        semantic = "model",
        opensSubmenu = false,
        nativeSelector = "option:$id",
    )

    private fun feature(id: String, selected: Boolean) = WebChatConsumerFeature(
        id = id,
        label = "图像",
        kind = "images",
        selected = selected,
        requiresUserConfirmation = false,
        nativeSelector = "feature:$id",
    )
}
