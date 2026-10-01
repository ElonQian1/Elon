package com.elon.app

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.pdf.PdfDocument
import java.io.File

/** Pinned, non-user media for the authenticated local acceptance command. */
internal object ChatGptWebAcceptanceMediaFixture {
    fun write(spec: ChatGptWebAcceptanceAttachmentFixture.Spec, target: File) {
        when (spec.name) {
            ChatGptWebAcceptanceAttachmentFixture.IMAGE_NAME -> writeImage(target)
            ChatGptWebAcceptanceAttachmentFixture.IMAGE_FIDELITY_NAME -> writeFidelityImage(target)
            ChatGptWebAcceptanceAttachmentFixture.PDF_NAME -> writePdf(target)
            else -> error("Unknown media fixture")
        }
    }

    private fun writeImage(target: File) {
        val bitmap = Bitmap.createBitmap(512, 384, Bitmap.Config.ARGB_8888)
        try {
            val canvas = Canvas(bitmap)
            canvas.drawColor(Color.WHITE)
            val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.BLUE }
            for (left in listOf(64f, 216f, 368f)) canvas.drawRect(left, 56f, left + 80f, 136f, paint)
            paint.color = Color.RED
            canvas.drawCircle(256f, 272f, 64f, paint)
            target.outputStream().use { require(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
        } finally {
            bitmap.recycle()
        }
    }

    private fun writePdf(target: File) {
        val document = PdfDocument()
        try {
            val page = document.startPage(PdfDocument.PageInfo.Builder(612, 792, 1).create())
            val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.BLACK; textSize = 16f }
            page.canvas.drawText("ELON_PRIVATE_PDF_FIXTURE_V1=ready", 42f, 60f, paint)
            page.canvas.drawText("This is a fixed acceptance document, not user data.", 42f, 90f, paint)
            document.finishPage(page)
            target.outputStream().use(document::writeTo)
        } finally {
            document.close()
        }
    }

    private fun writeFidelityImage(target: File) {
        val bitmap = Bitmap.createBitmap(1080, 6000, Bitmap.Config.ARGB_8888)
        try {
            val canvas = Canvas(bitmap)
            canvas.drawColor(Color.WHITE)
            val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.BLACK; textSize = 24f }
            for (top in 0 until 6000 step 300) {
                canvas.drawText("IMAGE FIDELITY 1080 x 6000 / ROW $top", 32f, top + 60f, paint)
                for (x in 32..1048 step 4) canvas.drawRect(x.toFloat(), top + 100f, x + 1f, top + 180f, paint)
            }
            target.outputStream().use { require(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
        } finally { bitmap.recycle() }
    }
}
