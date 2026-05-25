package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainMessageActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val chatAdapter: () -> ChatAdapter,
    private val saveConversations: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val showChat: () -> Unit,
    private val showMessageActionPopup: (View, ChatMessage, String) -> Unit,
    private val shareActions: () -> MainShareActions,
    private val apkDownloadUrl: () -> String,
    private val apkDownloadPageUrl: () -> String
) {
    fun showMessageActions(anchor: View, message: ChatMessage) {
        val text = shareableMessageText(message)
        if (text.isBlank()) return
        showMessageActionPopup(anchor, message, text)
    }

    fun deleteMessage(message: ChatMessage) {
        val messages = activeConversation().messages
        val index = messages.indexOf(message)
        if (index < 0) return
        messages.removeAt(index)
        chatAdapter().notifyItemRemoved(index)
        saveConversations()
        renderConversationList()
        Toast.makeText(activity, "已删除", Toast.LENGTH_SHORT).show()
    }

    fun quoteMessage(text: String) {
        showChat()
        binding.inputEdit.setText("> ${summarize(text, 40)}\n")
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
    }

    fun showPromotionDialog() {
        shareActions().showPromotionDialog(apkDownloadUrl(), apkDownloadPageUrl())
    }
}
