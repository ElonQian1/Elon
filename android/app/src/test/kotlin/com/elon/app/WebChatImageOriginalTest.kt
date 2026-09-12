package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebMessagePartParser
import com.elon.app.chatgptweb.ChatGptFriendMessageMapper
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.WebChatSnapshotCache
import com.elon.app.chatgptweb.WebChatSnapshotCacheCodec
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatImageOriginalTest {
    private val handle = "download_" + "1".repeat(32)
    private fun original() = JSONObject().put("path", "/c/synthetic").put("name", "image.png").put("handle", handle)

    @Test fun parsesOnlyOpaqueScopedOriginalDescriptors() {
        val value = WebChatImageOriginal.parse(original())!!
        assertEquals("/c/synthetic", value.path)
        assertEquals(handle, value.asFile().downloadHandle)
        assertEquals("image.png", value.asFile().name)
        assertEquals("", value.asFile().mediaType)
        assertEquals("", value.asFile().sourceUrl)
    }

    @Test fun rejectsUnboundedPathsRawUrlsAndMalformedSelections() {
        for ((key, value) in listOf(
            "path" to "/images", "path" to "/c/../../other", "path" to "https://chatgpt.com/c/synthetic",
            "handle" to "image_" + "1".repeat(16), "handle" to "https://files.oaiusercontent.com/x",
            "name" to "", "name" to "a".repeat(181), "name" to "bad\nname",
        )) assertNull(WebChatImageOriginal.parse(original().put(key, value)))
    }

    @Test fun acceptsProjectConversationScopeWithoutDroppingIt() {
        val path = "/g/g-p-" + "a".repeat(32) + "-project/c/synthetic"
        assertEquals(path, WebChatImageOriginal.parse(original().put("path", path))!!.path)
    }

    @Test fun exposesOriginalOnlyForImagePartsWithoutReplacingPreviewIdentity() {
        fun parse(type: String, original: JSONObject) = ChatGptWebMessagePartParser.parse(
            JSONObject().put("content", JSONArray().put(JSONObject().put("type", type).put("text", "image")
                .put("assetHandle", "image_" + "2".repeat(16)).put("original", original)))
        ).single().metadata!!
        assertEquals(handle, parse("image", original()).imageOriginal!!.handle)
        assertEquals("image_" + "2".repeat(16), parse("image", original()).assetHandle)
        assertNull(parse("file", original()).imageOriginal)
        assertNull(parse("image", original().put("handle", "invalid")).imageOriginal)
    }

    private fun snapshot(): ChatGptWebSnapshot {
        val parts = ChatGptWebMessagePartParser.parse(JSONObject().put("content", JSONArray().put(
            JSONObject().put("type", "image").put("text", "fixture").put("original", original())
                .put("assetHandle", "image_" + "2".repeat(16)))))
        return ChatGptWebSnapshot(title = "", url = "https://chatgpt.com/c/synthetic", draft = "",
            messages = listOf(ChatGptWebMessage("synthetic", "user", "", "completed", parts)),
            authenticated = true, composerReady = false, streaming = false, currentModel = "",
            attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY)
    }

    @Test fun mapsDownloadSelectionIntoProductionImagePreviewEvenWithoutAReadyComposer() {
        val message = ChatGptFriendMessageMapper.map(snapshot(), WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            pendingPrompt = null, imagePreviewPath = { "preview.jpg" }, timestampFor = { 1L }).single()
        val part = message.webChatMessage!!.contentParts.single()
        assertEquals("preview.jpg", part.imageSource)
        assertEquals(handle, part.imageOriginal!!.handle)
        assertEquals("image.png", part.imageOriginal!!.name)
    }

    @Test fun restartCacheNeverRestoresAnExpiredOriginalDownloadSelection() {
        val raw = WebChatSnapshotCacheCodec.encode(WebChatSnapshotCache(snapshot(), 10L))
        assertEquals(false, raw.contains(handle))
        val part = WebChatSnapshotCacheCodec.decode(raw)!!.snapshot.messages.single().parts.single()
        assertNull(part.metadata?.imageOriginal)
    }
}
