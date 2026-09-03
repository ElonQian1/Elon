package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebComposerOption
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation

internal object ChatGptConsumerModelOptionMapper {
    fun map(option: ChatGptWebComposerOption): WebChatConsumerOption? {
        val id = option.id.trim()
        val label = option.label.trim()
        if (id.isBlank() || label.isBlank()) return null
        return WebChatConsumerOption(
            id = id,
            label = label,
            selected = option.selected,
            semantic = option.semantic,
            opensSubmenu = option.opensSubmenu,
            nativeSelector = "web-chat-model-option:" +
                ChatGptNativeControlPresentation.stableContextId(id),
            parentId = option.parentId,
            parentLabel = option.parentLabel,
        )
    }
}
