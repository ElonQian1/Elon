package com.elon.app.scan

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.MultiFormatReader
import com.google.zxing.RGBLuminanceSource
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.multi.GenericMultipleBarcodeReader
import java.io.ByteArrayOutputStream

internal object ScanImageDecoder {
    fun decode(context: Context, uri: Uri): List<String> {
        val bytes = context.contentResolver.openInputStream(uri)?.use { stream ->
            val output = ByteArrayOutputStream(); val buffer = ByteArray(16384)
            while (true) {
                val read = stream.read(buffer); if (read < 0) break
                require(output.size() + read <= 12 * 1024 * 1024) { "请选择不超过 12 MB 的图片" }
                output.write(buffer, 0, read)
            }
            output.toByteArray()
        } ?: error("无法读取图片，请重新选择")
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        require(bounds.outWidth > 0 && bounds.outHeight > 0) { "图片格式无法读取" }
        val options = BitmapFactory.Options().apply {
            inSampleSize = 1
            while (bounds.outWidth.toLong() * bounds.outHeight / (inSampleSize.toLong() * inSampleSize) > 10_000_000) inSampleSize *= 2
        }
        val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options) ?: error("图片无法解码")
        return try { decode(bitmap) } finally { bitmap.recycle() }
    }
    fun decode(bitmap: Bitmap): List<String> {
        val pixels = IntArray(bitmap.width * bitmap.height)
        bitmap.getPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)
        val source = RGBLuminanceSource(bitmap.width, bitmap.height, pixels)
        val hints = mapOf(DecodeHintType.TRY_HARDER to true, DecodeHintType.CHARACTER_SET to "UTF-8", DecodeHintType.ALSO_INVERTED to true)
        val binary = BinaryBitmap(HybridBinarizer(source))
        val reader = MultiFormatReader()
        val results = runCatching { GenericMultipleBarcodeReader(reader).decodeMultiple(binary, hints).toList() }
            .getOrElse { runCatching { listOf(reader.decode(binary, hints)) }.getOrDefault(emptyList()) }
        return results.map { it.text }.filter { it.isNotBlank() && it.length <= 8192 }.distinct().take(8)
    }
}
