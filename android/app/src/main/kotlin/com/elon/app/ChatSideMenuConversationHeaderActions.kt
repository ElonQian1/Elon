package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

internal class ChatSideMenuConversationHeaderActions(
    private val activity: AppCompatActivity,
    private val createConversationAndOpenAction: () -> Unit,
    private val syncProjectsFromServer: (((Boolean) -> Unit)?) -> Unit,
    private val refreshVisibleContent: () -> Unit
) {
    private var refreshInFlight = false

    fun createConversationAndOpen() {
        createConversationAndOpenAction()
    }

    fun refreshConversations() {
        if (refreshInFlight) return
        refreshInFlight = true
        Toast.makeText(activity, "正在刷新会话...", Toast.LENGTH_SHORT).show()
        syncProjectsFromServer { ok ->
            refreshInFlight = false
            refreshVisibleContent()
            Toast.makeText(
                activity,
                if (ok) "会话已刷新" else "刷新失败，请稍后重试",
                Toast.LENGTH_SHORT
            ).show()
        }
    }
}
