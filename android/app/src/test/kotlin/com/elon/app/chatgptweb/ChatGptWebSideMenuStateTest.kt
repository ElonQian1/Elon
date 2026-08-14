package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebSideMenuStateTest {
    @Test
    fun parsesStableSidebarSections() {
        assertEquals(ChatGptWebSideMenuTab.DATE, ChatGptWebSideMenuTab.parse("date"))
        assertEquals(ChatGptWebSideMenuTab.PROJECTS, ChatGptWebSideMenuTab.parse(" PROJECTS "))
        assertNull(ChatGptWebSideMenuTab.parse("unknown"))
    }
}
