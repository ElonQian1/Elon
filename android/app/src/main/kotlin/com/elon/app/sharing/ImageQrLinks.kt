package com.elon.app.sharing

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.RGBLuminanceSource
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.multi.qrcode.QRCodeMultiReader
import java.io.File

internal object ImageQrLinks {
    fun decode(file: File): List<SourceLink> {
        if (!file.isFile || file.length() > 12 * 1024 * 1024) return emptyList()
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(file.path, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return emptyList()
        val options = BitmapFactory.Options().apply {
            while (bounds.outWidth.toLong() * bounds.outHeight / (inSampleSize.coerceAtLeast(1).toLong() * inSampleSize.coerceAtLeast(1)) > 8_000_000) inSampleSize = inSampleSize.coerceAtLeast(1) * 2
        }
        val bitmap = BitmapFactory.decodeFile(file.path, options) ?: return emptyList()
        return try { decode(bitmap) } finally { bitmap.recycle() }
    }
    fun decode(bitmap: Bitmap): List<SourceLink> = runCatching {
        val pixels = IntArray(bitmap.width * bitmap.height)
        bitmap.getPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)
        val source = RGBLuminanceSource(bitmap.width, bitmap.height, pixels)
        val hints = mapOf(DecodeHintType.TRY_HARDER to true)
        val decoded = runCatching { QRCodeMultiReader().decodeMultiple(BinaryBitmap(HybridBinarizer(source)), hints).toList() }
            .getOrElse { runCatching { listOf(com.google.zxing.qrcode.QRCodeReader().decode(BinaryBitmap(HybridBinarizer(source)), hints)) }.getOrDefault(emptyList()) }
        decoded.mapNotNull { SourceLink.webUrl(it.text)?.let(::SourceLink) }.distinctBy { it.url }.take(8)
    }.getOrDefault(emptyList())
}
