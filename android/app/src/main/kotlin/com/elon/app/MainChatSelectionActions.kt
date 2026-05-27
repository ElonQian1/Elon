package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
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
    private val isProjectChannelActive: () -> Boolean,
    private val summarizeInCurrentChannel: (SelectedDiscussionSummary) -> Boolean,
    private val summarizeInPersonalChat: (String) -> Unit,
    private val summarizeInNewPersonalChat: (String) -> Unit
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
        val summary = buildSelectedDiscussionSummary(selected)
        if (isProjectChannelActive()) {
            showChannelSummaryTargetDialog(summary)
            return
        }
        cancelSelection()
        summarizeInPersonalChat(summary.personalPrompt)
    }

    private fun showChannelSummaryTargetDialog(summary: SelectedDiscussionSummary) {
        AlertDialog.Builder(activity)
            .setTitle("AI 总结")
            .setItems(arrayOf("在当前频道发帖总结", "新个人会话总结")) { _, which ->
                when (which) {
                    0 -> {
                        val handled = summarizeInCurrentChannel(summary)
                        if (handled) {
                            cancelSelection()
                        } else {
                            Toast.makeText(activity, "当前频道暂不能总结", Toast.LENGTH_SHORT).show()
                        }
                    }
                    1 -> {
                        cancelSelection()
                        summarizeInNewPersonalChat(summary.personalPrompt)
                    }
                }
            }
            .show()
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
