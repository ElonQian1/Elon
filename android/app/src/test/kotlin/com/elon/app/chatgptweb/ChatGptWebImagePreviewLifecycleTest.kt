package com.elon.app.chatgptweb

import com.elon.app.WebChatProviderId
import com.elon.app.WebChatProviderRegistry
import java.nio.file.Files
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebImagePreviewLifecycleTest {
    private val root = Files.createTempDirectory("image-preview-state").toFile()
    private val store = ChatGptWebImageAssetStore(root, synchronous = true)
    private val requested = mutableListOf<String>()
    private val scheduled = linkedSetOf<Runnable>()
    private val callbacks = mutableListOf<() -> Unit>()
    private var acceptsRequest = true
    private val coordinator = ChatGptWebImageAssetCoordinator(
        store, { requested += it; acceptsRequest },
        { task, _ -> scheduled += task }, { scheduled.remove(it) },
        { callbacks += it }, {},
    )
    private val first = "image_0123456789abcdef"
    private val second = "image_fedcba9876543210"

    @After fun cleanup() { coordinator.reset(); root.deleteRecursively() }

    @Test fun repeatedSnapshotsCannotRequeueAnActiveOrExhaustedPreview() {
        repeat(10) { coordinator.observe(snapshot(first)) }
        assertEquals(listOf(first), requested)
        expire()
        repeat(10) { coordinator.observe(snapshot(first)) }
        expire()
        assertEquals(2, requested.size)
        repeat(10) { coordinator.observe(snapshot(first)) }
        assertEquals(ChatGptWebImagePreviewState.FAILED, coordinator.state(first))
        assertTrue(scheduled.isEmpty())
        coordinator.retry(first)
        assertEquals(3, requested.size)
        assertEquals(ChatGptWebImagePreviewState.PREPARING, coordinator.state(first))
    }

    @Test fun failedImageDoesNotBorrowAnotherImagesPreparingState() {
        coordinator.observe(snapshot(first, second))
        coordinator.accept(ChatGptWebImageAsset(first, "failed"))
        coordinator.accept(ChatGptWebImageAsset(second, "failed"))
        coordinator.accept(ChatGptWebImageAsset(first, "failed"))
        assertEquals(ChatGptWebImagePreviewState.FAILED, coordinator.state(first))
        assertEquals(ChatGptWebImagePreviewState.PREPARING, coordinator.state(second))
        val parts = mapParts(first, second)
        assertTrue(parts[0].previewFailed)
        assertFalse(parts[0].previewPending)
        assertFalse(parts[1].previewFailed)
        assertTrue(parts[1].previewPending)
    }

    @Test fun missingDispatcherIsFailureNotPermanentPreparing() {
        acceptsRequest = false
        coordinator.observe(snapshot(first))
        assertTrue(mapParts(first).single().previewFailed)
        assertFalse(mapParts(first).single().previewPending)
        acceptsRequest = true
        coordinator.retry(first)
        assertTrue(mapParts(first).single().previewPending)
    }

    @Test fun unrequestedImageIsIdleAndCachedImageIsImmediatelyVisible() {
        assertFalse(mapParts(first).single().previewPending)
        root.resolve("$first.jpg").writeBytes(ByteArray(128))
        coordinator.observe(snapshot(first))
        assertTrue(requested.isEmpty())
        assertEquals(root.resolve("$first.jpg").absolutePath, mapParts(first).single().imageSource)
    }

    @Test fun lateEventsAndRetriesDoNotDuplicateActiveRequests() {
        coordinator.observe(snapshot(first, second))
        coordinator.retry(first)
        assertEquals(1, requested.size)
        coordinator.accept(ChatGptWebImageAsset(second, "failed"))
        assertEquals(1, requested.size)
        assertEquals(ChatGptWebImagePreviewState.PREPARING, coordinator.state(second))
        coordinator.reset()
        coordinator.accept(ChatGptWebImageAsset(first, "failed"))
        assertEquals(ChatGptWebImagePreviewState.IDLE, coordinator.state())
    }

    @Test fun diskSaveCallbackCannotChangeResetSession() {
        coordinator.observe(snapshot(first))
        coordinator.accept(ChatGptWebImageAsset(first, "ready", "image/jpeg", 1, 1, "bad"))
        assertEquals(1, callbacks.size)
        coordinator.reset()
        callbacks.single().invoke()
        assertEquals(ChatGptWebImagePreviewState.IDLE, coordinator.state())
        assertTrue(scheduled.isEmpty())
    }

    private fun expire() { scheduled.first().also { scheduled.remove(it) }.run() }

    private fun mapParts(vararg handles: String) = ChatGptFriendMessageMapper.map(
        snapshot(*handles), WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB), null,
        imagePreviewPath = coordinator::resolvePath, imagePreviewState = coordinator::state,
        timestampFor = { 1L },
    ).single().webChatMessage!!.contentParts

    private fun snapshot(vararg handles: String) = ChatGptWebSnapshot(
        title = "", url = "https://chatgpt.com/", draft = "", authenticated = true,
        composerReady = true, streaming = false, currentModel = "", attachments = emptyList(),
        dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY,
        messages = listOf(ChatGptWebMessage("fixture", "assistant", "Fixture", "completed",
            handles.map { ChatGptWebMessagePart("image", "Image",
                metadata = ChatGptWebMessagePartMetadata(kind = "image", assetHandle = it)) })),
    )
}
