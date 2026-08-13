package com.elon.app

import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebAcceptanceAttachmentFixtureTest {
    @Test
    fun createsOnlyTheFixedHarmlessTextFixtureAndCleansItUp() {
        val cache = Files.createTempDirectory("chatgpt-web-fixture").toFile()
        try {
            val attachment = ChatGptWebAcceptanceAttachmentFixture.prepare(cache)

            assertEquals("document", attachment.kind)
            assertEquals(ChatGptWebAcceptanceAttachmentFixture.FILE_NAME, attachment.fileName)
            assertEquals(ChatGptWebAcceptanceAttachmentFixture.MIME_TYPE, attachment.mimeType)
            assertEquals(
                ChatGptWebAcceptanceAttachmentFixture.expectedContent(),
                attachment.file.readText(Charsets.UTF_8),
            )
            assertTrue(ChatGptWebAcceptanceAttachmentFixture.matches(cache, attachment))
            assertFalse(
                ChatGptWebAcceptanceAttachmentFixture.matches(
                    cache,
                    attachment.copy(fileName = "user-file.txt"),
                ),
            )

            ChatGptWebAcceptanceAttachmentFixture.cleanup(cache)
            assertFalse(attachment.file.exists())
        } finally {
            cache.deleteRecursively()
        }
    }
}
