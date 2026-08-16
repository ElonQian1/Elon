package com.elon.app.googleweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebProjectStoreTest {
    @Test
    fun createsAssignsAndRoundTripsLocalProjects() {
        val projectId = "12345678-1234-1234-1234-123456789abc"
        val conversationPath = "/google-ai-mode/conversation/${"a".repeat(64)}"
        val created = requireNotNull(GoogleWebProjectPolicy.create(
            GoogleWebProjectSnapshot(),
            "旅行计划",
            projectId,
        ))
        val assigned = requireNotNull(GoogleWebProjectPolicy.assign(
            created,
            conversationPath,
            projectId,
        ))

        val restored = GoogleWebProjectCodec.decode(GoogleWebProjectCodec.encode(assigned))

        assertEquals(assigned, restored)
        assertEquals(projectId, restored.assignments[conversationPath])
    }

    @Test
    fun rejectsDuplicateTitlesAndUnknownProjects() {
        val firstId = "12345678-1234-1234-1234-123456789abc"
        val secondId = "87654321-4321-4321-4321-cba987654321"
        val created = requireNotNull(GoogleWebProjectPolicy.create(
            GoogleWebProjectSnapshot(),
            "研究",
            firstId,
        ))

        assertNull(GoogleWebProjectPolicy.create(created, " 研究 ", secondId))
        assertNull(GoogleWebProjectPolicy.assign(
            created,
            "/google-ai-mode/conversation/${"b".repeat(64)}",
            secondId,
        ))
        assertTrue(GoogleWebProjectCodec.decode("{}").projects.isEmpty())
    }
}
