package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebFileSelectionResultTest {
    @Test
    fun allowsOnlyDistinctContentUris() {
        assertEquals(
            listOf("content://fixture/first", "CONTENT://fixture/second"),
            ChatGptWebFileSelectionPolicy.filter(
                listOf(
                    "file:///sdcard/private.txt",
                    "content://fixture/first",
                    "content://fixture/first",
                    "CONTENT://fixture/second",
                    "https://example.test/file",
                ),
            ),
        )
    }

    @Test
    fun boundsSelectedUris() {
        val values = (0 until 20).map { "content://fixture/$it" }
        assertEquals(values.take(10), ChatGptWebFileSelectionPolicy.filter(values))
    }
}
