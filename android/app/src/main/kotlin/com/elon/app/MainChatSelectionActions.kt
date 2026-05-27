package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainChatSelectionActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val chatAdapter: () -> ChatAdapter,
    private val activeConversation: () -> AppConversation,
    private val saveConversations: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val shareActions: () -> MainShareActions,
    private val sendAiSummaryPrompt: (String) -> Unit
) {
    fun setup() {
        binding.selectionCancelButton.setOnClickListener { cancelSelection() }
        binding.selectionCopyButton.setOnClickListener { copySelectedMessages() }
        binding.selectionForwardButton.setOnClickListener { forwardSelectedMessages() }
        binding.selectionSummarizeButton.setOnClickListener { summarizeSelectedMessages() }
        binding.selectionDeleteButton.setOnClickListener { deleteSelectedMessages() }
        renderSelectionCount(0)
    }

    fun startSelection(message: ChatMessage) {
        val adapter = currentAdapterOrNull() ?: return
        adapter.setSelectionChangedListener(::renderSelectionCount)
        adapter.startSelection(message)
        if (!adapter.isSelectionModeActive()) return
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.GONE
        binding.chatSelectionBar.visibility = View.VISIBLE
        renderSelectionCount(adapter.selectedMessagesInOrder().size)
    }

    fun cancelSelection() {
        val adapter = currentAdapterOrNull()
        adapter?.exitSelection()
        adapter?.setSelectionChangedListener(null)
        binding.chatSelectionBar.visibility = View.GONE
        if (binding.chatPage.visibility == View.VISIBLE) {
            binding.inputLayout.visibility = View.VISIBLE
        }
        renderSelectionCount(0)
    }

    fun isSelectionActive(): Boolean {
        return currentAdapterOrNull()?.isSelectionModeActive() == true
    }

    private fun copySelectedMessages() {
        val selected = selectedMessagesOrToast() ?: return
        shareActions().copyMessageText(selectedDiscussionTranscript(selected))
        cancelSelection()
    }

    private fun forwardSelectedMessages() {
        val selected = selectedMessagesOrToast() ?: return
        shareActions().forwardMessageText(selectedDiscussionTranscript(selected))
        cancelSelection()
    }

    private fun summarizeSelectedMessages() {
        val selected = selectedMessagesOrToast() ?: return
        val prompt = selectedDiscussionSummaryPrompt(selected)
        cancelSelection()
        sendAiSummaryPrompt(prompt)
    }

    private fun deleteSelectedMessages() {
        val adapter = currentAdapterOrNull() ?: return
        selectedMessagesOrToast() ?: return
        val messages = activeConversation().messages
        if (!adapter.ownsMessages(messages)) {
            Toast.makeText(activity, "当前聊天暂不支持批量删除服务器消息", Toast.LENGTH_SHORT).show()
            return
        }
        val indices = adapter.selectedPositionsDescending()
            .filter { index -> index in messages.indices }

        if (indices.isEmpty()) {
            Toast.makeText(activity, "当前聊天暂不支持批量删除服务器消息", Toast.LENGTH_SHORT).show()
            return
        }

        indices.forEach { index -> messages.removeAt(index) }
        adapter.exitSelection()
        adapter.setSelectionChangedListener(null)
        binding.chatSelectionBar.visibility = View.GONE
        if (binding.chatPage.visibility == View.VISIBLE) {
            binding.inputLayout.visibility = View.VISIBLE
        }
        saveConversations()
        renderConversationList()
        Toast.makeText(activity, "已删除 ${indices.size} 条", Toast.LENGTH_SHORT).show()
    }

    private fun selectedMessagesOrToast(): List<ChatMessage>? {
        val selected = currentAdapterOrNull()?.selectedMessagesInOrder().orEmpty()
        if (selected.isEmpty()) {
            Toast.makeText(activity, "请先选择消息", Toast.LENGTH_SHORT).show()
            return null
        }
        return selected
    }

    private fun renderSelectionCount(count: Int) {
        binding.selectionCountText.text = if (count > 0) {
            "已选择 $count 条"
        } else {
            "选择消息"
        }
        setSelectionActionsEnabled(count > 0)
    }

    private fun setSelectionActionsEnabled(enabled: Boolean) {
        listOf(
            binding.selectionCopyButton,
            binding.selectionForwardButton,
            binding.selectionSummarizeButton,
            binding.selectionDeleteButton
        ).forEach { button ->
            button.isEnabled = enabled
            button.alpha = if (enabled) 1f else 0.42f
        }
    }

    private fun currentAdapterOrNull(): ChatAdapter? {
        return runCatching { chatAdapter() }.getOrNull()
    }
}

private fun selectedDiscussionSummaryPrompt(messages: List<ChatMessage>): String {
    return """
        请总结下面我多选的聊天讨论，直接给出中文结论。

        输出结构：
        1. 核心结论
        2. 已决定事项
        3. 待确认问题
        4. 下一步行动

        选中的讨论：
        ${selectedDiscussionTranscript(messages)}
    """.trimIndent()
}

private fun selectedDiscussionTranscript(messages: List<ChatMessage>): String {
    val builder = StringBuilder()
    messages.forEachIndexed { index, message ->
        if (builder.length >= MAX_SELECTED_DISCUSSION_CHARS) return@forEachIndexed
        val line = "${index + 1}. ${message.selectionSpeakerLabel()}: ${message.selectionContent()}"
        val remaining = MAX_SELECTED_DISCUSSION_CHARS - builder.length
        builder.appendLine(line.take(remaining))
    }
    if (builder.length >= MAX_SELECTED_DISCUSSION_CHARS) {
        builder.appendLine("（后续内容过长，已截断）")
    }
    return builder.toString().trim()
}

private fun ChatMessage.selectionSpeakerLabel(): String {
    return when (role) {
        "user" -> "我"
        "friend" -> senderLabel?.takeIf { it.isNotBlank() } ?: "对方"
        "ai", "ai-intent", "ai-complete" -> "AI"
        "ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-stopped" -> "开发进度"
        "error" -> "错误"
        else -> role
    }
}

private fun ChatMessage.selectionContent(): String {
    val attachmentText = attachments
        ?.takeIf { it.isNotEmpty() }
        ?.let { " [附件 ${it.size} 个]" }
        .orEmpty()
    val text = content.replace(Regex("\\s+"), " ").trim().ifBlank { "（无文字内容）" }
    return "${summarize(text, MAX_SELECTED_MESSAGE_CHARS)}$attachmentText"
}

private const val MAX_SELECTED_MESSAGE_CHARS = 1200
private const val MAX_SELECTED_DISCUSSION_CHARS = 7000
