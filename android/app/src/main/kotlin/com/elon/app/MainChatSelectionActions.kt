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
    private val summarizeInNewPersonalChat: (String) -> Unit,
    private val isAiChat: () -> Boolean = { false },
    private val forwardAiMessages: (List<ChatMessage>, () -> Unit) -> Unit = { _, _ -> },
    private val isGroupChat: () -> Boolean = { false },
    private val analyzeGroupMessages: (List<ChatMessage>, () -> Unit) -> Unit = { _, _ -> },
) {
    private var inputVisibility = View.GONE
    private var tabsVisibility = View.GONE
    private val selectionHeader by lazy { MainChatSelectionHeader(binding, ::cancelSelection) }
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
        if (!adapter.isSelectionModeActive()) {
            inputVisibility = binding.inputLayout.visibility
            tabsVisibility = binding.pageTabs.visibility
        }
        adapter.setSelectionChangedListener(::renderSelectionCount)
        adapter.startSelection(message)
        if (!adapter.isSelectionModeActive()) return
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.GONE
        binding.chatSelectionBar.visibility = View.VISIBLE
        if (isAiChat() || isGroupChat()) {
            selectionHeader.show()
            binding.selectionCancelButton.visibility = View.GONE
            binding.selectionCountText.visibility = View.GONE
            binding.selectionDeleteButton.visibility = View.GONE
            if (isAiChat()) {
                binding.selectionForwardButton.text = "合并转发"
                binding.selectionForwardButton.contentDescription = "ai-conversation-share-selected"
            }
            if (isGroupChat()) binding.selectionSummarizeButton.text = "AI 分析"
        }
        renderSelectionCount(adapter.selectedMessagesInOrder().size)
    }

    fun cancelSelection() {
        val wasSelecting = binding.chatSelectionBar.visibility == View.VISIBLE
        val adapter = currentAdapterOrNull()
        adapter?.exitSelection()
        adapter?.setSelectionChangedListener(null)
        binding.chatSelectionBar.visibility = View.GONE
        selectionHeader.hide()
        binding.selectionCancelButton.visibility = View.VISIBLE
        binding.selectionCountText.visibility = View.VISIBLE
        binding.selectionDeleteButton.visibility = View.VISIBLE
        binding.selectionForwardButton.text = "转发"
        binding.selectionForwardButton.contentDescription = "转发"
        binding.selectionSummarizeButton.text = "AI 总结"
        if (wasSelecting && binding.chatPage.visibility == View.VISIBLE) {
            binding.inputLayout.visibility = inputVisibility
            binding.pageTabs.visibility = tabsVisibility
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
        if (isAiChat()) {
            forwardAiMessages(selected, ::cancelSelection)
            return
        }
        shareActions().forwardMessageText(selectedDiscussionTranscript(selected))
        cancelSelection()
    }

    private fun summarizeSelectedMessages() {
        val selected = selectedMessagesOrToast() ?: return
        if (isGroupChat()) { analyzeGroupMessages(selected, ::cancelSelection); return }
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
        cancelSelection()
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
        if (isAiChat() || isGroupChat()) selectionHeader.update(count)
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
