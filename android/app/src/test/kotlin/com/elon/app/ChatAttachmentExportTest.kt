package com.elon.app

import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import java.io.File
import java.io.RandomAccessFile
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class ChatAttachmentExportTest {
    private val context get() = RuntimeEnvironment.getApplication()
    private val mediaRoot get() = File(context.cacheDir, "pending_attachments").apply { mkdirs() }
    @Before fun fileProviderPaths() = installWindowsExportFileProvider(context)
    private fun attachment(): ChatAttachment {
        val file = File.createTempFile("image-fixture", ".png", mediaRoot)
        val bitmap = Bitmap.createBitmap(3, 2, Bitmap.Config.ARGB_8888)
        bitmap.eraseColor(android.graphics.Color.GREEN)
        file.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        bitmap.recycle()
        return ChatAttachment(kind = "image", fileName = "fixture.png", localPath = file.path)
    }

    @Test fun imageExportPreservesOriginalBytesAndUsesContentUri() {
        val attachment = attachment()
        val file = ChatAttachmentExport.prepare(context, listOf(attachment)).single()
        assertEquals("content", file.uri.scheme)
        assertEquals("${context.packageName}.fileprovider", file.uri.authority)
        assertEquals("image/png", file.mimeType)
        assertArrayEquals(File(attachment.localPath!!).readBytes(), context.contentResolver.openInputStream(file.uri)!!.use { it.readBytes() })
        val clip = ChatAttachmentExport.clip(context, listOf(file), "caption")
        assertEquals(file.uri, clip.getItemAt(0).uri)
        assertEquals("caption", clip.getItemAt(1).text)
        assertTrue(clip.description.hasMimeType("image/png"))
    }

    @Test fun singleAndMultipleShareUseReadOnlyStreamsAndRetainCaption() {
        val files = ChatAttachmentExport.prepare(context, listOf(attachment(), attachment()))
        val single = ChatAttachmentExport.intent(context, files.take(1), "caption")
        assertEquals(Intent.ACTION_SEND, single.action)
        assertEquals(files.first().uri, single.getParcelableExtra<Uri>(Intent.EXTRA_STREAM))
        val multiple = ChatAttachmentExport.intent(context, files, "caption")
        assertEquals(Intent.ACTION_SEND_MULTIPLE, multiple.action)
        assertEquals(files.map { it.uri }, multiple.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM))
        for (intent in listOf(single, multiple)) {
            assertEquals("image/png", intent.type)
            assertEquals("caption", intent.getStringExtra(Intent.EXTRA_TEXT))
            assertEquals(Intent.FLAG_GRANT_READ_URI_PERMISSION, intent.flags)
            assertNotNull(intent.clipData)
        }
    }

    @Test fun imageForwardCanBeImportedByOurExistingShareReceiver() {
        val original = attachment()
        val files = ChatAttachmentExport.prepare(context, listOf(original))
        val store = com.elon.app.sharing.ShareDraftStore(context)
        val draft = store.import(ChatAttachmentExport.intent(context, files, "fixture caption"))
        try {
            assertEquals("fixture caption", draft.text)
            assertEquals(1, draft.files.size)
            assertEquals("image", draft.files.single().kind)
            assertArrayEquals(File(original.localPath!!).readBytes(), draft.files.single().file.readBytes())
        } finally { store.remove(draft) }
    }

    @Test fun invalidAndOversizeAttachmentsFailWithoutReturningPartialShare() {
        val invalid = File.createTempFile("invalid-fixture", ".png", mediaRoot).apply { writeText("not an image") }
        val bad = ChatAttachment(kind = "image", localPath = invalid.path)
        assertThrows(IllegalArgumentException::class.java) { ChatAttachmentExport.prepare(context, listOf(attachment(), bad)) }
        val large = File.createTempFile("large-fixture", ".bin", mediaRoot)
        RandomAccessFile(large, "rw").use { it.setLength(ChatAttachmentExport.MAX_BYTES.toLong() + 1) }
        assertThrows(IllegalArgumentException::class.java) {
            ChatAttachmentExport.prepare(context, listOf(ChatAttachment(kind = "file", localPath = large.path)))
        }
        large.delete()
        assertThrows(IllegalArgumentException::class.java) { ChatAttachmentExport.prepare(context, List(7) { bad }) }
    }

    @Test fun repeatedExportReusesBytesAndOnlyExpiresOldExportCache() {
        val image = attachment()
        val first = ChatAttachmentExport.prepare(context, listOf(image)).single()
        val directory = File(context.cacheDir, "message_exports")
        val expired = File(directory, "expired.bin").apply { writeText("old"); setLastModified(1) }
        val recent = File(directory, "recent.bin").apply { writeText("keep") }
        val second = ChatAttachmentExport.prepare(context, listOf(image)).single()
        assertEquals(first.uri, second.uri)
        assertFalse(expired.exists())
        assertTrue(recent.exists())
        assertTrue(File(image.localPath!!).exists())
    }

    @Test fun fileAndImageTogetherUseMixedMimeAndBothStreams() {
        val document = File.createTempFile("text-fixture", ".txt", mediaRoot).apply { writeText("fixture") }
        val files = ChatAttachmentExport.prepare(context, listOf(attachment(),
            ChatAttachment(kind = "file", mimeType = "text/plain", localPath = document.path, fileName = "note.txt")))
        val intent = ChatAttachmentExport.intent(context, files, "")
        assertEquals("*/*", intent.type)
        assertEquals(2, intent.clipData!!.itemCount)
        assertFalse(intent.hasExtra(Intent.EXTRA_TEXT))
    }

    @Test fun attachmentMetadataCannotExportUnrelatedAppFiles() {
        val privateFixture = File.createTempFile("private-fixture", ".txt", context.filesDir).apply { writeText("not media") }
        try {
            assertThrows(IllegalArgumentException::class.java) {
                ChatAttachmentExport.prepare(context, listOf(ChatAttachment(kind = "file", localPath = privateFixture.path)))
            }
            assertTrue(privateFixture.exists())
        } finally { privateFixture.delete() }
    }
}
