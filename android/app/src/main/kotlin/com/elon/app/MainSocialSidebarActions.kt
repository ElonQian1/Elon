package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainSocialSidebarActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val state: MainActivityState,
    private val serverUrl: () -> String,
    private val friendChatActions: () -> MainFriendChatActions,
    private val groupChatActions: () -> MainGroupChatActions,
    private val projectSpaceController: () -> ProjectSpaceController,
    private val syncVisibleChatNotificationState: () -> Unit,
    private val refreshSidebar: () -> Unit
) {
    private val favorites by lazy { ChatSocialFavorites(activity) }
    private val messageLoader by lazy {
        ChatSocialSidebarMessageLoader(activity, state.http, serverUrl())
    }

    val coordinator by lazy {
        ChatSocialSideMenuCoordinator(
            activity = activity,
            binding = binding,
            isSocialChatActive = {
                friendChatActions().isActive() || groupChatActions().isActive()
            },
            friends = { state.friends },
            groups = { state.groups },
            activeFriendId = { friendChatActions().currentFriend()?.id },
            activeGroupId = { groupChatActions().currentGroup()?.id },
            favorites = { favorites.list() },
            openConversation = ::openConversation,
            loadTimelineMessage = messageLoader::loadLatestIncoming,
            sendTimelineMessage = ::sendTimelineMessage
        )
    }

    fun favoriteMessage(message: ChatMessage) {
        if (!friendChatActions().isActive() && !groupChatActions().isActive()) return
        favorites.add(message)
        refreshSidebar()
    }

    private fun openConversation(item: SocialSidebarTimelineItem) {
        when (item.key.type) {
            SocialSidebarConversationType.FRIEND ->
                state.friends.firstOrNull { it.id == item.key.id }?.let { friend ->
                    groupChatActions().closeGroupChat()
                    projectSpaceController().closeChannelChat()
                    friendChatActions().openFriend(friend, animate = true)
                    syncVisibleChatNotificationState()
                }
            SocialSidebarConversationType.GROUP ->
                state.groups.firstOrNull { it.id == item.key.id }?.let { group ->
                    friendChatActions().closeFriendChat()
                    projectSpaceController().closeChannelChat()
                    groupChatActions().openGroup(group, animate = true)
                    syncVisibleChatNotificationState()
                }
        }
    }

    private fun sendTimelineMessage(message: ChatMessage) {
        if (!friendChatActions().trySendForwardedMessage(message)) {
            groupChatActions().trySendForwardedMessage(message)
        }
    }
}
