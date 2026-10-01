package com.elon.app.scan

import android.app.Application
import android.graphics.Bitmap
import android.graphics.Color
import com.google.zxing.BarcodeFormat
import com.google.zxing.qrcode.QRCodeWriter
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ScanImageDecoderTest {
    private fun qr(value: String): Bitmap {
        val matrix = QRCodeWriter().encode(value, BarcodeFormat.QR_CODE, 320, 320)
        return Bitmap.createBitmap(320, 320, Bitmap.Config.ARGB_8888).apply {
            for (y in 0 until 320) for (x in 0 until 320) setPixel(x, y, if (matrix[x, y]) Color.BLACK else Color.WHITE)
        }
    }
    @Test fun decodesFriendAndUrlSymbolsFromSameImage() {
        val id = "yilong://friend/v1/0123456789abcdef0123456789abcdef"
        val first = qr(id); val second = qr("https://example.com")
        val bitmap = Bitmap.createBitmap(760, 360, Bitmap.Config.ARGB_8888)
        bitmap.eraseColor(Color.WHITE)
        for ((image, left) in listOf(first to 10, second to 420)) {
            val pixels = IntArray(320 * 320); image.getPixels(pixels, 0, 320, 0, 0, 320, 320)
            bitmap.setPixels(pixels, 0, 320, left, 10, 320, 320)
        }
        try { assertEquals(setOf(id, "https://example.com"), ScanImageDecoder.decode(bitmap).toSet()) }
        finally { bitmap.recycle(); first.recycle(); second.recycle() }
    }
    @Test fun blankImageReturnsNoResults() {
        val bitmap = Bitmap.createBitmap(320, 320, Bitmap.Config.ARGB_8888).apply { eraseColor(Color.WHITE) }
        try { assertTrue(ScanImageDecoder.decode(bitmap).isEmpty()) } finally { bitmap.recycle() }
    }
}
