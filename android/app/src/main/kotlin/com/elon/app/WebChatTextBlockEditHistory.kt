package com.elon.app

internal class WebChatTextBlockEditHistory(
    private val maxEntries: Int = 100,
    private val maxCharacters: Int = WebChatTextBlock.MAX_CONTENT * 4,
) {
    data class Change(val start: Int, val before: String, val after: String) {
        val size: Int get() = before.length + after.length
        fun reversed() = Change(start, after, before)
        fun matches(text: String) = start >= 0 && start <= text.length &&
            before.length <= text.length - start && text.regionMatches(start, before, 0, before.length)
    }

    private val past = ArrayDeque<Change>()
    private val future = ArrayDeque<Change>()
    private var characters = 0
    val canUndo: Boolean get() = past.isNotEmpty()
    val canRedo: Boolean get() = future.isNotEmpty()

    init { require(maxEntries > 0 && maxCharacters > 0) }

    // Keep deltas, not a full document copy for every keystroke.
    fun record(start: Int, before: String, after: String) {
        require(start >= 0)
        if (before == after) return
        characters -= future.sumOf { it.size }
        future.clear()
        val change = Change(start, before, after)
        if (change.size > maxCharacters) { clear(); return }
        past.addLast(change)
        characters += change.size
        while (past.size > maxEntries || characters > maxCharacters) characters -= past.removeFirst().size
    }

    fun undo(text: String): Change? = move(past, future, text, reverse = true)
    fun redo(text: String): Change? = move(future, past, text, reverse = false)

    private fun move(from: ArrayDeque<Change>, to: ArrayDeque<Change>, text: String, reverse: Boolean): Change? {
        val entry = from.lastOrNull() ?: return null
        val change = if (reverse) entry.reversed() else entry
        if (!change.matches(text)) { clear(); return null }
        from.removeLast()
        to.addLast(entry)
        return change
    }

    fun clear() { past.clear(); future.clear(); characters = 0 }
}
