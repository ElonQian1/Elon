package com.elon.app

import android.graphics.Typeface
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.StrikethroughSpan
import android.text.style.StyleSpan
import android.text.style.TypefaceSpan
import org.commonmark.ext.gfm.strikethrough.Strikethrough
import org.commonmark.ext.gfm.strikethrough.StrikethroughExtension
import org.commonmark.ext.gfm.tables.*
import org.commonmark.node.*
import org.commonmark.parser.Parser

/** PDF presentation of the existing CommonMark dialect; HTML and remote media stay inert. */
internal object WebChatTextBlockPdfContent {
    data class Block(val cells: List<CharSequence>, val size: Float = 11f, val indent: Int = 0,
        val code: Boolean = false, val table: Boolean = false, val heading: Boolean = false,
        val rule: Boolean = false)

    fun parse(content: String): List<Block> {
        WebChatTextBlockExport.bytes(content) // Same strict Unicode and input-size policy as text export.
        val root = Parser.builder().extensions(listOf(TablesExtension.create(), StrikethroughExtension.create()))
            .build().parse(content)
        val queue = java.util.ArrayDeque<Pair<Node, Int>>()
        queue.add(root to 0)
        var count = 0
        while (queue.isNotEmpty()) {
            val (node, depth) = queue.removeLast()
            require(++count <= 30_000 && depth <= 64) { "Document structure exceeds export limit" }
            children(node) { queue.add(it to depth + 1) }
        }
        val output = mutableListOf<Block>()
        children(root) { block(it, output, 0) }
        return output.ifEmpty { listOf(Block(listOf(""))) }
    }

    private fun children(node: Node, action: (Node) -> Unit) {
        var child = node.firstChild
        while (child != null) { action(child); child = child.next }
    }

    private fun block(node: Node, output: MutableList<Block>, indent: Int) {
        when (node) {
            is Paragraph, is Heading -> output.add(Block(listOf(text(node)),
                size = (node as? Heading)?.let { 25f - it.level * 2f } ?: 11f,
                indent = indent, heading = node is Heading))
            is FencedCodeBlock -> output.add(Block(listOf(node.literal), 10f, indent, code = true))
            is IndentedCodeBlock -> output.add(Block(listOf(node.literal), 10f, indent, code = true))
            is HtmlBlock -> output.add(Block(listOf(node.literal), 10f, indent, code = true))
            is BlockQuote -> children(node) { block(it, output, indent + 1) }
            is BulletList, is OrderedList -> {
                var number = (node as? OrderedList)?.startNumber ?: 1
                children(node) { item ->
                    require(item is ListItem)
                    val start = output.size
                    children(item) { block(it, output, indent + 1) }
                    val marker = if (node is OrderedList) "${number++}. " else "\u2022 "
                    if (output.size == start || output[start].table || output[start].rule) {
                        output.add(start, Block(listOf(marker), indent = indent + 1))
                    } else {
                        val first = output[start]
                        output[start] = first.copy(cells = listOf(SpannableStringBuilder(marker).append(first.cells.single())))
                    }
                }
            }
            is TableBlock -> children(node) { group ->
                require(group is TableHead || group is TableBody)
                children(group) { row ->
                    require(row is TableRow)
                    val cells = mutableListOf<CharSequence>()
                    children(row) { cell -> require(cell is TableCell); cells.add(text(cell)) }
                    require(cells.size in 1..32) { "Table width exceeds export limit" }
                    output.add(Block(cells, indent = indent, table = true, heading = group is TableHead))
                }
            }
            is ThematicBreak -> output.add(Block(listOf(""), indent = indent, rule = true))
            else -> throw IllegalArgumentException("Unsupported PDF document block")
        }
    }

    private fun text(node: Node): CharSequence = SpannableStringBuilder().apply {
        children(node) { inline(it, this) }
    }

    private fun inline(node: Node, output: SpannableStringBuilder) {
        val start = output.length
        when (node) {
            is Text -> output.append(node.literal)
            is Code -> output.append(node.literal)
            is SoftLineBreak -> output.append(' ')
            is HardLineBreak -> output.append('\n')
            is HtmlInline -> output.append(node.literal)
            is Link -> { children(node) { inline(it, output) }; output.append(" (${node.destination})") }
            is Image -> {
                output.append('[')
                children(node) { inline(it, output) }
                output.append("] (${node.destination})")
            }
            is Emphasis, is StrongEmphasis, is Strikethrough -> children(node) { inline(it, output) }
            else -> throw IllegalArgumentException("Unsupported PDF document inline")
        }
        val span = when (node) {
            is Code -> TypefaceSpan("monospace")
            is Emphasis -> StyleSpan(Typeface.ITALIC)
            is StrongEmphasis -> StyleSpan(Typeface.BOLD)
            is Strikethrough -> StrikethroughSpan()
            else -> null
        }
        if (span != null && output.length > start) output.setSpan(span, start, output.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
    }
}
