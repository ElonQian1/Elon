package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.net.URLEncoder
import java.time.Instant
import kotlin.concurrent.thread

internal class MainMessageActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val activeProject: () -> AppProject,
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
        if (text.isBlank() && message.attachments.isNullOrEmpty()) return
        showMessageActionPopup(anchor, message, text)
    }

    fun canRecallMessage(message: ChatMessage): Boolean {
        return activeConversation().messages.contains(message) && message.canRecallNow()
    }

    fun recallMessage(message: ChatMessage) {
        val project = activeProject()
        val conversation = activeConversation()
        val messages = conversation.messages
        val index = messages.indexOf(message)
        if (index < 0 || !message.canRecallNow()) return
        val messageId = message.id?.trim().takeIf { !it.isNullOrEmpty() }
        if (messageId == null) {
            markMessageRecalled(message)
            chatAdapter().notifyMessageUpdated(index)
            saveConversations()
            renderConversationList()
            Toast.makeText(activity, "已撤回", Toast.LENGTH_SHORT).show()
            return
        }
        thread {
            val result = runCatching {
                recallRemoteProjectConversationMessage(project.id, conversation.id, messageId)
            }
            activity.runOnUiThread {
                val currentMessages = activeConversation().messages
                val currentIndex = currentMessages.indexOfFirst { it === message || it.id == messageId }
                result
                    .onSuccess {
                        if (currentIndex >= 0) {
                            markMessageRecalled(currentMessages[currentIndex])
                            chatAdapter().notifyMessageUpdated(currentIndex)
                        }
                        saveConversations()
                        renderConversationList()
                        Toast.makeText(activity, "已撤回", Toast.LENGTH_SHORT).show()
                    }
                    .onFailure { error ->
                        Toast.makeText(activity, error.message ?: "撤回失败", Toast.LENGTH_LONG).show()
                    }
            }
        }
    }

    fun deleteMessage(message: ChatMessage) {
        val messages = activeConversation().messages
        val index = messages.indexOf(message)
        if (index < 0) return
        messages.removeAt(index)
        chatAdapter().notifyMessageRemoved(index)
        saveConversations()
        renderConversationList()
        Toast.makeText(activity, "已删除", Toast.LENGTH_SHORT).show()
    }

    private fun markMessageRecalled(message: ChatMessage) {
        message.content = ""
        message.attachments = null
        message.apkUrl = null
        message.codexThreadUri = null
        message.evidenceTitle = null
        message.evidenceDetails = null
        message.finalReply = false
        message.modelUsed = null
        message.nodeId = null
        message.sendStatus = null
        message.recalledAt = message.recalledAt ?: Instant.now().toString()
        message.recalledBy = message.recalledBy ?: AuthManager.userId(activity)
    }

    private fun recallRemoteProjectConversationMessage(projectId: String, conversationId: String, messageId: String) {
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url(
                    "$serverUrl/api/projects/${urlPart(projectId)}/conversations/${urlPart(conversationId)}/messages/${urlPart(messageId)}"
                )
                .delete()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "撤回失败"))
        }
    }

    private fun readErrorMessage(body: String, fallback: String): String {
        if (body.isBlank()) return fallback
        return runCatching {
            JSONObject(body).optString("error", "").ifBlank { fallback }
        }.getOrDefault(fallback)
    }

    private fun urlPart(value: String): String {
        return URLEncoder.encode(value, Charsets.UTF_8.name())
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
