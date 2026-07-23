package com.elon.app

import android.graphics.drawable.Drawable
import android.view.DragEvent
import android.view.View
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class ChatSocialSideMenuCoordinator(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val isSocialChatActive: () -> Boolean,
    private val friends: () -> List<AppFriend>,
    private val groups: () -> List<AppGroup>,
    private val activeFriendId: () -> String?,
    private val activeGroupId: () -> String?,
    private val favorites: () -> List<SocialSidebarFavorite>,
    private val openConversation: (SocialSidebarTimelineItem) -> Unit,
    private val loadTimelineMessage: (
        SocialSidebarTimelineItem,
        (Result<ChatMessage>) -> Unit
    ) -> Unit,
    private val sendTimelineMessage: (ChatMessage) -> Unit
) {
    private lateinit var view: ChatSocialSideMenuView

    fun attach(
        panel: FrameLayout,
        requestClose: (Boolean) -> Unit,
        openSettings: () -> Unit,
        dp: (Int) -> Int,
        selectableForeground: () -> Drawable?
    ) {
        view = ChatSocialSideMenuView(
            context = activity,
            timelineItems = {
                buildSocialSidebarTimeline(
                    friends = friends(),
                    groups = groups(),
                    activeFriendId = activeFriendId(),
                    activeGroupId = activeGroupId()
                )
            },
            favoriteItems = favorites,
            openConversation = openConversation,
            loadTimelineMessage = loadTimelineMessage,
            openSettings = openSettings,
            requestClose = requestClose,
            dp = dp,
            selectableForeground = selectableForeground
        )
        panel.addView(
            view,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
        )
        view.visibility = View.GONE
    }

    fun show() {
        view.visibility = View.VISIBLE
        view.render()
    }

    fun hide() {
        view.visibility = View.GONE
    }

    fun handleDrag(
        event: DragEvent,
        panelWidth: Int,
        overlay: FrameLayout,
        close: () -> Unit
    ): Boolean? {
        val payload = event.localState as? SocialTimelineDragPayload ?: return null
        if (!isSocialChatActive()) return false
        return when (event.action) {
            DragEvent.ACTION_DRAG_STARTED -> true
            DragEvent.ACTION_DROP -> {
                if (event.x > panelWidth) {
                    showChatSocialDropRipple(overlay, binding.contentContainer, event.x, event.y)
                    close()
                    overlay.postDelayed({ sendTimelineMessage(payload.message) }, 140L)
                }
                true
            }
            DragEvent.ACTION_DRAG_ENDED -> true
            else -> true
        }
    }
}
