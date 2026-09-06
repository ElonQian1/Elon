package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebNativeAttachmentPolicyTest {
    @Test fun acceptsAlreadyNormalizedPhotosAndText() {
        assertTrue(ChatGptWebNativeAttachmentPolicy.supports("text/plain", 78, null, null))
        listOf("image/png", "image/jpeg", "image/webp").forEach {
            assertTrue(ChatGptWebNativeAttachmentPolicy.supports(it, 1024, 2400, 1600))
        }
    }

    @Test fun rejectsUnknownImageMetadataAndUncoveredTypes() {
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("image/png", 1024, null, 100))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("image/png", 1024, 0, 100))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("image/jpeg", 1024, 3000, 2000))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("image/png", 1024, Int.MAX_VALUE, Int.MAX_VALUE))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("image/gif", 1024, 100, 100))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("application/octet-stream", 1024, null, null))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("application/msword", 1024, null, null))
    }

    @Test fun acceptsPdfBytesWithoutImageMetadataButKeepsTheSameSizeLimit() {
        assertTrue(ChatGptWebNativeAttachmentPolicy.supports("application/pdf", 1024, null, null))
        assertTrue(ChatGptWebNativeAttachmentPolicy.supports("application/pdf", 8 * 1024 * 1024L, null, null))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("application/pdf", 0, null, null))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("application/pdf", 8 * 1024 * 1024L + 1, null, null))
    }

    @Test fun boundsSelectedBytesBeforeCreatingALease() {
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("text/plain", 0, null, null))
        assertFalse(ChatGptWebNativeAttachmentPolicy.supports("image/png", 8 * 1024 * 1024L + 1, 100, 100))
        assertTrue(ChatGptWebNativeAttachmentPolicy.supports("image/png", 8 * 1024 * 1024L, 2000, 2000))
    }
}
