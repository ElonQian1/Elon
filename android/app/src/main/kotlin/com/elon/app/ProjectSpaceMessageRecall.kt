package com.elon.app

import android.widget.Toast
import kotlin.concurrent.thread

private const val RECALL_BLOCKED_CHANNEL_KIND = "docs"

internal fun ProjectSpaceController.canRecallCurrentMessage(message: ChatMessage): Boolean {
    val channel = activeChannel ?: return false
    if (activeMemberConversation != null) return false
    if (channel.kind == RECALL_BLOCKED_CHANNEL_KIND) return false
    return message.canRecallNow()
}

internal fun ProjectSpaceController.recallCurrentMessage(message: ChatMessage, onRecalled: () -> Unit = {}) {
    val channel = activeChannel ?: return
    if (!canRecallCurrentMessage(message)) return
    val messageId = message.id?.trim().takeIf { !it.isNullOrEmpty() } ?: run {
        Toast.makeText(activity, "消息尚未同步，稍后再试", Toast.LENGTH_SHORT).show()
        return
    }
    val postMessageId = activePostMessageId
    val messageKey = activeChannelMessageKey(channel, postMessageId)
    val route = activeRoute
    thread {
        val result = runCatching {
            recallProjectChannelMessage(
                http = http,
                serverUrl = serverUrl,
                context = activity,
                projectId = channel.projectId,
                channelId = channel.id,
                messageId = messageId,
                route = route
            )
        }
        activity.runOnUiThread {
            if (activeChannel?.id != channel.id || activePostMessageId != postMessageId) return@runOnUiThread
            val messages = messagesByChannel[messageKey] ?: return@runOnUiThread
            val index = messages.indexOfFirst { it.id == messageId }
            result
                .onSuccess {
                    if (index >= 0) {
                        markChatMessageRecalled(messages[index], activeProjectId)
                        activeAdapter?.notifyMessageUpdated(index)
                    }
                    onRecalled()
                    Toast.makeText(activity, "已撤回", Toast.LENGTH_SHORT).show()
                    loadMessages(channel, silent = true, scrollToBottom = false, allowPendingRefresh = true)
                }
                .onFailure { error ->
                    Toast.makeText(activity, error.message ?: "撤回失败", Toast.LENGTH_LONG).show()
                }
        }
    }
}
