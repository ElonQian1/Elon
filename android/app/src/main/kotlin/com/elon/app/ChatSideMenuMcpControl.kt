package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebSideMenuState
import com.elon.app.chatgptweb.ChatGptWebSideMenuTab
import java.time.LocalDate

internal class ChatSideMenuMcpControl(
    private val controller: ChatSideMenuController,
) {
    fun isOpen(): Boolean = controller.isOpen

    fun open() {
        controller.open()
    }

    fun close() {
        controller.close(animate = false)
    }

    fun webChatState(): ChatGptWebSideMenuState? = controller.webChatControl.state()

    fun selectWebChatTab(tab: ChatGptWebSideMenuTab): Boolean =
        controller.webChatControl.selectTab(tab)

    fun selectWebChatDate(date: LocalDate): Boolean =
        controller.webChatControl.selectDate(date)
}
