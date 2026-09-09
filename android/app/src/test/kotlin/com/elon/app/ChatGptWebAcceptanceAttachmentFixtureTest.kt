package com.elon.app

import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebAcceptanceAttachmentFixtureTest {
    @Test
    fun fixedMediaBatchHasExactOrderedMetadataAndNoUserPaths() {
        val cache = Files.createTempDirectory("chatgpt-web-media").toFile()
        try {
            val files = ChatGptWebAcceptanceAttachmentFixture.prepareBatch(
                cache, ChatGptWebAcceptanceAttachmentFixture.MEDIA_BATCH_ID,
            ) { spec, target -> target.writeText("synthetic ${spec.mime}") }
            assertEquals(listOf("text/plain", "image/png", "application/pdf"), files.map { it.mimeType })
            assertEquals(listOf("document", "image", "document"), files.map { it.kind })
            assertEquals(512, files[1].imageWidth)
            assertEquals(384, files[1].imageHeight)
            assertTrue(ChatGptWebAcceptanceAttachmentFixture.matchesSelection(
                cache, files, ChatGptWebAcceptanceAttachmentFixture.MEDIA_BATCH_ID,
            ))
            assertFalse(ChatGptWebAcceptanceAttachmentFixture.matchesSelection(
                cache, files.take(1), ChatGptWebAcceptanceAttachmentFixture.MEDIA_BATCH_ID,
            ))
            assertFalse(ChatGptWebAcceptanceAttachmentFixture.matches(cache, files[1].copy(file = cache)))
            ChatGptWebAcceptanceAttachmentFixture.cleanup(cache)
            assertTrue(files.none { it.file.exists() })
        } finally { cache.deleteRecursively() }
    }

    @Test
    fun failedMediaPreparationCleansPartialAndTemporaryFiles() {
        val cache = Files.createTempDirectory("chatgpt-web-media-failure").toFile()
        try {
            val result = runCatching {
                ChatGptWebAcceptanceAttachmentFixture.prepareBatch(
                    cache, ChatGptWebAcceptanceAttachmentFixture.MEDIA_BATCH_ID,
                ) { _, target -> target.writeText("partial"); error("synthetic writer failure") }
            }
            assertTrue(result.isFailure)
            assertTrue(cache.walkTopDown().none { it.isFile })
            assertFalse(ChatGptWebAcceptanceAttachmentFixture.supports("../../user-photo.png"))
        } finally { cache.deleteRecursively() }
    }

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
