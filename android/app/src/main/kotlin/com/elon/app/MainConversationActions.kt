package com.elon.app

import android.view.View
import android.widget.EditText
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainConversationActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val conversationsProvider: () -> MutableList<AppConversation>,
    private val activeProjectProvider: () -> AppProject,
    private val activeConversationIndexProvider: () -> Int,
    private val setActiveConversationIndex: (Int) -> Unit,
    private val chatAdapterProvider: () -> ChatAdapter,
    private val titleEditText: (String) -> EditText,
    private val saveConversations: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val setSendEnabled: (Boolean) -> Unit,
    private val onConversationsChanged: () -> Unit = {}
) {
    fun showCreateConversationDialog(suggestedTitle: String? = null, onCreated: ((Int) -> Unit)? = null) {
        val conversations = conversationsProvider()
        val input = titleEditText(suggestedTitle ?: "新会话 ${conversations.size + 1}")
        val dialog = AlertDialog.Builder(activity)
            .setTitle("新建会话")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入会话标题"
                    return@setOnClickListener
                }
                createConversation(title, onCreated)
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    fun showConversationActions(index: Int) {
        val conversations = conversationsProvider()
        if (index !in conversations.indices) return
        val conversation = conversations[index]
        val actions = if (conversation.ended) {
            arrayOf("编辑标题", "删除会话")
        } else {
            arrayOf("编辑标题", "结束会话", "删除会话")
        }

        AlertDialog.Builder(activity)
            .setTitle(conversation.title)
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "编辑标题" -> showRenameConversationDialog(index)
                    "结束会话" -> confirmEndConversation(index)
                    "删除会话" -> confirmDeleteConversation(index)
                }
            }
            .show()
    }

    private fun createConversation(title: String, onCreated: ((Int) -> Unit)? = null) {
        val conversations = conversationsProvider()
        val project = activeProjectProvider()
        conversations.add(newAppConversation(title, "点击进入开发会话"))
        project.updatedAt = System.currentTimeMillis()
        project.subtitle = "${conversations.size} 个会话"
        saveConversations()
        renderConversationList()
        onConversationsChanged()
        onCreated?.invoke(conversations.lastIndex)
    }

    private fun showRenameConversationDialog(index: Int) {
        val conversations = conversationsProvider()
        if (index !in conversations.indices) return
        val conversation = conversations[index]
        val input = titleEditText(conversation.title)
        val dialog = AlertDialog.Builder(activity)
            .setTitle("编辑会话标题")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入会话标题"
                    return@setOnClickListener
                }
                conversation.title = summarize(title, 24)
                conversation.updatedAt = System.currentTimeMillis()
                saveConversations()
                renderConversationList()
                onConversationsChanged()
                if (activeConversationIndexProvider() == index && binding.chatPage.visibility == View.VISIBLE) {
                    binding.topTitleText.text = conversation.title
                }
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    private fun confirmEndConversation(index: Int) {
        val conversations = conversationsProvider()
        if (index !in conversations.indices) return
        AlertDialog.Builder(activity)
            .setTitle("结束会话")
            .setMessage("结束后仍可查看记录，但不能继续发送消息。")
            .setNegativeButton("取消", null)
            .setPositiveButton("结束") { _, _ -> endConversation(index) }
            .show()
    }

    private fun endConversation(index: Int) {
        val conversations = conversationsProvider()
        if (index !in conversations.indices) return
        val conversation = conversations[index]
        val project = activeProjectProvider()
        conversation.ended = true
        conversation.subtitle = "会话已结束"
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        conversation.messages.add(ChatMessage("ai", "本会话已结束，可以在会话列表长按删除，或新建会话继续。"))
        saveConversations()
        renderConversationList()
        onConversationsChanged()

        if (activeConversationIndexProvider() == index && binding.chatPage.visibility == View.VISIBLE) {
            val chatAdapter = chatAdapterProvider()
            chatAdapter.notifyItemInserted(conversation.messages.lastIndex)
            binding.chatList.scrollToPosition(conversation.messages.lastIndex)
            setSendEnabled(false)
        }
    }

    private fun confirmDeleteConversation(index: Int) {
        val conversations = conversationsProvider()
        if (index !in conversations.indices) return
        AlertDialog.Builder(activity)
            .setTitle("删除会话")
            .setMessage("删除后这条会话记录会从本机移除。")
            .setNegativeButton("取消", null)
            .setPositiveButton("删除") { _, _ -> deleteConversation(index) }
            .show()
    }

    private fun deleteConversation(index: Int) {
        val conversations = conversationsProvider()
        if (index !in conversations.indices) return
        val project = activeProjectProvider()
        conversations.removeAt(index)
        if (conversations.isEmpty()) {
            conversations.add(defaultAppConversation())
        }
        project.subtitle = "${conversations.size} 个会话"
        project.updatedAt = System.currentTimeMillis()
        setActiveConversationIndex(activeConversationIndexProvider().coerceAtMost(conversations.lastIndex))
        saveConversations()
        renderConversationList()
        onConversationsChanged()
        if (binding.chatPage.visibility == View.VISIBLE) {
            binding.tabChat.performClick()
        }
    }
}
