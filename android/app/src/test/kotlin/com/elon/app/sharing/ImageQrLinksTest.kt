package com.elon.app.sharing

import android.graphics.Bitmap
import com.google.zxing.BarcodeFormat
import com.google.zxing.qrcode.QRCodeWriter
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], application = android.app.Application::class, manifest = Config.NONE)
class ImageQrLinksTest {
    @Test fun findsMultipleWebLinksAndIgnoresNonWebQr() {
        val urls = listOf("https://example.com/a?scene=90", "https://example.com/b", "WIFI:T:WPA;S:fixture;;")
        val image = Bitmap.createBitmap(1000, 340, Bitmap.Config.ARGB_8888)
        image.eraseColor(-1)
        urls.forEachIndexed { index, url ->
            val matrix = QRCodeWriter().encode(url, BarcodeFormat.QR_CODE, 300, 300)
            for (y in 0 until 300) for (x in 0 until 300) image.setPixel(index * 330 + x, y + 20, if (matrix[x, y]) android.graphics.Color.BLACK else android.graphics.Color.WHITE)
        }
        assertEquals(urls.take(2).toSet(), ImageQrLinks.decode(image).map { it.url }.toSet())
        image.recycle()
    }
}
