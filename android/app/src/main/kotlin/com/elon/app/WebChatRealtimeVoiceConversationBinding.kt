package com.elon.app

import android.view.View

internal fun resolveRealtimeVoiceContext(
    controller: ChatGptSocialChatController,
): WebChatRealtimeVoiceContext {
    val temporaryChat = controller.consumerPort().state().controls.any { descriptor ->
        descriptor.control.semantic == "temporary_chat" && descriptor.control.selected
    }
    return WebChatRealtimeVoiceContextPolicy.resolve(
        conversationPath = controller.currentConversationPath(),
        conversations = controller.conversationIndex().conversations,
        temporaryChat = temporaryChat,
    )
}

internal fun openRealtimeVoiceConversation(
    context: WebChatRealtimeVoiceContext,
    modeController: SocialAiChatModeController,
    host: View,
    controller: ChatGptSocialChatController,
) {
    modeController.openChatGptWeb()
    val path = context.conversationPath ?: return
    host.post { controller.openConversation(path) }
}
