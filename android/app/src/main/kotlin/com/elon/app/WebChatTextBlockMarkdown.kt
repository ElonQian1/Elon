package com.elon.app

import com.elon.app.WebChatTextBlockDocx.word
import org.commonmark.ext.gfm.strikethrough.Strikethrough
import org.commonmark.ext.gfm.strikethrough.StrikethroughExtension
import org.commonmark.ext.gfm.tables.*
import org.commonmark.node.*
import org.commonmark.parser.Parser
import org.w3c.dom.Element

/** CommonMark is shared with the chat renderer; export never interprets HTML or fetches images. */
internal class WebChatTextBlockMarkdown(private val body: Element, private val numbering: Element) {
    private data class Style(val bold: Boolean = false, val italic: Boolean = false,
        val strike: Boolean = false, val code: Boolean = false)
    private data class ListScope(val id: Int, val level: Int, var first: Boolean = true)
    private var nodes = 0
    private var lists = 0

    fun render(content: String) {
        val parser = Parser.builder().extensions(listOf(TablesExtension.create(), StrikethroughExtension.create())).build()
        val root = parser.parse(content)
        // Bound both shape and depth before recursive rendering (including unsupported nodes).
        val queue = java.util.ArrayDeque<Pair<Node, Int>>()
        queue.add(root to 0)
        while (queue.isNotEmpty()) {
            val (node, depth) = queue.removeLast()
            require(++nodes <= 30_000 && depth <= 64) { "Document structure exceeds export limit" }
            var child = node.firstChild
            while (child != null) { queue.add(child to depth + 1); child = child.next }
        }
        children(root) { block(it, body, 0, null) }
        if (!body.hasChildNodes()) body.word("p")
    }

    private fun children(node: Node, action: (Node) -> Unit) {
        var child = node.firstChild
        while (child != null) { action(child); child = child.next }
    }

    private fun block(node: Node, target: Element, quote: Int, scope: ListScope?) {
        when (node) {
            is Paragraph, is Heading -> {
                val paragraph = paragraph(target, quote, scope, (node as? Heading)?.level)
                children(node) { inline(it, paragraph, Style()) }
            }
            is BlockQuote -> children(node) { block(it, target, quote + 1, scope) }
            is BulletList -> list(node, target, quote, scope, false, 1)
            is OrderedList -> list(node, target, quote, scope, true, node.startNumber)
            is FencedCodeBlock -> code(target, node.literal, quote, scope)
            is IndentedCodeBlock -> code(target, node.literal, quote, scope)
            is HtmlBlock -> run(paragraph(target, quote, scope), node.literal, Style(code = true))
            is ThematicBreak -> paragraph(target, quote, scope).apply {
                (firstChild as Element).apply {
                    val border = word("pBdr")
                    border.word("bottom", "val" to "single", "sz" to "4", "color" to "B0B0B0")
                    val indent = getElementsByTagNameNS(WebChatTextBlockDocx.WORD, "ind").item(0)
                    if (indent != null) insertBefore(border, indent)
                }
            }
            is TableBlock -> {
                // An empty paragraph is the list marker when a table is the first item child.
                if (scope?.first == true) paragraph(target, quote, scope)
                table(node, target)
            }
            else -> throw IllegalArgumentException("Unsupported document block")
        }
    }

    private fun paragraph(target: Element, quote: Int, scope: ListScope?, heading: Int? = null): Element = target.word("p").apply {
        word("pPr").apply {
            if (heading != null) word("pStyle", "val" to "Heading$heading")
            val marker = scope?.first == true
            if (marker) word("numPr").apply {
                word("ilvl", "val" to "${scope!!.level}")
                word("numId", "val" to "${scope.id}")
                scope.first = false
            }
            val indent = quote * 360 + (scope?.let { (it.level + 1) * 360 } ?: 0)
            if (indent > 0) {
                if (marker) word("ind", "left" to "$indent", "hanging" to "240")
                else word("ind", "left" to "$indent")
            }
        }
    }

    private fun list(node: Node, target: Element, quote: Int, parent: ListScope?, ordered: Boolean, start: Int) {
        if (parent?.first == true) paragraph(target, quote, parent)
        val level = parent?.let { it.level + 1 } ?: 0
        require(level <= 8) { "List nesting exceeds Word limit" }
        val id = ++lists
        require(id <= 512) { "List count exceeds export limit" }
        // Each list has its own definition so ordered starts and nested restarts remain independent.
        val definition = numbering.word("abstractNum", "abstractNumId" to "$id")
        definition.word("multiLevelType", "val" to "multilevel")
        for (depth in 0..8) definition.word("lvl", "ilvl" to "$depth").apply {
            word("start", "val" to if (depth == level && ordered) "$start" else "1")
            word("numFmt", "val" to if (ordered) "decimal" else "bullet")
            word("lvlText", "val" to if (ordered) "%${depth + 1}." else "\u2022")
            word("lvlJc", "val" to "left")
            word("pPr").word("ind", "left" to "${(depth + 1) * 360}", "hanging" to "240")
        }
        // Definitions must precede instances in the OOXML sequence.
        val firstInstance = numbering.getElementsByTagNameNS(WebChatTextBlockDocx.WORD, "num").item(0)
        if (firstInstance != null) numbering.insertBefore(definition, firstInstance)
        numbering.word("num", "numId" to "$id").word("abstractNumId", "val" to "$id")
        children(node) { item ->
            require(item is ListItem)
            val current = ListScope(id, level)
            children(item) { block(it, target, quote, current) }
            if (current.first) paragraph(target, quote, current)
        }
    }

    private fun code(target: Element, literal: String, quote: Int, scope: ListScope?) {
        val paragraph = paragraph(target, quote, scope)
        (paragraph.firstChild as Element).apply {
            val indent = getElementsByTagNameNS(WebChatTextBlockDocx.WORD, "ind").item(0)
            val shading = word("shd", "val" to "clear", "fill" to "F2F2F2")
            val spacing = word("spacing", "after" to "120", "line" to "240", "lineRule" to "auto")
            if (indent != null) { insertBefore(shading, indent); insertBefore(spacing, indent) }
        }
        run(paragraph, literal, Style(code = true))
    }

    private fun table(node: TableBlock, target: Element) {
        val rows = mutableListOf<TableRow>()
        children(node) { group ->
            require(group is TableHead || group is TableBody)
            children(group) { require(it is TableRow); rows.add(it) }
        }
        val columns = rows.maxOfOrNull { row -> var count = 0; children(row) { count++ }; count } ?: 0
        require(columns in 1..32)
        val table = target.word("tbl")
        table.word("tblPr").apply {
            word("tblW", "w" to "0", "type" to "auto")
            word("tblBorders").apply {
                for (edge in listOf("top", "left", "bottom", "right", "insideH", "insideV"))
                    word(edge, "val" to "single", "sz" to "4", "color" to "B0B0B0")
            }
        }
        table.word("tblGrid").apply { repeat(columns) { word("gridCol", "w" to "${9026 / columns}") } }
        for (row in rows) {
            val tr = table.word("tr")
            if (row.parent is TableHead) tr.word("trPr").word("tblHeader")
            children(row) { cell ->
                require(cell is TableCell)
                val tc = tr.word("tc")
                tc.word("tcPr").word("tcW", "w" to "${9026 / columns}", "type" to "dxa")
                val p = tc.word("p")
                if (cell.alignment != null) p.word("pPr").word("jc", "val" to cell.alignment.name.lowercase())
                children(cell) { inline(it, p, Style(bold = cell.isHeader)) }
            }
        }
    }

    private fun inline(node: Node, paragraph: Element, style: Style) {
        when (node) {
            is Text -> run(paragraph, node.literal, style)
            is Code -> run(paragraph, node.literal, style.copy(code = true))
            is Emphasis -> children(node) { inline(it, paragraph, style.copy(italic = true)) }
            is StrongEmphasis -> children(node) { inline(it, paragraph, style.copy(bold = true)) }
            is Strikethrough -> children(node) { inline(it, paragraph, style.copy(strike = true)) }
            is SoftLineBreak -> run(paragraph, " ", style)
            is HardLineBreak -> run(paragraph, "\n", style)
            is HtmlInline -> run(paragraph, node.literal, style)
            is Link -> {
                children(node) { inline(it, paragraph, style) }
                run(paragraph, " (${node.destination})", style)
            }
            is Image -> {
                run(paragraph, "[", style)
                children(node) { inline(it, paragraph, style) }
                run(paragraph, "] (${node.destination})", style)
            }
            else -> throw IllegalArgumentException("Unsupported document inline")
        }
    }

    private fun run(paragraph: Element, text: String, style: Style) {
        if (text.isEmpty()) return
        paragraph.word("r").apply {
            if (style != Style()) word("rPr").apply {
                if (style.code) word("rFonts", "ascii" to "Consolas", "hAnsi" to "Consolas", "eastAsia" to "Microsoft YaHei")
                if (style.bold) word("b")
                if (style.italic) word("i")
                if (style.strike) word("strike")
            }
            var start = 0
            var index = 0
            fun textPart(end: Int) {
                if (end > start) word("t").apply {
                    setAttributeNS("http://www.w3.org/XML/1998/namespace", "xml:space", "preserve")
                    textContent = text.substring(start, end)
                }
            }
            while (index < text.length) {
                when (text[index]) {
                    '\t', '\r', '\n' -> {
                        textPart(index)
                        word(if (text[index] == '\t') "tab" else "br")
                        if (text[index] == '\r' && text.getOrNull(index + 1) == '\n') index++
                        start = index + 1
                    }
                }
                index++
            }
            textPart(text.length)
        }
    }
}
