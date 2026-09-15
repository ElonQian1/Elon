package com.elon.app

import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.TextWatcher
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
import java.time.Instant
import kotlin.concurrent.thread

internal class MainFriendChatActions(
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
    private val onFriendSummariesChanged: () -> Unit,
    private val onActiveFriendChanged: (AppFriend?) -> Unit = {},
) {
    private val reader = SocialChatReadChannel(activity, http, serverUrl) { key, rows ->
        protectSocialChatRows(messagesByFriend[key.removePrefix("friend:")].orEmpty(), rows)
    }
    private var owner = AuthManager.userId(activity)
    private val messagesByFriend = linkedMapOf<String, MutableList<ChatMessage>>()
    private val pollHandler = Handler(Looper.getMainLooper())
    private var activeFriend: AppFriend? = null
    private var activeAdapter: ChatAdapter? = null
    private var polling = false
    private var foreground = true

    // 正在输入提示：3秒无新事件后自动隐藏
    private val typingHandler = Handler(Looper.getMainLooper())
    private val hideTypingRunnable = Runnable { restoreTitleFromTyping() }
    private var savedFriendTitle = ""
    private var typingShowing = false

    // 发送打字状态防抖：2秒内最多发一次
    private var lastTypingSentMs = 0L
    private val typingWatcher = object : TextWatcher {
        override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
        override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
        override fun afterTextChanged(s: Editable?) {
            val friend = activeFriend ?: return
            val now = System.currentTimeMillis()
            if (now - lastTypingSentMs < 2_000L) return
            lastTypingSentMs = now
            val payload = JSONObject().apply {
                put("type", "typing")
                put("toUserId", friend.id)
            }.toString()
            (activity.application as? ElonApplication)?.globalWs?.send(payload)
        }
    }

    private val pollRunnable = object : Runnable {
        override fun run() {
            val friend = activeFriend ?: return
            loadMessages(friend, silent = true, scrollToBottom = false)
            if (polling) pollHandler.postDelayed(this, POLL_INTERVAL_MS)
        }
    }

    fun openFriend(friend: AppFriend, animate: Boolean) {
        ensureOwner(); foreground = true; reader.cancel()
        activeFriend = friend
        val messages = messagesByFriend.getOrPut(friend.id) { mutableListOf() }
        val adapter = createAdapter(messages)
        activeAdapter = adapter
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        if (messages.isNotEmpty()) {
            binding.chatList.jumpToLatestMessageBeforeNextDraw()
        }
        savedFriendTitle = friend.name
        binding.inputEdit.removeTextChangedListener(typingWatcher)
        binding.inputEdit.addTextChangedListener(typingWatcher)
        showFriendChat(friend.name, animate)
        onActiveFriendChanged(friend)
        loadMessages(friend, silent = false, scrollToBottom = true)
        startPolling()
    }

    fun closeFriendChat() {
        showSocialChatStatus(binding, null)
        binding.inputEdit.removeTextChangedListener(typingWatcher)
        typingHandler.removeCallbacks(hideTypingRunnable)
        typingShowing = false
        savedFriendTitle = ""
        activeFriend = null
        onActiveFriendChanged(null)
        activeAdapter = null
        stopPolling()
    }

    fun isActive(): Boolean = activeFriend != null

    fun isDirectSocialAiActive(): Boolean = activeFriend.isSocialAi()

    fun currentFriend(): AppFriend? = activeFriend

    fun currentMessages(): List<ChatMessage> = activeFriend?.id
        ?.let(messagesByFriend::get)
        ?.toList()
        .orEmpty()

    fun rebindCurrentFriend() {
        foreground = true
        if (!ensureOwner()) return
        val friend = activeFriend ?: return
        val messages = messagesByFriend.getOrPut(friend.id) { mutableListOf() }
        val adapter = createAdapter(messages)
        activeAdapter = adapter
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        savedFriendTitle = friend.name
        binding.topTitleText.text = friend.name
        binding.inputEdit.removeTextChangedListener(typingWatcher)
        binding.inputEdit.addTextChangedListener(typingWatcher)
        if (messages.isNotEmpty()) binding.chatList.jumpToLatestMessageBeforeNextDraw()
        loadMessages(friend, silent = true, scrollToBottom = false)
        startPolling()
    }

    fun suspendForExternalChat() {
        binding.inputEdit.removeTextChangedListener(typingWatcher)
        typingHandler.removeCallbacks(hideTypingRunnable)
        typingShowing = false
        stopPolling()
    }

    fun clearCurrentMessages() {
        val friend = activeFriend ?: return
        reader.invalidate("friend:${friend.id}")
        messagesByFriend[friend.id]?.clear()
        activeAdapter?.notifyDataSetChanged()
        onFriendSummariesChanged()
    }

    fun resumeIfActive() {
        foreground = true
        if (!ensureOwner()) return
        val friend = activeFriend ?: return
        reader.cancel()
        loadMessages(friend, silent = false, scrollToBottom = false)
        startPolling()
    }

    fun handleRealtimeMessage(fromUserId: String): Boolean {
        val friend = activeFriend ?: return false
        if (friend.id != fromUserId) return false
        loadMessages(friend, silent = true, scrollToBottom = false, allowPendingRefresh = true)
        return true
    }

    /** 收到好友的 typing 事件：显示"正在输入..."，3 秒后自动恢复好友名称 */
    fun handleTypingEvent(fromUserId: String) {
        val friend = activeFriend ?: return
        if (friend.id != fromUserId) return
        typingShowing = true
        binding.topTitleText.text = "正在输入…"
        typingHandler.removeCallbacks(hideTypingRunnable)
        typingHandler.postDelayed(hideTypingRunnable, 3_000L)
    }

    private fun restoreTitleFromTyping() {
        if (!typingShowing) return
        typingShowing = false
        binding.topTitleText.text = savedFriendTitle
    }

    /** 收到好友的 read_receipt 事件：标记该好友已读的所有自己发出的消息 */
    fun handleReadReceiptEvent(fromUserId: String, lastReadAt: String) {
        val lastReadAtMs = parseChatMessageCreatedAt(lastReadAt) ?: return
        val messages = messagesByFriend[fromUserId] ?: return
        var changed = false
        for (msg in messages) {
            if (msg.role == "user" && !msg.isRead && msg.createdAtMs in 1..lastReadAtMs) {
                msg.isRead = true
                changed = true
            }
        }
        if (changed && activeFriend?.id == fromUserId) {
            activeAdapter?.notifyDataSetChanged()
        }
    }

    fun stopPolling() {
        foreground = false
        reader.cancel()
        polling = false
        pollHandler.removeCallbacks(pollRunnable)
    }

    private fun createAdapter(messages: MutableList<ChatMessage>): ChatAdapter = ChatAdapter(
        messages = messages,
        onMessageLongPress = showMessageActions,
        onProjectShareAction = onProjectShareAction,
        onProjectShareLongPress = onProjectShareLongPress,
    )

    fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>): Boolean {
        val friend = activeFriend ?: return false
        val attachmentsToSend = pendingAttachments.toList()
        val localAttachments = chatAttachmentsFromPending(attachmentsToSend)
        val text = visibleTextForPendingAttachments(rawText, attachmentsToSend)
        if (text.isBlank() && attachmentsToSend.isEmpty()) return true

        val messages = messagesByFriend.getOrPut(friend.id) { mutableListOf() }
        val pending = ChatMessage(
            role = "user",
            content = text,
            attachments = localAttachments.takeIf { it.isNotEmpty() },
            sendStatus = SENDING_STATUS
        )
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        binding.chatList.scrollToPosition(messages.lastIndex)
        binding.inputEdit.text.clear()
        clearPendingAttachments()
        collapseInputComposer()

        val sendingSession = socialSession(activity)
        thread {
            val result = runCatching {
                val attachments = uploadFriendAttachments(friend, attachmentsToSend)
                postMessage(friend, text, attachments)
            }
            activity.runOnUiThread {
                if (sendingSession != socialSession(activity)) return@runOnUiThread
                result.onSuccess { sentMessage ->
                    sentMessage.withMissingImageAnnotationsFrom(localAttachments)
                    completeSocialChatSend(messages, pending, sentMessage)
                    reader.invalidate("friend:${friend.id}")
                    if (activeFriend?.id == friend.id) {
                        activeAdapter?.notifyDataSetChanged()
                        loadMessages(friend, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                    }
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "发送失败"
                    val index = messages.indexOf(pending)
                    if (index >= 0 && activeFriend?.id == friend.id) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    fun trySendForwardedMessage(source: ChatMessage): Boolean {
        val friend = activeFriend ?: return false
        val text = source.content.trim()
        val attachments = source.attachments.orEmpty().map { it.copy() }
        if (text.isBlank() && attachments.isEmpty()) return true

        val messages = messagesByFriend.getOrPut(friend.id) { mutableListOf() }
        val pending = ChatMessage(
            role = "user",
            content = text,
            attachments = attachments.takeIf { it.isNotEmpty() },
            sendStatus = SENDING_STATUS
        )
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        binding.chatList.scrollToPosition(messages.lastIndex)
        collapseInputComposer()

        val sendingSession = socialSession(activity)
        thread {
            val result = runCatching {
                postMessage(friend, text, chatAttachmentRefsFromChatAttachments(attachments))
            }
            activity.runOnUiThread {
                if (sendingSession != socialSession(activity)) return@runOnUiThread
                result.onSuccess { sentMessage ->
                    completeSocialChatSend(messages, pending, sentMessage)
                    reader.invalidate("friend:${friend.id}")
                    if (activeFriend?.id == friend.id) {
                        activeAdapter?.notifyDataSetChanged()
                        loadMessages(friend, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                    }
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "发送失败"
                    val index = messages.indexOf(pending)
                    if (index >= 0 && activeFriend?.id == friend.id) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    fun requestAiReply(message: ChatMessage) {
        val friend = activeFriend ?: return
        val messageId = message.id?.trim().takeIf { !it.isNullOrEmpty() }
        if (messageId == null) {
            Toast.makeText(activity, "消息尚未同步，稍后再试", Toast.LENGTH_SHORT).show()
            return
        }
        if (message.content.isBlank()) {
            Toast.makeText(activity, "这条消息没有可供 AI 回复的文本", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "EL 正在回复这条消息", Toast.LENGTH_SHORT).show()
        thread {
            val result = runCatching {
                requestFriendSelectedAiReply(http, serverUrl, activity, friend.id, messageId)
            }
            activity.runOnUiThread {
                if (activeFriend?.id != friend.id) return@runOnUiThread
                result
                    .onSuccess {
                        pollHandler.postDelayed({
                            if (activeFriend?.id == friend.id) {
                                loadMessages(friend, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                            }
                        }, AI_REPLY_REFRESH_DELAY_MS)
                    }
                    .onFailure { error ->
                        Toast.makeText(activity, error.message ?: "AI回复触发失败", Toast.LENGTH_LONG).show()
                    }
            }
        }
    }

    fun deleteCurrentMessage(message: ChatMessage, onDeleted: () -> Unit) {
        recallCurrentMessage(message, onDeleted)
    }

    fun recallCurrentMessage(message: ChatMessage, onRecalled: () -> Unit = {}) {
        val friend = activeFriend ?: return
        val messageId = message.id?.trim().takeIf { !it.isNullOrEmpty() }
        if (messageId == null) {
            Toast.makeText(activity, "消息尚未同步，稍后再试", Toast.LENGTH_SHORT).show()
            return
        }
        thread {
            val result = runCatching { deleteMessage(friend, messageId) }
            activity.runOnUiThread {
                if (activeFriend?.id != friend.id) return@runOnUiThread
                result
                    .onSuccess {
                        val messages = messagesByFriend.getOrPut(friend.id) { mutableListOf() }
                        val index = messages.indexOfFirst { it.id == messageId }
                        if (index >= 0) {
                            reader.invalidate("friend:${friend.id}")
                            markMessageRecalled(messages[index])
                            activeAdapter?.notifyMessageUpdated(index)
                        }
                        onFriendSummariesChanged()
                        onRecalled()
                        Toast.makeText(activity, "已撤回", Toast.LENGTH_SHORT).show()
                    }
                    .onFailure { error ->
                        Toast.makeText(
                            activity,
                            error.message ?: "撤回失败",
                            Toast.LENGTH_LONG
                        ).show()
                    }
            }
        }
    }

    fun removeProjectShareCards(projectIds: Set<String>): Int {
        val ids = projectIds.map { it.trim() }.filter { it.isNotEmpty() }.toSet()
        if (ids.isEmpty()) return 0
        var removed = 0
        var activeChanged = false
        val activeId = activeFriend?.id
        messagesByFriend.forEach { (friendId, messages) ->
            val before = messages.size
            messages.removeAll { message ->
                message.role == "user" && parseChatProjectShareMessage(message.content)?.id in ids
            }
            val removedHere = before - messages.size
            if (removedHere > 0) {
                removed += removedHere
                if (friendId == activeId) activeChanged = true
            }
        }
        if (activeChanged) activeAdapter?.notifyDataSetChanged()
        if (removed > 0) onFriendSummariesChanged()
        return removed
    }

    private fun startPolling() {
        if (polling) return
        polling = true
        pollHandler.removeCallbacks(pollRunnable)
        pollHandler.postDelayed(pollRunnable, POLL_INTERVAL_MS)
    }

    private fun loadMessages(
        friend: AppFriend,
        silent: Boolean,
        scrollToBottom: Boolean,
        allowPendingRefresh: Boolean = false
    ) {
        if (!foreground || !ensureOwner() || activeFriend?.id != friend.id) return
        val currentMessages = messagesByFriend.getOrPut(friend.id) { mutableListOf() }
        fun apply(rows: JSONArray) {
            val remote = List(rows.length()) { friendMessageFromJson(friend, rows.getJSONObject(it)) }
            val merged = mergeSocialChatMessages(currentMessages, remote.withMissingImageAnnotationsFromCurrent(currentMessages))
            val changed = currentMessages != merged
            val follow = scrollToBottom || !binding.chatList.canScrollVertically(1)
            currentMessages.clear(); currentMessages.addAll(merged)

            if (changed) activeAdapter?.notifyDataSetChanged()
            if (follow && changed && currentMessages.isNotEmpty()) binding.chatList.jumpToLatestMessageBeforeNextDraw()
            showSocialChatStatus(binding, if (currentMessages.isEmpty()) "还没有消息" else null)
            if (changed || !silent || allowPendingRefresh) onFriendSummariesChanged()
        }
        if (currentMessages.isEmpty()) showSocialChatStatus(binding, "正在同步好友消息…")
        reader.read("friend:${friend.id}", "/api/me/friends/${urlPart(friend.id)}/messages?limit=120&preserve_unread=false", "messages",
            hydrate = currentMessages.isEmpty(), cached = { if (it.length() > 0) apply(it) }, value = ::apply,
            error = { failure ->
                if (failure.socialAccessDenied()) { currentMessages.clear(); activeAdapter?.notifyDataSetChanged() }
                if (currentMessages.isEmpty()) showSocialChatStatus(binding, "${if (failure.socialAccessDenied()) "无法访问此会话，请检查账号或成员权限" else "同步暂时失败"}\n点击重试") { loadMessages(friend, false, false) }
                else if (!silent) Toast.makeText(activity, "同步暂时失败，已保留现有消息，将自动重试", Toast.LENGTH_SHORT).show()
            })
    }

    private fun ensureOwner(): Boolean {
        val next = AuthManager.userId(activity)
        if (owner == next) return true
        messagesByFriend.values.forEach { it.clear() }; messagesByFriend.clear()
        activeAdapter?.notifyDataSetChanged(); closeFriendChat(); owner = next
        return false
    }

    private fun uploadFriendAttachments(
        friend: AppFriend,
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
                projectTitle = "好友聊天附件",
                conversationId = "friend-${friend.id}",
                conversationTitle = friend.name
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

    private fun postMessage(friend: AppFriend, text: String, attachments: JsonArray): ChatMessage {
        val payloadJson = JSONObject().put("content", text)
        if (attachments.size() > 0) {
            payloadJson.put("attachments", JSONArray(attachments.toString()))
        }
        val payload = payloadJson.toString()
            .toRequestBody("application/json".toMediaType())
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/friends/${urlPart(friend.id)}/messages")
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
            return friendMessageFromJson(friend, message)
        }
    }

    private fun deleteMessage(friend: AppFriend, messageId: String) {
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/friends/${urlPart(friend.id)}/messages/${urlPart(messageId)}")
                .delete()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "撤销发布失败"))
        }
    }

    private fun markMessageRecalled(message: ChatMessage) {
        message.content = ""
        message.attachments = null
        message.sendStatus = null
        message.recalledAt = message.recalledAt ?: Instant.now().toString()
        message.recalledBy = message.recalledBy ?: userId()
    }

    private fun friendMessageFromJson(friend: AppFriend, json: JSONObject): ChatMessage {
        val outgoing = json.optBoolean("outgoing", false)
        val senderUserId = json.optString("sender_user_id", "").trim()
        val isElAssistant = SocialAiIdentity.matches(senderUserId)
        val senderName = json.optString("sender_name", "").trim().takeIf { it.isNotEmpty() }
        return ChatMessage(
            role = if (outgoing) "user" else if (isElAssistant) "ai" else "friend",
            content = json.optString("content", ""),
            attachments = chatAttachmentsFromJsonArray(json.optJSONArray("attachments")).takeIf { it.isNotEmpty() },
            senderLabel = if (outgoing || isElAssistant) null else senderName ?: friend.name,
            id = json.optString("id").trim().takeIf { it.isNotEmpty() },
            senderAvatarDataUrl = if (outgoing || isElAssistant) null else friend.avatarDataUrl,
            createdAtMs = parseChatMessageCreatedAt(json.optString("created_at", "")) ?: 0L,
            recalledAt = json.cleanRecallString("recalled_at"),
            recalledBy = json.cleanRecallString("recalled_by")
        )
    }

    private fun JSONObject.cleanRecallString(key: String): String? {
        return optString(key, "")
            .trim()
            .takeIf { it.isNotEmpty() && !it.equals("null", ignoreCase = true) }
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
        const val AI_REPLY_REFRESH_DELAY_MS = 1200L
        const val SENDING_STATUS = "发送中..."
        const val MAX_ATTACHMENT_BYTES = 12 * 1024 * 1024
    }
}
