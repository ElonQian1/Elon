package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebSideMenuCoordinator
import com.elon.app.chatgptweb.ChatGptWebSideMenuState
import com.elon.app.chatgptweb.ChatGptWebSideMenuTab
import java.time.LocalDate

internal class ChatSideMenuWebChatControl(
    private val coordinator: () -> ChatGptWebSideMenuCoordinator,
    private val ensureOpen: () -> Unit,
) {
    fun state(): ChatGptWebSideMenuState? = coordinator().state()

    fun selectTab(tab: ChatGptWebSideMenuTab): Boolean = select {
        coordinator().selectTab(tab)
    }

    fun selectDate(date: LocalDate): Boolean = select {
        coordinator().selectDate(date)
    }

    private fun select(apply: () -> Boolean): Boolean {
        if (!apply()) return false
        ensureOpen()
        return true
    }
}
