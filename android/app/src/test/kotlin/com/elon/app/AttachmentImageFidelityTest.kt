package com.elon.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.net.Uri
import java.io.File
import java.io.RandomAccessFile
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class AttachmentImageFidelityTest {
    private val context get() = RuntimeEnvironment.getApplication()

    private fun fixture(width: Int, height: Int, format: Bitmap.CompressFormat, suffix: String): File {
        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        bitmap.eraseColor(Color.WHITE)
        // Single-pixel edges must not disappear through an upload resize/re-encode.
        for (x in 0 until width step 2) bitmap.setPixel(x, height / 2, Color.BLACK)
        val file = File.createTempFile("fidelity", suffix, context.cacheDir)
        file.outputStream().use { bitmap.compress(format, 97, it) }
        bitmap.recycle()
        return file
    }

    @Test fun longPngRetainsEveryByteAndFullDimensions() {
        val source = fixture(1080, 6000, Bitmap.CompressFormat.PNG, ".png")
        val result = copyAttachmentToCache(context, "图片", Uri.fromFile(source), source.name, 1)
        assertArrayEquals(source.readBytes(), result.file.readBytes())
        assertEquals(1080, result.imageWidth)
        assertEquals(6000, result.imageHeight)
        val bitmap = BitmapFactory.decodeFile(result.file.path)
        assertEquals(Color.BLACK, bitmap.getPixel(0, 3000))
        assertEquals(Color.WHITE, bitmap.getPixel(1, 3000))
        bitmap.recycle()
    }

    @Test fun jpegIsCopiedWithoutLossySecondEncode() {
        val source = fixture(2400, 1800, Bitmap.CompressFormat.JPEG, ".jpg")
        val result = copyAttachmentToCache(context, "图片", Uri.fromFile(source), source.name, 1)
        assertArrayEquals(source.readBytes(), result.file.readBytes())
        assertEquals(2400, result.imageWidth)
        assertEquals("image/jpeg", result.mimeType)
    }

    @Test fun oversizePhotoRetainsCompressionCapabilityAndLabelsIt() {
        val source = fixture(100, 100, Bitmap.CompressFormat.PNG, ".png")
        RandomAccessFile(source, "rw").use { it.setLength(MAX_ATTACHMENT_BYTES.toLong() + 1) }
        val result = copyAttachmentToCache(context, "图片", Uri.fromFile(source), source.name, 1)
        assertEquals("图片（已压缩）", result.displayLabel)
        assertTrue(result.file.length() in 1..MAX_ATTACHMENT_BYTES.toLong())
        assertEquals(100, result.imageWidth)
        assertEquals(100, result.imageHeight)
    }

    @Test fun invalidImageAndOversizeDocumentLeaveNoPartialFiles() {
        val directory = File(context.cacheDir, "pending_attachments").apply { mkdirs() }
        val before = directory.listFiles().orEmpty().map { it.name }.toSet()
        val invalid = File.createTempFile("invalid", ".png", context.cacheDir).apply { writeText("not image") }
        assertThrows(IllegalArgumentException::class.java) {
            copyAttachmentToCache(context, "图片", Uri.fromFile(invalid), invalid.name, 1)
        }
        val large = File.createTempFile("large", ".bin", context.cacheDir)
        RandomAccessFile(large, "rw").use { it.setLength(MAX_ATTACHMENT_BYTES.toLong() + 1) }
        assertThrows(IllegalArgumentException::class.java) {
            copyAttachmentToCache(context, "文档", Uri.fromFile(large), large.name, 1)
        }
        assertEquals(before, directory.listFiles().orEmpty().map { it.name }.toSet())
    }

    @Test fun socialUploadAllowsTwelveMbOriginalWithoutChangingWorkLimit() {
        val source = fixture(100, 100, Bitmap.CompressFormat.PNG, ".png")
        RandomAccessFile(source, "rw").use { it.setLength(9L * 1024 * 1024) }
        val result = copyAttachmentToCache(context, "图片", Uri.fromFile(source), source.name, 1, 12 * 1024 * 1024)
        assertEquals("图片", result.displayLabel)
        assertArrayEquals(source.readBytes(), result.file.readBytes())
        assertEquals(8 * 1024 * 1024, MAX_ATTACHMENT_BYTES)
    }
}
