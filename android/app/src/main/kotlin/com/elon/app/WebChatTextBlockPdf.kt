package com.elon.app

import android.graphics.Color
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.pdf.PdfDocument
import android.os.Build
import android.text.Layout
import android.text.StaticLayout
import android.text.TextDirectionHeuristics
import android.text.TextPaint
import java.io.ByteArrayOutputStream
import java.io.OutputStream

/** Text/vector PDF, not a screenshot or renamed source file. No WebView or network is used. */
internal object WebChatTextBlockPdf {
    private const val MARGIN = 42
    private const val PADDING = 5
    private const val MAX_PAGES = 300
    private const val MAX_BYTES = 16 * 1024 * 1024

    internal data class Slice(val start: Int, val end: Int, val top: Int, val height: Int)

    internal fun slice(layout: StaticLayout, start: Int, available: Int): Slice {
        require(start in 0..layout.lineCount)
        val top = layout.getLineTop(start)
        var end = start
        while (end < layout.lineCount && layout.getLineTop(end + 1) - top <= available) end++
        return Slice(start, end, top, layout.getLineTop(end) - top)
    }

    internal fun layout(text: CharSequence, size: Float, width: Int, code: Boolean, bold: Boolean): StaticLayout {
        val paint = TextPaint(Paint.ANTI_ALIAS_FLAG).apply {
            textSize = size
            color = Color.BLACK
            density = 1f
            typeface = Typeface.create(if (code) "monospace" else "sans-serif", if (bold) Typeface.BOLD else Typeface.NORMAL)
        }
        return StaticLayout.Builder.obtain(text, 0, text.length, paint, width)
            .setAlignment(Layout.Alignment.ALIGN_NORMAL).setIncludePad(true)
            .setTextDirection(TextDirectionHeuristics.FIRSTSTRONG_LTR)
            .setBreakStrategy(Layout.BREAK_STRATEGY_SIMPLE).setHyphenationFrequency(Layout.HYPHENATION_FREQUENCY_NONE)
            .setLineSpacing(2f, 1f)
            .apply { if (Build.VERSION.SDK_INT >= 28) setUseLineSpacingFromFallbacks(true) }.build()
    }

    fun bytes(content: String): ByteArray {
        val blocks = WebChatTextBlockPdfContent.parse(content)
        val landscape = blocks.any { it.cells.size > 5 }
        val width = if (landscape) 842 else 595
        val height = if (landscape) 595 else 842
        val bottom = height - MARGIN
        val document = PdfDocument()
        var page: PdfDocument.Page? = null
        var number = 0
        var y = MARGIN
        val decoration = Paint(Paint.ANTI_ALIAS_FLAG)

        fun finishPage() {
            page?.let {
                decoration.apply { color = Color.DKGRAY; textSize = 9f; style = Paint.Style.FILL; textAlign = Paint.Align.CENTER }
                it.canvas.drawText("$number", width / 2f, height - 20f, decoration)
                document.finishPage(it)
                page = null
            }
        }
        fun nextPage() {
            finishPage()
            require(++number <= MAX_PAGES) { "PDF page count exceeds export limit" }
            page = document.startPage(PdfDocument.PageInfo.Builder(width, height, number).create())
            page!!.canvas.drawColor(Color.WHITE)
            y = MARGIN
        }

        try {
            nextPage()
            for (block in blocks) {
                val left = MARGIN + if (block.table) 0 else (block.indent * 14).coerceAtMost((width - MARGIN * 2) / 3)
                val cellWidth = (width - MARGIN - left) / block.cells.size
                if (block.rule) {
                    if (y + 12 > bottom) nextPage()
                    decoration.apply { color = Color.GRAY; strokeWidth = 0.5f; style = Paint.Style.STROKE }
                    page!!.canvas.drawLine(left.toFloat(), y + 4f, (width - MARGIN).toFloat(), y + 4f, decoration)
                    y += 12
                    continue
                }
                val layouts = block.cells.map { layout(it, block.size, cellWidth - 2 * PADDING, block.code, block.heading) }
                val offsets = IntArray(layouts.size)
                while (offsets.indices.any { offsets[it] < layouts[it].lineCount }) {
                    val available = bottom - y - PADDING * 2
                    val slices = layouts.mapIndexed { i, layout -> slice(layout, offsets[i], available) }
                    if (slices.indices.any { offsets[it] < layouts[it].lineCount && slices[it].height == 0 }) {
                        require(y != MARGIN) { "PDF line exceeds page height" }
                        nextPage()
                        continue
                    }
                    val used = slices.maxOf { it.height } + PADDING * 2
                    val canvas = page!!.canvas
                    slices.forEachIndexed { i, part ->
                        val x = left + cellWidth * i
                        if (block.code || block.table) {
                            decoration.apply { color = if (block.code || block.heading) 0xfff2f2f2.toInt() else Color.WHITE; style = Paint.Style.FILL }
                            canvas.drawRect(x.toFloat(), y.toFloat(), (x + cellWidth).toFloat(), (y + used).toFloat(), decoration)
                        }
                        if (block.table) {
                            decoration.apply { color = Color.LTGRAY; strokeWidth = 0.5f; style = Paint.Style.STROKE }
                            canvas.drawRect(x.toFloat(), y.toFloat(), (x + cellWidth).toFloat(), (y + used).toFloat(), decoration)
                        }
                        if (part.end > part.start) {
                            val save = canvas.save()
                            canvas.clipRect(x + PADDING, y + PADDING, x + cellWidth - PADDING, y + PADDING + part.height)
                            canvas.translate((x + PADDING).toFloat(), (y + PADDING - part.top).toFloat())
                            layouts[i].draw(canvas)
                            canvas.restoreToCount(save)
                        }
                        offsets[i] = part.end
                    }
                    y += used
                    if (offsets.indices.any { offsets[it] < layouts[it].lineCount }) nextPage()
                }
                y += if (block.table) 0 else 5
            }
            finishPage()
            return ByteArrayOutputStream().use { bytes ->
                document.writeTo(object : OutputStream() {
                    override fun write(value: Int) {
                        require(bytes.size() < MAX_BYTES) { "PDF size exceeds export limit" }
                        bytes.write(value)
                    }
                    override fun write(buffer: ByteArray, offset: Int, count: Int) {
                        require(count <= MAX_BYTES - bytes.size()) { "PDF size exceeds export limit" }
                        bytes.write(buffer, offset, count)
                    }
                })
                bytes.toByteArray()
            }
        } finally {
            finishPage()
            document.close()
        }
    }
}
