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

    @Test
    fun sidebarStateKeepsSelectedProjectFolder() {
        val state = ChatGptWebSideMenuState(
            tab = ChatGptWebSideMenuTab.PROJECTS,
            date = java.time.LocalDate.of(2026, 8, 18),
            selectedProjectId = "project-1",
        )

        assertEquals("project-1", state.selectedProjectId)
    }
}
