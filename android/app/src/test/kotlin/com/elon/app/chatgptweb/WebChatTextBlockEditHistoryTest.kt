package com.elon.app.chatgptweb

import com.elon.app.WebChatTextBlock
import com.elon.app.WebChatTextBlockEditHistory
import org.junit.Assert.*
import org.junit.Test

class WebChatTextBlockEditHistoryTest {
    private fun apply(text: String, change: WebChatTextBlockEditHistory.Change?) =
        requireNotNull(change).let { text.replaceRange(it.start, it.start + it.before.length, it.after) }

    @Test fun replacementsCanBeUndoneAndRedoneWithoutChangingWhitespaceOrUnicode() {
        val history = WebChatTextBlockEditHistory()
        val original = "  first\r\n\r\n\t\uD83D\uDE00 end\n"
        val edited = original.replace("first", "second")
        history.record(2, "first", "second")
        assertEquals(original, apply(edited, history.undo(edited)))
        assertFalse(history.canUndo)
        assertTrue(history.canRedo)
        assertEquals(edited, apply(original, history.redo(original)))
        assertFalse(history.canRedo)
    }

    @Test fun emptyDocumentAndFullResetRemainReversible() {
        val original = "x".repeat(WebChatTextBlock.MAX_CONTENT)
        val history = WebChatTextBlockEditHistory()
        history.record(0, original, "")
        assertEquals(original, apply("", history.undo("")))
        assertEquals("", apply(original, history.redo(original)))
        history.record(0, "", original)
        assertEquals("", apply(original, history.undo(original)))
    }

    @Test fun newEditAfterUndoDropsOnlyTheRedoBranch() {
        val history = WebChatTextBlockEditHistory()
        history.record(0, "", "a")
        history.record(1, "", "b")
        assertEquals("a", apply("ab", history.undo("ab")))
        history.record(1, "", "c")
        assertFalse(history.canRedo)
        assertEquals("a", apply("ac", history.undo("ac")))
        assertEquals("", apply("a", history.undo("a")))
    }

    @Test fun noOpDoesNotConsumeHistoryOrDropRedo() {
        val history = WebChatTextBlockEditHistory()
        history.record(0, "", "a")
        history.undo("a")
        history.record(0, "", "")
        assertTrue(history.canRedo)
        assertEquals("a", apply("", history.redo("")))
    }

    @Test fun entryLimitEvictsOldestEditsAndKeepsRecentEditsConsistent() {
        val history = WebChatTextBlockEditHistory(maxEntries = 2)
        history.record(0, "", "a")
        history.record(1, "", "b")
        history.record(2, "", "c")
        assertEquals("ab", apply("abc", history.undo("abc")))
        assertEquals("a", apply("ab", history.undo("ab")))
        assertNull(history.undo("a"))
        assertEquals("ab", apply("a", history.redo("a")))
        assertEquals("abc", apply("ab", history.redo("ab")))
    }

    @Test fun characterBudgetCountsDeltasInsteadOfLargeDocumentSnapshots() {
        val original = "x".repeat(100_000)
        val history = WebChatTextBlockEditHistory(maxCharacters = 2)
        history.record(original.length, "", "a")
        history.record(original.length + 1, "", "b")
        assertEquals(original + "a", apply(original + "ab", history.undo(original + "ab")))
        assertEquals(original, apply(original + "a", history.undo(original + "a")))
    }

    @Test fun characterLimitEvictionAndRedoBranchAccountingAreBounded() {
        val history = WebChatTextBlockEditHistory(maxCharacters = 3)
        history.record(0, "", "aa")
        history.record(2, "", "bb")
        assertEquals("aa", apply("aabb", history.undo("aabb")))
        assertNull(history.undo("aa"))
        history.record(2, "", "ccc")
        assertFalse(history.canRedo)
        assertEquals("aa", apply("aaccc", history.undo("aaccc")))
        history.record(2, "", "oversized")
        assertFalse(history.canUndo)
        assertFalse(history.canRedo)
    }

    @Test fun unexpectedTextCannotBeOverwrittenByStaleUndoOrRedo() {
        val history = WebChatTextBlockEditHistory()
        history.record(2, "old", "new")
        assertNull(history.undo("unrelated"))
        assertFalse(history.canUndo)
        history.record(2, "old", "new")
        history.undo("  new")
        assertNull(history.redo(""))
        assertFalse(history.canRedo)
    }
}
