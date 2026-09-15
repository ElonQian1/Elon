package com.elon.app

import com.elon.app.AiConversationShareMediaException.Reason
import java.io.File
import java.io.RandomAccessFile
import java.nio.file.Files
import java.security.MessageDigest
import java.util.concurrent.CancellationException
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeNoException
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class AiConversationShareMediaSourcesTest {
    @get:Rule val temporary = TemporaryFolder()
    private val handle = "image_" + "1".repeat(16)
    private val bytes = byteArrayOf(1, 2, 3, 4)
    private fun reader() = AiConversationShareMediaSources(temporary.root, "com.elon.app")
    private fun cached(path: String, content: ByteArray = bytes): File = File(temporary.root, path).apply {
        parentFile.mkdirs()
        writeBytes(content)
    }

    @Test fun readsOnlyTheSelectedPreviewAndBindsItsHandle() {
        val file = cached("chatgpt-web-image-assets-v1/$handle.jpg")
        cached("chatgpt-web-image-assets-v1/image_${"2".repeat(16)}.jpg", byteArrayOf(9))
        val result = reader().read(file.absolutePath, handle)
        assertArrayEquals(bytes, result.bytes)
        assertTrue(result.isPreview)
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(file.absolutePath, "image_${"2".repeat(16)}") }
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(file.absolutePath, null) }
    }

    @Test fun readsSelectedAttachmentPathsAndFileUrisWithoutTreatingThemAsOriginals() {
        val file = cached("pending_attachments/attachment.jpg")
        assertArrayEquals(bytes, reader().read(file.absolutePath, null).bytes)
        assertArrayEquals(bytes, reader().read(file.toURI().toString(), null).bytes)
        assertFalse(reader().read(file.absolutePath, null).isPreview)
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(file.absolutePath, handle) }
    }

    @Test fun readsOnlyAppFileProviderCacheMappingsIncludingExactUploadStagingLayout() {
        cached("attachments/selected image.png")
        cached("chatgpt_web_uploads/1789473000000/0/selected.png")
        assertArrayEquals(bytes, reader().read(
            "content://com.elon.app.fileprovider/attachment_cache/selected%20image.png", null).bytes)
        assertArrayEquals(bytes, reader().read(
            "content://com.elon.app.fileprovider/chatgpt_web_uploads/1789473000000/0/selected.png", null).bytes)
    }

    @Test fun rejectsForeignProvidersTraversalCredentialsAndNonImageProviderRoots() {
        for (source in listOf(
            "content://foreign.provider/attachment_cache/selected.png",
            "content://com.elon.app.fileprovider/elon_updates/selected.png",
            "content://com.elon.app.fileprovider/official_quant_apk/selected.png",
            "content://com.elon.app.fileprovider/attachment_cache/%2e%2e/secret.png",
            "content://com.elon.app.fileprovider/attachment_cache/%2fsecret.png",
            "content://com.elon.app.fileprovider/attachment_cache/%2e%2e%5csecret.png",
            "content://com.elon.app.fileprovider/attachment_cache/%00secret.png",
            "content://com.elon.app.fileprovider/attachment_cache/x.png?token=secret",
            "content://user@com.elon.app.fileprovider/attachment_cache/x.png",
            "content://com.elon.app.fileprovider/chatgpt_web_uploads/arbitrary/private/file.png",
            "blob:https://chatgpt.com/private", "data:image/png;base64,c2VjcmV0",
            "https://user:secret@example.invalid/image.png", "file://server/private/image.png",
            "attachment_cache/relative.png",
        )) rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(source, null) }
    }

    @Test fun remoteSourcesUseOnlyTheExistingDiskCacheAndNeverFetchOnMiss() {
        // Loopback port 1 is intentionally not served. A miss must remain a typed local-cache error.
        val url = "http://127.0.0.1:1/image.png?signature=private"
        val file = cached("chat_image_cache/${hash(url)}.img")
        assertArrayEquals(bytes, reader().read(url, null).bytes)
        assertTrue(file.delete())
        rejected(Reason.LOCAL_IMAGE_MISSING_RETRY) { reader().read(url, null) }
        assertFalse(file.exists())
    }

    @Test fun rejectsFilesOutsideKnownCacheRootsAndSiblingPrefixConfusion() {
        val file = cached("private/secret.png")
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(file.absolutePath, null) }
        val sibling = cached("pending_attachments-other/selected.png")
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(sibling.absolutePath, null) }
        val nested = cached("pending_attachments/arbitrary/selected.png")
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(nested.absolutePath, null) }
        val traversal = File(temporary.root, "pending_attachments/../private/secret.png")
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(traversal.absolutePath, null) }
    }

    @Test fun rejectsFileSymlinksEvenWhenTheirTargetIsInAnotherAllowedCache() {
        val target = cached("attachments/target.png")
        val link = File(temporary.root, "pending_attachments/link.png").apply { parentFile.mkdirs() }
        try { Files.createSymbolicLink(link.toPath(), target.toPath()) }
        catch (error: Exception) { assumeNoException("Symlink creation requires OS support", error) }
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(link.absolutePath, null) }
    }

    @Test fun rejectsSymlinkedCacheDirectories() {
        val target = temporary.newFolder("private")
        File(target, "secret.png").writeBytes(bytes)
        val link = File(temporary.root, "attachments")
        try { Files.createSymbolicLink(link.toPath(), target.toPath()) }
        catch (error: Exception) { assumeNoException("Symlink creation requires OS support", error) }
        rejected(Reason.SOURCE_NOT_ALLOWED) { reader().read(File(link, "secret.png").absolutePath, null) }
    }

    @Test fun enforcesTheActualTwelveMiBBoundAndAcceptsTheExactBoundary() {
        val file = cached("attachments/bounded.png")
        RandomAccessFile(file, "rw").use { it.setLength(AiConversationShareMediaPreparer.MAX_ASSET_BYTES.toLong()) }
        assertEquals(AiConversationShareMediaPreparer.MAX_ASSET_BYTES, reader().read(file.absolutePath, null).bytes.size)
        RandomAccessFile(file, "rw").use { it.setLength(AiConversationShareMediaPreparer.MAX_ASSET_BYTES + 1L) }
        rejected(Reason.ASSET_TOO_LARGE) { reader().read(file.absolutePath, null) }
    }

    @Test fun boundsFilesThatGrowAfterTheInitialSizeCheck() {
        val file = cached("attachments/growing.png")
        var grown = false
        rejected(Reason.ASSET_TOO_LARGE) {
            reader().read(file.absolutePath, null) {
                if (!grown) {
                    grown = true
                    RandomAccessFile(file, "rw").use { it.setLength(AiConversationShareMediaPreparer.MAX_ASSET_BYTES + 1L) }
                }
            }
        }
    }

    @Test fun propagatesCancellationAndSanitizesMissingFileErrors() {
        val file = cached("attachments/selected.png")
        val cancellation = CancellationException("cancelled")
        try {
            reader().read(file.absolutePath, null) { throw cancellation }
            throw AssertionError("Cancellation was swallowed")
        } catch (error: CancellationException) { assertTrue(error === cancellation) }
        rejected(Reason.LOCAL_IMAGE_MISSING_RETRY) {
            reader().read(File(temporary.root, "attachments/private-name.png").absolutePath, null)
        }
        rejected(Reason.INVALID_IMAGE) { reader().read(cached("attachments/empty.png", byteArrayOf()).absolutePath, null) }
    }

    private fun hash(value: String): String = MessageDigest.getInstance("SHA-256")
        .digest(value.toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it.toInt() and 0xff) }

    private fun rejected(reason: Reason, block: () -> Unit) {
        try { block(); throw AssertionError("Expected $reason") }
        catch (error: AiConversationShareMediaException) {
            assertEquals(reason, error.reason)
            assertEquals(reason.name, error.message)
            assertEquals(null, error.cause)
        }
    }
}
