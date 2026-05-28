package com.elon.app

import android.os.Handler
import android.os.Looper
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.google.gson.JsonArray
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder
import kotlin.concurrent.thread

internal class MainGroupChatActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val showFriendChat: (String, Boolean) -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val onProjectShareAction: (ChatProjectShare) -> Unit,
    private val onProjectShareLongPress: (View, ChatMessage, ChatProjectShare) -> Unit,
    private val userId: () -> String,
    private val clearPendingAttachments: () -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val onGroupSummariesChanged: () -> Unit
) {
    private val messagesByGroup = linkedMapOf<String, MutableList<ChatMessage>>()
    private val pollHandler = Handler(Looper.getMainLooper())
    private var activeGroup: AppGroup? = null
    private var activeAdapter: ChatAdapter? = null
    private var polling = false

    private val pollRunnable = object : Runnable {
        override fun run() {
            val group = activeGroup ?: return
            loadMessages(group, silent = true, scrollToBottom = false)
            if (polling) pollHandler.postDelayed(this, POLL_INTERVAL_MS)
        }
    }

    fun openGroup(group: AppGroup, animate: Boolean) {
        activeGroup = group
        val messages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
        val adapter = ChatAdapter(
            messages = messages,
            onMessageLongPress = showMessageActions,
            onProjectShareAction = onProjectShareAction,
            onProjectShareLongPress = onProjectShareLongPress
        )
        activeAdapter = adapter
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        showFriendChat(group.name, animate)
        loadMessages(group, silent = false, scrollToBottom = true)
        startPolling()
    }

    fun closeGroupChat() {
        activeGroup = null
        activeAdapter = null
        stopPolling()
    }

    fun isActive(): Boolean = activeGroup != null

    fun currentGroup(): AppGroup? = activeGroup

    fun clearCurrentMessages() {
        val group = activeGroup ?: return
        messagesByGroup[group.id]?.clear()
        activeAdapter?.notifyDataSetChanged()
        onGroupSummariesChanged()
    }

    fun resumeIfActive() {
        if (activeGroup != null) startPolling()
    }

    fun handleRealtimeMessage(groupId: String): Boolean {
        val group = activeGroup ?: return false
        if (group.id != groupId) return false
        loadMessages(group, silent = true, scrollToBottom = true, allowPendingRefresh = true)
        return true
    }

    fun stopPolling() {
        polling = false
        pollHandler.removeCallbacks(pollRunnable)
    }

    fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>): Boolean {
        val group = activeGroup ?: return false
        val attachmentsToSend = pendingAttachments.toList()
        val text = visibleTextForPendingAttachments(rawText, attachmentsToSend)
        if (text.isBlank() && attachmentsToSend.isEmpty()) return true

        val messages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
        val pending = ChatMessage(
            role = "user",
            content = text,
            attachments = chatAttachmentsFromPending(attachmentsToSend).takeIf { it.isNotEmpty() },
            sendStatus = SENDING_STATUS
        )
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        binding.chatList.scrollToPosition(messages.lastIndex)
        binding.inputEdit.text.clear()
        clearPendingAttachments()
        collapseInputComposer()

        thread {
            val result = runCatching {
                val attachments = uploadGroupAttachments(group, attachmentsToSend)
                postMessage(group, text, attachments)
            }
            activity.runOnUiThread {
                if (activeGroup?.id != group.id) return@runOnUiThread
                result.onSuccess { sentMessage ->
                    val index = messages.indexOf(pending)
                    if (index >= 0) {
                        messages[index] = sentMessage
                        activeAdapter?.notifyMessageUpdated(index)
                    }
                    loadMessages(group, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "发送失败"
                    val index = messages.indexOf(pending)
                    if (index >= 0) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    fun deleteCurrentMessage(message: ChatMessage, onDeleted: () -> Unit) {
        val group = activeGroup ?: return
        val messageId = message.id?.trim().takeIf { !it.isNullOrEmpty() }
        if (messageId == null) {
            Toast.makeText(activity, "消息尚未同步，稍后再试", Toast.LENGTH_SHORT).show()
            return
        }
        thread {
            val result = runCatching { deleteMessage(group, messageId) }
            activity.runOnUiThread {
                if (activeGroup?.id != group.id) return@runOnUiThread
                result
                    .onSuccess {
                        val messages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
                        val index = messages.indexOfFirst { it.id == messageId }
                        if (index >= 0) {
                            messages.removeAt(index)
                            activeAdapter?.notifyItemRemoved(index)
                        }
                        onGroupSummariesChanged()
                        onDeleted()
                    }
                    .onFailure { error ->
                        Toast.makeText(
                            activity,
                            error.message ?: "撤销发布失败",
                            Toast.LENGTH_LONG
                        ).show()
                    }
            }
        }
    }

    private fun startPolling() {
        if (polling) return
        polling = true
        pollHandler.removeCallbacks(pollRunnable)
        pollHandler.postDelayed(pollRunnable, POLL_INTERVAL_MS)
    }

    private fun loadMessages(
        group: AppGroup,
        silent: Boolean,
        scrollToBottom: Boolean,
        allowPendingRefresh: Boolean = false
    ) {
        val currentMessages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
        if (!allowPendingRefresh && currentMessages.any { it.sendStatus == SENDING_STATUS }) return
        thread {
            val result = runCatching { fetchMessages(group) }
            activity.runOnUiThread {
                if (activeGroup?.id != group.id) return@runOnUiThread
                result.onSuccess { remoteMessages ->
                    val changed = currentMessages.size != remoteMessages.size ||
                        currentMessages.zip(remoteMessages).any { (current, incoming) ->
                            current.role != incoming.role ||
                                current.content != incoming.content ||
                                current.senderLabel != incoming.senderLabel ||
                                current.senderAvatarDataUrl != incoming.senderAvatarDataUrl ||
                                current.attachments != incoming.attachments
                        }
                    currentMessages.clear()
                    currentMessages.addAll(remoteMessages)
                    activeAdapter?.notifyDataSetChanged()
                    if (scrollToBottom && currentMessages.isNotEmpty()) {
                        binding.chatList.scrollToPosition(currentMessages.lastIndex)
                    }
                    if (changed || !silent || allowPendingRefresh) {
                        onGroupSummariesChanged()
                    }
                }.onFailure { error ->
                    if (!silent) Toast.makeText(
                        activity,
                        error.message ?: "加载群聊消息失败",
                        Toast.LENGTH_SHORT
                    ).show()
                }
            }
        }
    }

    private fun fetchMessages(group: AppGroup): List<ChatMessage> {
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/groups/${urlPart(group.id)}/messages?limit=120")
                .get()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "加载群聊消息失败"))
            val array = JSONObject(body).optJSONArray("messages") ?: JSONArray()
            return List(array.length()) { index ->
                val json = array.optJSONObject(index) ?: JSONObject()
                groupMessageFromJson(group, json)
            }
        }
    }

    private fun uploadGroupAttachments(
        group: AppGroup,
        attachments: List<PendingAttachment>
    ): JsonArray {
        if (attachments.isEmpty()) return JsonArray()
        return uploadAttachmentRefsOrNull(
            http = http,
            serverUrl = serverUrl,
            userId = userId(),
            attachments = attachments,
            target = SendTarget(
                projectId = CHAT_ATTACHMENT_TARGET_ID,
                projectTitle = "群聊附件",
                conversationId = "group-${group.id}",
                conversationTitle = group.name
            ),
            maxAttachmentBytes = MAX_ATTACHMENT_BYTES,
            showShortToast = { message ->
                activity.runOnUiThread { Toast.makeText(activity, message, Toast.LENGTH_SHORT).show() }
            },
            showLongToast = { message ->
                activity.runOnUiThread { Toast.makeText(activity, message, Toast.LENGTH_LONG).show() }
            }
        ) ?: error("附件上传失败")
    }

    private fun postMessage(group: AppGroup, text: String, attachments: JsonArray): ChatMessage {
        val payloadJson = JSONObject().put("content", text)
        if (attachments.size() > 0) {
            payloadJson.put("attachments", JSONArray(attachments.toString()))
        }
        val payload = payloadJson.toString()
            .toRequestBody("application/json".toMediaType())
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/groups/${urlPart(group.id)}/messages")
                .post(payload)
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "发送失败"))
            val message = JSONObject(body).optJSONObject("message") ?: JSONObject()
                .put("content", text)
                .put("outgoing", true)
                .also { fallback ->
                    if (attachments.size() > 0) {
                        fallback.put("attachments", JSONArray(attachments.toString()))
                    }
                }
            return groupMessageFromJson(group, message)
        }
    }

    private fun deleteMessage(group: AppGroup, messageId: String) {
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/groups/${urlPart(group.id)}/messages/${urlPart(messageId)}")
                .delete()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "撤销发布失败"))
        }
    }

    private fun groupMessageFromJson(group: AppGroup, json: JSONObject): ChatMessage {
        val outgoing = json.optBoolean("outgoing", false)
        val senderUserId = json.optString("sender_user_id", "").trim()
        val isElAssistant = senderUserId == SOCIAL_AI_USER_ID
        val senderName = json.optString("sender_name", "").trim().takeIf { it.isNotEmpty() }
        val senderAvatar = if (outgoing || isElAssistant) {
            null
        } else {
            group.members.firstOrNull { it.id == senderUserId }?.avatarDataUrl
        }
        return ChatMessage(
            role = if (outgoing) "user" else if (isElAssistant) "ai" else "friend",
            content = json.optString("content", ""),
            attachments = chatAttachmentsFromJsonArray(json.optJSONArray("attachments")).takeIf { it.isNotEmpty() },
            senderLabel = if (outgoing || isElAssistant) null else senderName,
            id = json.optString("id").trim().takeIf { it.isNotEmpty() },
            senderAvatarDataUrl = senderAvatar
        )
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

    private companion object {
        const val POLL_INTERVAL_MS = 3000L
        const val SENDING_STATUS = "发送中..."
        const val MAX_ATTACHMENT_BYTES = 12 * 1024 * 1024
        const val SOCIAL_AI_USER_ID = "usr_elon_ai"
    }
}
