package com.elon.app.googleweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GoogleWebConversationSnapshotStoreTest {
    @Test
    fun createsAProviderScopedFileNameForAValidatedConversationPath() {
        val id = "a".repeat(64)

        assertEquals(
            "google-web-conversation-$id-v1.json",
            GoogleWebConversationSnapshotStore.fileName("/google-ai-mode/conversation/$id"),
        )
    }

    @Test
    fun rejectsUnknownOrTraversalPaths() {
        assertNull(GoogleWebConversationSnapshotStore.fileName("/conversation/${"a".repeat(64)}"))
        assertNull(GoogleWebConversationSnapshotStore.fileName(
            "/google-ai-mode/conversation/../../private",
        ))
        assertNull(GoogleWebConversationSnapshotStore.fileName(
            "/google-ai-mode/conversation/${"A".repeat(64)}",
        ))
    }
}
