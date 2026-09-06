package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPrivateImageGalleryProtocolTest {
    private fun page() = JSONObject()
        .put("type", "image_gallery_snapshot").put("source", "private_image_gallery_v1")
        .put("requestId", "mcp_gallery1").put("state", "ready").put("observedCount", 1)
        .put("handles", JSONArray().put("image_0123456789abcdef"))
        .put("pageIndex", 0).put("hasNext", true).put("hasPrevious", false).put("unavailableCount", 0)

    @Test fun acceptsScopedOrderedPageWithoutPrivateUrls() {
        val parsed = ChatGptWebImageAssetProtocol.parseGallery(page())!!
        assertEquals("mcp_gallery1", parsed.requestId)
        assertEquals(listOf("image_0123456789abcdef"), parsed.handles)
        assertTrue(parsed.hasNext)
        assertFalse(parsed.hasPrevious)
    }

    @Test fun acceptsLoadingWithoutReplacingExistingPage() {
        val value = page().put("state", "loading").put("observedCount", 0)
        value.remove("handles")
        assertNull(ChatGptWebImageAssetProtocol.parseGallery(value)!!.handles)
        assertNull(ChatGptWebImageAssetProtocol.parseGallery(value.put("state", "ready")))
    }

    @Test fun acceptsVerifiedEmptyCatalogAndPartialPreviews() {
        val empty = page().put("observedCount", 0).put("handles", JSONArray()).put("hasNext", false)
        assertEquals(emptyList<String>(), ChatGptWebImageAssetProtocol.parseGallery(empty)!!.handles)
        val partial = page().put("state", "partial").put("observedCount", 2).put("unavailableCount", 1)
        assertEquals(1, ChatGptWebImageAssetProtocol.parseGallery(partial)!!.unavailableCount)
        assertNull(ChatGptWebImageAssetProtocol.parseGallery(partial.put("state", "ready")))
    }

    @Test fun rejectsUnsafeDuplicateAndOversizedHandleLists() {
        for (handles in listOf(
            JSONArray().put("https://files.example/private?token=secret"),
            JSONArray().put("image_0123456789abcdef").put("image_0123456789abcdef"),
            JSONArray((0..25).map { "image_" + it.toString(16).padStart(16, '0') }),
        )) assertNull(ChatGptWebImageAssetProtocol.parseGallery(page().put("handles", handles)))
    }

    @Test fun rejectsMalformedPageMetadataInsteadOfCoercingSuccess() {
        for ((key, value) in listOf(
            "pageIndex" to -1, "pageIndex" to 256, "pageIndex" to "1", "hasNext" to "true",
            "hasPrevious" to true, "observedCount" to 26, "observedCount" to 0,
            "unavailableCount" to -1, "requestId" to "unscoped",
        )) assertNull("$key=$value", ChatGptWebImageAssetProtocol.parseGallery(page().put(key, value)))
    }

    @Test fun galleryAssetReceiptKeepsItsRequestOwner() {
        val asset = JSONObject().put("source", "private_image_gallery_v1")
            .put("requestId", "mcp_gallery1").put("handle", "image_0123456789abcdef")
            .put("state", "failed").put("error", "http_error")
        assertEquals("mcp_gallery1", ChatGptWebImageAssetProtocol.parseAsset(asset)!!.galleryRequestId)
        asset.remove("requestId")
        assertNull(ChatGptWebImageAssetProtocol.parseAsset(asset))
        asset.remove("source")
        assertNull(ChatGptWebImageAssetProtocol.parseAsset(asset)!!.galleryRequestId)
    }
}
