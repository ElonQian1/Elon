package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class MainEmojiActionsLayoutTest {
    @Test
    fun panelKeepsKeyboardExtentWhileLeavingComposerGap() {
        val viewportBottom = 2400
        val keyboardHeight = 900
        val composerBottomGap = 18
        val extent = resolveEmojiPanelExtent(keyboardHeight, composerBottomGap)

        assertEquals(18, extent.topGap)
        assertEquals(882, extent.contentHeight)
        assertEquals(900, extent.totalHeight)
        assertEquals(
            viewportBottom - keyboardHeight,
            viewportBottom - composerBottomGap - extent.totalHeight + extent.topGap
        )
        assertEquals(
            viewportBottom - keyboardHeight - composerBottomGap,
            viewportBottom - composerBottomGap - extent.totalHeight
        )
    }

    @Test
    fun animationExtentSmallerThanGapDoesNotJumpPastCurrentHeight() {
        val extent = resolveEmojiPanelExtent(totalHeight = 10, preferredTopGap = 18)

        assertEquals(10, extent.topGap)
        assertEquals(0, extent.contentHeight)
        assertEquals(10, extent.totalHeight)
    }

    @Test
    fun zeroExtentAndZeroGapStayUnchanged() {
        assertEquals(EmojiPanelExtent(0, 0), resolveEmojiPanelExtent(0, 18))
        assertEquals(EmojiPanelExtent(0, 900), resolveEmojiPanelExtent(900, 0))
    }
}
