package com.elon.app.chatgptweb

import com.elon.app.WebChatTextBlock
import com.elon.app.WebChatTextBlockDocx
import com.elon.app.WebChatTextBlockExport
import java.io.ByteArrayInputStream
import java.io.File
import java.util.zip.ZipInputStream
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.*
import org.junit.Test
import org.w3c.dom.Document
import org.w3c.dom.Element
import org.w3c.dom.Node

class WebChatTextBlockDocxTest {
    private val word = WebChatTextBlockDocx.WORD
    private val format = ChatGptWebCanvasExportFormats.find("document", "docx")!!
    private fun block() = WebChatTextBlock("sample", "writing", "Synthetic fixture", "", "", true)
    private fun bytes(content: String) = WebChatTextBlockExport.bytes(block(), content, format)
    private fun parts(bytes: ByteArray): Map<String, ByteArray> = linkedMapOf<String, ByteArray>().apply {
        ZipInputStream(ByteArrayInputStream(bytes)).use { zip ->
            while (true) {
                val entry = zip.nextEntry ?: break
                assertFalse(entry.isDirectory)
                assertNull(put(entry.name, zip.readBytes()))
            }
        }
    }
    private fun xml(bytes: ByteArray): Document = DocumentBuilderFactory.newInstance().apply {
        isNamespaceAware = true
        setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
    }.newDocumentBuilder().parse(ByteArrayInputStream(bytes))
    private fun doc(content: String) = xml(parts(bytes(content)).getValue("word/document.xml"))
    private fun Node.elements(name: String): List<Element> {
        val found = if (this is Document) getElementsByTagNameNS(word, name) else (this as Element).getElementsByTagNameNS(word, name)
        return (0 until found.length).map { found.item(it) as Element }
    }
    private fun Element.value() = getAttributeNS(word, "val")
    private fun Node.visible(): String = when (localName) {
        "t" -> textContent
        "tab" -> "\t"
        "br" -> "\n"
        else -> (0 until childNodes.length).joinToString("") { childNodes.item(it).visible() }
    }

    @Test fun exportIsARealSelfContainedWordPackageNotRenamedMarkdown() {
        val body = "# Document\n\nExport **checked**."
        val result = bytes(body)
        assertEquals("PK", result.take(2).toByteArray().toString(Charsets.US_ASCII))
        val entries = parts(result)
        assertEquals(setOf("[Content_Types].xml", "_rels/.rels", "word/document.xml", "word/styles.xml",
            "word/numbering.xml", "word/_rels/document.xml.rels"), entries.keys)
        entries.values.forEach { xml(it) }
        val types = xml(entries.getValue("[Content_Types].xml"))
        val overrides = types.getElementsByTagNameNS("http://schemas.openxmlformats.org/package/2006/content-types", "Override")
        for (index in 0 until overrides.length) {
            val part = overrides.item(index) as Element
            assertTrue(entries.containsKey(part.getAttribute("PartName").removePrefix("/")))
        }
        assertEquals(3, overrides.length)
        for (name in listOf("_rels/.rels", "word/_rels/document.xml.rels")) {
            val relations = xml(entries.getValue(name)).documentElement.childNodes
            for (index in 0 until relations.length) {
                val relation = relations.item(index) as? Element ?: continue
                assertFalse(relation.hasAttribute("TargetMode"))
                val prefix = if (name.startsWith("word/")) "word/" else ""
                assertTrue(entries.containsKey(prefix + relation.getAttribute("Target")))
            }
        }
        assertArrayEquals(result, bytes(body))
    }

    @Test fun headingsEmphasisUnicodeAndHardBreaksAreStructured() {
        val parsed = doc("# \u4E2D\u6587 \uD83D\uDE00\n\n**bold** *italic* ~~strike~~ `a < b & c`  \nnext")
        assertEquals(listOf("Heading1"), parsed.elements("pStyle").map { it.value() })
        assertTrue(parsed.elements("r").any { it.visible() == "bold" && it.elements("b").size == 1 })
        assertTrue(parsed.elements("r").any { it.visible() == "italic" && it.elements("i").size == 1 })
        assertTrue(parsed.elements("r").any { it.visible() == "strike" && it.elements("strike").size == 1 })
        assertTrue(parsed.visible().contains("\u4E2D\u6587 \uD83D\uDE00"))
        assertTrue(parsed.visible().contains("a < b & c\nnext"))
        assertFalse(parsed.visible().contains("**"))
    }

    @Test fun fencedCodePreservesWhitespaceTabsBlankLinesAndUnicode() {
        val code = "  one\n\t\u4E8C\n\n<&> \uD83D\uDE00\n"
        val parsed = doc("```python\n${code}```")
        assertEquals(code, parsed.visible())
        assertEquals(1, parsed.elements("tab").size)
        assertEquals(4, parsed.elements("br").size)
        assertEquals("Consolas", parsed.elements("rFonts").single().getAttributeNS(word, "ascii"))
        assertTrue(parsed.elements("t").all { it.getAttributeNS("http://www.w3.org/XML/1998/namespace", "space") == "preserve" })
    }

    @Test fun orderedAndNestedListsHaveIndependentNativeNumbering() {
        val content = "7. First\n8. Second\n   - Child\n\nParagraph\n\n3. Restart"
        val entries = parts(bytes(content))
        val parsed = xml(entries.getValue("word/document.xml"))
        val definitions = xml(entries.getValue("word/numbering.xml"))
        val markers = parsed.elements("numPr")
        assertEquals(4, markers.size)
        assertEquals(listOf("0", "0", "1", "0"), markers.map { it.elements("ilvl").single().value() })
        assertEquals(listOf("1", "1", "2", "3"), markers.map { it.elements("numId").single().value() })
        assertEquals(listOf("7", "1", "3"), definitions.elements("abstractNum").map { it.elements("start").first().value() })
        val children = definitions.documentElement.childNodes
        assertEquals(listOf("abstractNum", "abstractNum", "abstractNum", "num", "num", "num"),
            (0 until children.length).map { children.item(it).localName })
    }

    @Test fun tablesHaveCellsHeadersAlignmentAndStyledContent() {
        val parsed = doc("| Item | Value |\n| :--- | ---: |\n| **Alpha** | 12 |\n| Beta | 34 |")
        assertEquals(1, parsed.elements("tbl").size)
        assertEquals(3, parsed.elements("tr").size)
        assertEquals(6, parsed.elements("tc").size)
        assertEquals(1, parsed.elements("tblHeader").size)
        assertEquals(2, parsed.elements("gridCol").size)
        assertEquals(listOf("left", "right", "left", "right", "left", "right"), parsed.elements("jc").map { it.value() })
        assertEquals(listOf("Item", "Value", "Alpha", "12", "Beta", "34"), parsed.elements("tc").map { it.visible() })
    }

    @Test fun rawHtmlLinksAndImagesNeverBecomeExecutableOrExternalDocumentParts() {
        val content = "[link](https://example.invalid/a) ![alt](https://example.invalid/img.png)\n\n<script>alert('test')</script>"
        val entries = parts(bytes(content))
        val parsed = xml(entries.getValue("word/document.xml"))
        assertTrue(parsed.visible().contains("link (https://example.invalid/a)"))
        assertTrue(parsed.visible().contains("[alt] (https://example.invalid/img.png)"))
        assertTrue(parsed.visible().contains("<script>alert('test')</script>"))
        assertTrue(parsed.elements("hyperlink").isEmpty())
        assertTrue(parsed.elements("drawing").isEmpty())
        assertTrue(entries.keys.none { it.contains("media") || it.contains("vba") || it.contains("embeddings") })
    }

    @Test fun emptyAndMaximumSizeDocumentsAreExportable() {
        assertEquals(1, doc("").elements("p").size)
        val content = "a".repeat(WebChatTextBlock.MAX_CONTENT)
        assertEquals(content, doc(content).visible())
        assertThrows(IllegalArgumentException::class.java) { bytes(content + "b") }
    }

    @Test fun invalidXmlCharactersAndMalformedUnicodeFailWithoutChangingTextExports() {
        for (content in listOf("\u0000", "\u000B", "\uFFFF", "\uD800", "\uDC00", "a\uD800b"))
            assertThrows(IllegalArgumentException::class.java) { bytes(content) }
        assertArrayEquals(byteArrayOf(0), WebChatTextBlockExport.bytes("\u0000"))
        assertTrue(doc("\uFEFF\uD83D\uDE00").visible().contains("\uD83D\uDE00"))
    }

    @Test fun unsupportedOrIncompleteBlocksCannotBeExportedAsWord() {
        assertThrows(IllegalArgumentException::class.java) { WebChatTextBlockExport.bytes(block().copy(complete = false), "a", format) }
        assertThrows(IllegalArgumentException::class.java) { WebChatTextBlockExport.bytes(block().copy(kind = "code"), "a", format) }
        assertThrows(IllegalArgumentException::class.java) { WebChatTextBlockExport.bytes(block(), "a", format.copy(mediaType = "text/plain")) }
    }

    @Test fun pathologicallyDeepOrFragmentedDocumentsFailInsteadOfTruncating() {
        assertThrows(IllegalArgumentException::class.java) { bytes("> ".repeat(65) + "deep") }
        assertThrows(IllegalArgumentException::class.java) { bytes("a *b* ".repeat(11_000)) }
        assertThrows(IllegalArgumentException::class.java) { bytes("- item\n\nseparate\n\n".repeat(513)) }
    }

    @Test fun quotedCodeAndSeparatorsKeepValidParagraphPropertyOrder() {
        val parsed = doc("> ```text\n> code\n> ```\n>\n> ---")
        val order = listOf("pStyle", "numPr", "pBdr", "shd", "spacing", "ind")
        for (properties in parsed.elements("pPr")) {
            val children = properties.childNodes
            val indices = (0 until children.length).map { order.indexOf(children.item(it).localName) }
            assertEquals(indices.sorted(), indices)
            assertFalse(indices.contains(-1))
        }
    }

    @Test fun syntheticInteroperabilityFixtureUsesTheProductionEncoder() {
        val content = "# Writing Block export\n\n\u4E2D\u6587\u4E0E Unicode \uD83D\uDE00. **Bold** and *italic*.\n\n" +
            "3. First\n4. Second\n   - Nested\n\n> Quote\n\n" +
            "| Item | Count |\n| --- | ---: |\n| Alpha | 2 |\n\n" +
            "```python\ndef example():\n    return \"synthetic\"\n```\n\n[Reference](https://example.invalid)"
        File("build/test-artifacts/writing-block-export.docx").apply { requireNotNull(parentFile).mkdirs(); writeBytes(bytes(content)) }
        assertTrue(doc(content).visible().contains("synthetic"))
    }
}
