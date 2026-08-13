package com.elon.app

import android.view.View
import com.elon.app.chatgptweb.ChatGptWebSideMenuCoordinator

internal data class ChatSideMenuContentSources(
    val social: ChatSocialSideMenuCoordinator,
    val webChat: ChatGptWebSideMenuCoordinator,
)

internal fun applyChatSideMenuContentMode(
    aiMenu: ChatAiSideMenuView,
    projectMenu: ChatProjectSideMenuView,
    sources: ChatSideMenuContentSources,
    projectShareVisible: Boolean,
    conversationHomeVisible: Boolean,
) {
    val webChatVisible = sources.webChat.isActive()
    aiMenu.visibility = if (webChatVisible || projectShareVisible || conversationHomeVisible) {
        View.GONE
    } else {
        View.VISIBLE
    }
    projectMenu.visibility = if (conversationHomeVisible && !webChatVisible && !projectShareVisible) {
        View.VISIBLE
    } else {
        View.GONE
    }
    if (aiMenu.visibility == View.VISIBLE) aiMenu.render() else aiMenu.stopAnimations()
    if (projectMenu.visibility == View.VISIBLE) projectMenu.render()
    if (projectShareVisible && !webChatVisible) sources.social.show() else sources.social.hide()
    if (webChatVisible) sources.webChat.show() else sources.webChat.hide()
}
