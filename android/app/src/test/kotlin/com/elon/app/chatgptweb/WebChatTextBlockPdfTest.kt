package com.elon.app.chatgptweb

import android.app.Application
import android.graphics.Typeface
import android.text.Spanned
import android.text.style.StrikethroughSpan
import android.text.style.StyleSpan
import android.text.style.TypefaceSpan
import com.elon.app.WebChatTextBlock
import com.elon.app.WebChatTextBlockExport
import com.elon.app.WebChatTextBlockPdf
import com.elon.app.WebChatTextBlockPdfContent
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], manifest = Config.NONE, application = Application::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class WebChatTextBlockPdfTest {
    private val format = ChatGptWebCanvasExportFormats.find("document", "pdf")!!
    private val block = WebChatTextBlock("fixture", "writing", "Synthetic PDF", "", "", true)
    private fun bytes(content: String) = WebChatTextBlockExport.bytes(block, content, format)

    @Test fun markdownKeepsStructureAndInlineStylesWithoutFetchingMedia() {
        val rows = WebChatTextBlockPdfContent.parse("# Title\n\n**Bold** *italic* ~~gone~~ `code`\n\n" +
            "3. First\n4. Second\n\n> Quote\n\n![alt](https://example.invalid/image)\n\n<script>blocked()</script>")
        assertTrue(rows.first().heading)
        val text = rows[1].cells.single() as Spanned
        assertEquals(setOf(Typeface.BOLD, Typeface.ITALIC), text.getSpans(0, text.length, StyleSpan::class.java).map { it.style }.toSet())
        assertEquals(1, text.getSpans(0, text.length, StrikethroughSpan::class.java).size)
        assertEquals(1, text.getSpans(0, text.length, TypefaceSpan::class.java).size)
        assertTrue(rows.any { it.cells.single().toString() == "3. First" })
        assertTrue(rows.any { it.cells.single().toString() == "4. Second" })
        assertTrue(rows.any { it.indent == 1 && it.cells.single().toString() == "Quote" })
        assertTrue(rows.any { it.cells.single().toString() == "[alt] (https://example.invalid/image)" })
        assertTrue(rows.last().code)
        assertTrue(rows.last().cells.single().contains("<script>"))
    }

    @Test fun codeAndUnicodeRemainWholeAndTablesRemainCells() {
        val source = "    value = \"\u4e2d\u6587\ud83d\ude00\"\n\n    return value\n"
        val rows = WebChatTextBlockPdfContent.parse("```python\n$source```\n\n| A | B |\n|---|---|\n| One | **Two** |")
        assertEquals(source, rows.first().cells.single().toString())
        assertTrue(rows.first().code)
        assertEquals(listOf("A", "B"), rows[1].cells.map { it.toString() })
        assertTrue(rows[1].heading)
        assertEquals(listOf("One", "Two"), rows[2].cells.map { it.toString() })
        assertTrue(rows[2].table)
    }

    @Test fun paginationCoversEveryWrappedLineExactlyOnce() {
        val content = (1..170).joinToString("\n") { "Line $it: \u4e2d\u6587 longword" + "x".repeat(60) }
        val layout = WebChatTextBlockPdf.layout(content, 11f, 130, false, false)
        var line = 0
        var end = 0
        var pages = 0
        while (line < layout.lineCount) {
            val part = WebChatTextBlockPdf.slice(layout, line, 160)
            assertTrue(part.end > line)
            assertEquals(end, layout.getLineStart(part.start))
            assertTrue(part.height <= 160)
            line = part.end
            end = layout.getLineEnd(line - 1)
            pages++
        }
        assertTrue(pages > 1)
        assertEquals(content.length, end)
        assertEquals(0, WebChatTextBlockPdf.slice(layout, 0, 0).height)
    }

    @Test fun unsupportedInputsCannotCreateMislabelledPdf() {
        assertThrows(Exception::class.java) { bytes("broken\ud800") }
        assertThrows(IllegalArgumentException::class.java) { bytes("a".repeat(WebChatTextBlock.MAX_CONTENT + 1)) }
        assertThrows(IllegalArgumentException::class.java) { WebChatTextBlockExport.bytes(block.copy(complete = false), "x", format) }
        assertThrows(IllegalArgumentException::class.java) { WebChatTextBlockExport.bytes(block.copy(kind = "code"), "x", format) }
        assertThrows(IllegalArgumentException::class.java) { WebChatTextBlockExport.bytes(block, "x", format.copy(mediaType = "text/plain")) }
        assertThrows(IllegalArgumentException::class.java) { WebChatTextBlockPdfContent.parse("> ".repeat(70) + "nested") }
    }
}
