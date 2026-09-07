package com.elon.app

import java.io.File
import org.junit.Assert.*
import org.junit.Test

class PendingAttachmentUploadChoiceTest {
    private fun file() = PendingAttachment(
        kind = "file", displayName = "fixture.txt", fileName = "fixture.txt",
        mimeType = "text/plain", file = File("synthetic-fixture.txt"),
    )

    @Test fun defaultRemainsReuseAndExplicitChoiceOnlyChangesItsFlag() {
        val original = file()
        val files = mutableListOf(original)
        assertFalse(original.chatGptUploadCopy)
        assertFalse(updatePendingAttachmentUploadChoice(files, original, false))
        assertSame(original, files.single())
        assertTrue(updatePendingAttachmentUploadChoice(files, original, true))
        assertEquals(original.copy(chatGptUploadCopy = true), files.single())
        assertFalse(original.chatGptUploadCopy)
        assertTrue(updatePendingAttachmentUploadChoice(files, files.single(), false))
        assertEquals(original, files.single())
    }

    @Test fun anOldMenuCannotMutateARemovedEditedOrReplacedFile() {
        val original = file()
        for (files in listOf(mutableListOf(), mutableListOf(original.copy()),
            mutableListOf(original.copy(file = File("edited.txt"))))) {
            val before = files.toList()
            assertFalse(updatePendingAttachmentUploadChoice(files, original, true))
            assertEquals(before, files)
        }
    }

    @Test fun addingAnotherAttachmentCannotEnableCopyButCanResetAnEarlierChoice() {
        val original = file()
        val files = mutableListOf(original, file())
        assertFalse(updatePendingAttachmentUploadChoice(files, original, true))
        files[0] = original.copy(chatGptUploadCopy = true)
        assertTrue(updatePendingAttachmentUploadChoice(files, files[0], false))
        assertFalse(files[0].chatGptUploadCopy)
    }
}
