package com.elon.app.chatgptweb

import android.graphics.Color
import android.graphics.drawable.Drawable
import android.view.Gravity
import android.widget.ImageView
import android.widget.LinearLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R
import com.elon.app.WebChatLocalProjectActions
import com.elon.app.WebChatLocalProjectDialogs

internal class ChatGptWebSideMenuConversationActions(
    private val activity: AppCompatActivity,
    private val index: () -> ChatGptWebConversationIndexState,
    private val localProjectActions: () -> WebChatLocalProjectActions?,
    private val remoteActionsAvailable: () -> Boolean,
    private val openRemoteActions: (ChatGptWebConversation) -> Unit,
    private val closeThen: (() -> Unit) -> Unit,
    private val render: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
) {
    fun available(): Boolean = localProjectActions() != null || remoteActionsAvailable()

    fun show(conversation: ChatGptWebConversation): Boolean {
        localProjectActions()?.let { actions ->
            WebChatLocalProjectDialogs.showAssignment(activity, index(), conversation, actions, render)
            return true
        }
        if (!remoteActionsAvailable()) return false
        closeThen { openRemoteActions(conversation) }
        return true
    }

    fun button(conversation: ChatGptWebConversation) = ImageView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(dp(48), LinearLayout.LayoutParams.MATCH_PARENT).apply {
            gravity = Gravity.CENTER_VERTICAL
        }
        setImageResource(R.drawable.ic_more_horizontal)
        setColorFilter(Color.parseColor("#B3DDDBD5"))
        scaleType = ImageView.ScaleType.CENTER_INSIDE
        setPadding(dp(10), dp(10), dp(10), dp(10))
        contentDescription = ChatGptNativeNavigationSelector.conversationActions(conversation)
        isClickable = true
        isFocusable = true
        foreground = selectableForeground()
        setOnClickListener { show(conversation) }
    }
}
