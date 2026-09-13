package com.elon.app

import java.io.ByteArrayOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import javax.xml.parsers.DocumentBuilderFactory
import javax.xml.transform.OutputKeys
import javax.xml.transform.TransformerFactory
import javax.xml.transform.dom.DOMSource
import javax.xml.transform.stream.StreamResult
import org.w3c.dom.Document
import org.w3c.dom.Element

/** A local document copy. It never invokes the provider's Canvas export or cloud save. */
internal object WebChatTextBlockDocx {
    const val WORD = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
    private const val RELS = "http://schemas.openxmlformats.org/package/2006/relationships"
    private const val TYPES = "http://schemas.openxmlformats.org/package/2006/content-types"
    private const val OFFICE = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"

    fun bytes(content: String): ByteArray {
        require(content.length <= WebChatTextBlock.MAX_CONTENT)
        var offset = 0
        while (offset < content.length) {
            val point = content.codePointAt(offset)
            require(point == 9 || point == 10 || point == 13 || point in 0x20..0xD7FF ||
                point in 0xE000..0xFFFD || point in 0x10000..0x10FFFF) { "Invalid document character" }
            offset += Character.charCount(point)
        }
        val document = document(WORD, "w:document")
        val numbering = document(WORD, "w:numbering")
        val body = document.documentElement.word("body")
        WebChatTextBlockMarkdown(body, numbering.documentElement).render(content)
        body.word("sectPr").apply {
            word("pgSz", "w" to "11906", "h" to "16838")
            word("pgMar", "top" to "1440", "right" to "1440", "bottom" to "1440", "left" to "1440")
        }
        val parts = linkedMapOf(
            "[Content_Types].xml" to contentTypes(),
            "_rels/.rels" to relationships(listOf("officeDocument" to "word/document.xml")),
            "word/document.xml" to document,
            "word/styles.xml" to styles(),
            "word/numbering.xml" to numbering,
            "word/_rels/document.xml.rels" to relationships(listOf("styles" to "styles.xml", "numbering" to "numbering.xml")),
        )
        return ByteArrayOutputStream().use { output ->
            ZipOutputStream(output).use { zip ->
                for ((path, xml) in parts) {
                    zip.putNextEntry(ZipEntry(path).apply { time = 0 })
                    val transformer = TransformerFactory.newInstance().newTransformer().apply {
                        setOutputProperty(OutputKeys.ENCODING, "UTF-8")
                        setOutputProperty(OutputKeys.STANDALONE, "yes")
                    }
                    transformer.transform(DOMSource(xml), StreamResult(zip))
                    zip.closeEntry()
                }
            }
            output.toByteArray()
        }
    }

    private fun document(namespace: String, name: String): Document =
        DocumentBuilderFactory.newInstance().newDocumentBuilder().newDocument().apply {
            appendChild(createElementNS(namespace, name))
        }

    private fun contentTypes(): Document = document(TYPES, "Types").apply {
        fun part(tag: String, vararg attributes: Pair<String, String>) {
            documentElement.appendChild(createElementNS(TYPES, tag).apply {
                attributes.forEach { (key, value) -> setAttribute(key, value) }
            })
        }
        part("Default", "Extension" to "rels", "ContentType" to "application/vnd.openxmlformats-package.relationships+xml")
        part("Default", "Extension" to "xml", "ContentType" to "application/xml")
        for ((file, type) in listOf("document" to "document.main", "styles" to "styles", "numbering" to "numbering"))
            part("Override", "PartName" to "/word/$file.xml", "ContentType" to "application/vnd.openxmlformats-officedocument.wordprocessingml.$type+xml")
    }

    private fun relationships(entries: List<Pair<String, String>>): Document = document(RELS, "Relationships").apply {
        entries.forEachIndexed { index, (type, target) ->
            documentElement.appendChild(createElementNS(RELS, "Relationship").apply {
                setAttribute("Id", "rId${index + 1}")
                setAttribute("Type", OFFICE + type)
                setAttribute("Target", target)
            })
        }
    }

    private fun styles(): Document = document(WORD, "w:styles").apply {
        val defaults = documentElement.word("docDefaults")
        defaults.word("rPrDefault").word("rPr").apply {
            word("rFonts", "ascii" to "Calibri", "hAnsi" to "Calibri", "eastAsia" to "Microsoft YaHei")
            word("sz", "val" to "22")
            word("szCs", "val" to "22")
        }
        defaults.word("pPrDefault").word("pPr").word("spacing", "after" to "120", "line" to "276", "lineRule" to "auto")
        documentElement.word("style", "type" to "paragraph", "default" to "1", "styleId" to "Normal").word("name", "val" to "Normal")
        for (level in 1..6) {
            documentElement.word("style", "type" to "paragraph", "styleId" to "Heading$level").apply {
                word("name", "val" to "heading $level")
                word("basedOn", "val" to "Normal")
                word("next", "val" to "Normal")
                word("qFormat")
                word("pPr").apply {
                    word("keepNext")
                    word("spacing", "before" to "240", "after" to "120")
                    word("outlineLvl", "val" to "${level - 1}")
                }
                word("rPr").apply { word("b"); word("sz", "val" to "${36 - level * 2}") }
            }
        }
    }

    internal fun Element.word(name: String, vararg attributes: Pair<String, String>): Element =
        ownerDocument.createElementNS(WORD, "w:$name").also { element ->
            attributes.forEach { (key, value) -> element.setAttributeNS(WORD, "w:$key", value) }
            appendChild(element)
        }
}
