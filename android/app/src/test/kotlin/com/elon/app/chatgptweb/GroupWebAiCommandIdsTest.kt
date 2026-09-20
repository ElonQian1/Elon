package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class GroupWebAiCommandIdsTest {
    @Test fun groupCommandsUseThePersonalChatsCanonicalLongSequence() {
        val ids = List(100) { GroupWebAiCommandIds.next() }
        val values = ids.map { it.removePrefix("mcp_").toLong(36) }
        assertTrue(values.first() > 0)
        assertTrue(values.zipWithNext().all { (first, second) -> second == first + 1 })
        assertTrue(ids.all { Regex("mcp_[1-9a-z][a-z0-9]{0,12}").matches(it) })
        assertEquals(values.map { "mcp_${it.toString(36)}" }, ids)
        val canonical = ChatGptWebObservedState()
        assertEquals("mcp_1", canonical.nextRequestId())
        assertEquals("mcp_2", canonical.nextRequestId())
    }
}
