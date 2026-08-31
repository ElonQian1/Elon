package com.elon.app.chatgptweb

import java.nio.file.Files
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebImageAssetStoreTest {
    @Test
    fun gallerySyncMarkerIsFreshOnlyWithinConfiguredWindow() {
        val root = Files.createTempDirectory("chatgpt-image-store").toFile()
        try {
            val store = ChatGptWebImageAssetStore(root, synchronous = true)
            val syncedAt = System.currentTimeMillis() - 1_000L

            assertFalse(store.hasFreshGallerySync(nowMs = syncedAt, maxAgeMs = 5_000L))
            assertTrue(store.markGallerySynced(syncedAt))
            assertTrue(store.hasFreshGallerySync(nowMs = syncedAt + 4_999L, maxAgeMs = 5_000L))
            assertFalse(store.hasFreshGallerySync(nowMs = syncedAt + 5_001L, maxAgeMs = 5_000L))
            assertFalse(store.hasFreshGallerySync(nowMs = syncedAt - 1L, maxAgeMs = 5_000L))
            assertTrue(store.handles().isEmpty())
        } finally {
            root.deleteRecursively()
        }
    }
}
