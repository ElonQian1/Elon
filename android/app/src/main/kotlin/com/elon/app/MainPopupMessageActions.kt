package com.elon.app

internal fun canRecallPopupMessage(
    message: ChatMessage,
    friendChatActions: MainFriendChatActions,
    groupChatActions: MainGroupChatActions,
    projectSpaceController: ProjectSpaceController,
    messageActions: MainMessageActions
): Boolean = when {
    friendChatActions.isActive() -> message.canRecallNow()
    groupChatActions.isActive() -> message.canRecallNow()
    projectSpaceController.isChannelActive() -> projectSpaceController.canRecallCurrentMessage(message)
    else -> messageActions.canRecallMessage(message)
}

internal fun recallPopupMessage(
    message: ChatMessage,
    friendChatActions: MainFriendChatActions,
    groupChatActions: MainGroupChatActions,
    projectSpaceController: ProjectSpaceController,
    messageActions: MainMessageActions
) {
    when {
        friendChatActions.isActive() -> friendChatActions.recallCurrentMessage(message)
        groupChatActions.isActive() -> groupChatActions.recallCurrentMessage(message)
        projectSpaceController.isChannelActive() -> projectSpaceController.recallCurrentMessage(message)
        else -> messageActions.recallMessage(message)
    }
}

internal fun deletePopupMessage(
    message: ChatMessage,
    friendChatActions: MainFriendChatActions,
    groupChatActions: MainGroupChatActions,
    projectSpaceController: ProjectSpaceController,
    messageActions: MainMessageActions
) {
    when {
        friendChatActions.isActive() -> friendChatActions.deleteCurrentMessage(message) {}
        groupChatActions.isActive() -> groupChatActions.deleteCurrentMessage(message) {}
        projectSpaceController.isChannelActive() -> projectSpaceController.recallCurrentMessage(message)
        else -> messageActions.deleteMessage(message)
    }
}
