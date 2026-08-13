package com.elon.app

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
}
