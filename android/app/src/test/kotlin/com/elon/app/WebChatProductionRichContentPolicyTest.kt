package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProductionRichContentPolicyTest {
    @Test
    fun inlineRenderedPartsDoNotCreateDuplicateOfficialFallbackRows() {
        val parts = listOf("citation", "code", "table", "math", "artifact", "chart", "interactive")
            .map { type -> WebChatProductionContentPart(type = type, label = type) }

        assertEquals(
            listOf("artifact", "chart", "interactive"),
            WebChatProductionRichContentPolicy.fallbackParts(parts).map { it.type },
        )
    }

    @Test
    fun fallbackPartOrderRemainsStable() {
        val parts = listOf(
            WebChatProductionContentPart(type = "video", label = "video"),
            WebChatProductionContentPart(type = "map", label = "map"),
            WebChatProductionContentPart(type = "audio", label = "audio"),
        )

        assertEquals(parts, WebChatProductionRichContentPolicy.fallbackParts(parts))
    }

    @Test
    fun nativeImagePreviewRetainsOnlyOpaqueAssetMetadata() {
        val image = WebChatProductionContentPart(
            type = "image",
            label = "图片",
            assetHandle = "image_0123456789abcdef",
            imageSource = "/private/cache/image_0123456789abcdef.jpg",
            imageWidth = 1024,
            imageHeight = 576,
            previewPending = false,
        )

        assertEquals(listOf(image), WebChatProductionRichContentPolicy.fallbackParts(listOf(image)))
    }
}
