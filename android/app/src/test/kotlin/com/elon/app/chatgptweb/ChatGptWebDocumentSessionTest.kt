package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebDocumentSessionTest {
    @Test
    fun acceptsOnlyTheCurrentDocumentTokenAndAdvancesGenerations() {
        val session = ChatGptWebDocumentSession { generation -> "doc_test_$generation" }

        val first = session.beginPage()
        assertEquals(1L, first.pageGeneration)
        assertFalse(first.adapterCurrent)
        assertNull(session.accept("doc_test_0"))
        assertTrue(session.accept(first.documentToken)?.adapterCurrent == true)

        val second = session.beginPage()
        assertEquals(2L, second.pageGeneration)
        assertEquals(0L, second.adapterGeneration)
        assertFalse(second.adapterCurrent)
        assertNull(session.accept(first.documentToken))
        assertTrue(session.accept(second.documentToken)?.adapterCurrent == true)
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsUnsafeGeneratedTokens() {
        ChatGptWebDocumentSession { "../../stale" }.beginPage()
    }
}
