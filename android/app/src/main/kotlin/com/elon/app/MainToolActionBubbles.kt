package com.elon.app

import com.google.gson.JsonObject

internal class MainToolActionBubbles(
    private val activeConversation: () -> AppConversation,
    private val chatAdapter: () -> ChatAdapter,
    private val saveConversations: () -> Unit,
    private val appendMessage: (ChatMessage) -> Unit
) {
    private val pendingToolActionBubbles = linkedMapOf<String, ArrayDeque<Int>>()

    fun clear() {
        pendingToolActionBubbles.clear()
    }

    fun appendToolCallBubble(tool: String, args: JsonObject?) {
        val actionText = describeToolAction(tool, args)
        appendMessage(ChatMessage("ai-action", actionText))
        val messages = activeConversation().messages
        val index = messages.indices.lastOrNull { messages[it].role == "ai-action" } ?: -1
        if (index >= 0) {
            pendingToolActionBubbles.getOrPut(tool) { ArrayDeque() }.addLast(index)
        }
    }

    fun markToolResultDone(tool: String) {
        val queue = pendingToolActionBubbles[tool]
        if (queue == null || queue.isEmpty()) return

        val index = queue.removeFirst()
        val messages = activeConversation().messages
        if (index in messages.indices && messages[index].role == "ai-action") {
            val old = messages[index]
            val newContent = markToolActionDone(old.content)
            if (newContent != old.content) {
                messages[index] = old.copy(content = newContent)
                chatAdapter().notifyMessageUpdated(index)
                saveConversations()
            }
        }
        if (queue.isEmpty()) pendingToolActionBubbles.remove(tool)
    }
}
