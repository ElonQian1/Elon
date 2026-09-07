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

    @Test fun eachBatchFileKeepsItsOwnCopyChoice() {
        val original = file()
        val files = mutableListOf(original, file())
        val other = files[1]
        assertTrue(updatePendingAttachmentUploadChoice(files, original, true))
        assertTrue(files[0].chatGptUploadCopy)
        assertSame(other, files[1])
        assertFalse(files[1].chatGptUploadCopy)
        assertTrue(updatePendingAttachmentUploadChoice(files, files[0], false))
        assertFalse(files[0].chatGptUploadCopy)
    }
}
