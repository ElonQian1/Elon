package com.elon.app.chatgptweb

import android.graphics.Color
import android.graphics.drawable.Drawable
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal object WebChatSideMenuStateViews {
    fun status(
        activity: AppCompatActivity,
        state: ChatGptWebConversationIndexState,
        selectedDate: java.time.LocalDate,
        dp: (Int) -> Int,
    ): View = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        textSize = 12f
        setTextColor(Color.parseColor("#80BEBEBA"))
        val activeCount = ChatGptWebConversationIndex.activeOn(state.conversations, selectedDate).size
        val unassignedCount = state.conversations.count { it.projectId == null }
        val countLabel = "$activeCount 个活跃 · $unassignedCount 个未归项目 · 共 ${state.conversations.size} 个会话"
        text = when {
            state.collection.officialLoadState == ChatGptWebConversationCollection.LOAD_LOADING &&
                state.conversations.isEmpty() -> "正在读取会话"
            state.collection.officialLoadState == ChatGptWebConversationCollection.LOAD_FAILED &&
                state.conversations.isEmpty() -> "读取失败 · 可重新加载"
            state.collection.officialLoadState == ChatGptWebConversationCollection.LOAD_LOADING ->
                "$countLabel · 正在刷新"
            state.collection.officialLoadState == ChatGptWebConversationCollection.LOAD_FAILED ->
                "$countLabel · 显示本机缓存"
            else -> countLabel
        }
        contentDescription = ChatGptNativeNavigationSelector.STATUS
    }

    fun create(
        activity: AppCompatActivity,
        status: WebChatSideMenuContentStatus,
        emptyMessage: String,
        loadingMessage: String,
        failedMessage: String,
        onRetry: () -> Unit,
        dp: (Int) -> Int,
        selectableForeground: () -> Drawable?,
    ): View = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(184))
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(72))
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = when (status) {
                WebChatSideMenuContentStatus.LOADING -> loadingMessage
                WebChatSideMenuContentStatus.FAILED -> failedMessage
                else -> emptyMessage
            }
            textSize = 14f
            setTextColor(Color.parseColor("#99BEBEBA"))
        })
        if (status == WebChatSideMenuContentStatus.FAILED) {
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(128), dp(48))
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "重新加载"
                textSize = 14f
                setTextColor(Color.parseColor("#AFC8F4"))
                contentDescription = ChatGptNativeNavigationSelector.RETRY_CONVERSATIONS
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { onRetry() }
            })
        }
    }
}
