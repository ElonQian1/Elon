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
import java.time.Instant
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
    private val inputFocusActions: () -> MainInputFocusActions,
    private val onGroupSummariesChanged: () -> Unit,
    private val aiComposer: GroupAiComposer,
) {
    private val reader = SocialChatReadChannel(activity, http, serverUrl) { key, rows ->
        protectSocialChatRows(messagesByGroup[key.removePrefix("group:")].orEmpty(), rows)
    }
    private var owner = AuthManager.userId(activity)
    private val messagesByGroup = linkedMapOf<String, MutableList<ChatMessage>>()
    private val readPositions = linkedMapOf<String, android.os.Parcelable>()
    private var pendingReadPosition: android.os.Parcelable? = null
    private val pollHandler = Handler(Looper.getMainLooper())
    private var activeGroup: AppGroup? = null
    private var activeAdapter: ChatAdapter? = null
    private var polling = false
    private var foreground = true
    private val webAi by lazy {
        GroupWebAiFeature(activity, binding.root, http, serverUrl, userId) { handleRealtimeMessage(it) }
    }
    private val mentions by lazy { GroupMentionController(activity, binding.inputEdit, http, serverUrl, userId) { inputFocusActions().focusInputComposer() } }
    private val revisions by lazy { GroupMessageRevisionController(activity, http, serverUrl, { activeGroup?.id == it }, ::applyRevision) }

    fun revisionActions(message: ChatMessage): List<TopAction> {
        if (com.elon.app.articles.ArticleApi.reference(message.content) != null) return emptyList()
        val group = activeGroup ?: return emptyList()
        if (messagesByGroup[group.id]?.none { it.id == message.id } != false) return emptyList()
        return revisions.actions(group.id, message)
    }

    private fun applyRevision(groupId: String, edited: JSONObject) {
        val messages = messagesByGroup[groupId] ?: return
        val index = messages.indexOfFirst { it.id == edited.optString("id") }
        if (index < 0) return
        val message = messages[index]
        if (!message.recalledAt.isNullOrBlank() || message.revision > edited.optLong("revision", 1)) return
        reader.invalidate("group:$groupId")
        message.content = edited.optString("content")
        message.revision = edited.optLong("revision", 1)
        message.editedAt = edited.optString("edited_at").takeIf { it != "null" && it.isNotBlank() }
        if (activeGroup?.id == groupId) activeAdapter?.notifyMessageUpdated(index)
    }
    private val summaryPosts by lazy {
        MainGroupSummaryPosts(
            activity = activity,
            binding = binding,
            http = http,
            serverUrl = serverUrl,
            onPostsChanged = onGroupSummariesChanged
        )
    }

    private val pollRunnable = object : Runnable {
        override fun run() {
            val group = activeGroup ?: return
            loadMessages(group, silent = true, scrollToBottom = false)
            if (polling) pollHandler.postDelayed(this, POLL_INTERVAL_MS)
        }
    }

    fun openGroup(group: AppGroup, animate: Boolean, restorePosition: Boolean = false) {
        ensureOwner(); foreground = true; reader.cancel()
        pendingReadPosition = if (restorePosition) readPositions[group.id] else null
        revisions.close()
        activeGroup = group
        mentions.setGroup(group)
        val messages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
        val adapter = ChatAdapter(
            messages = messages,
            onMessageLongPress = showMessageActions,
            onProjectShareAction = onProjectShareAction,
            onProjectShareLongPress = onProjectShareLongPress
        )
        activeAdapter = adapter
        adapter.onSenderAvatarLongPress = mentions::mentionSender
        adapter.onMessageHistory = { revisions.history(group.id, it) }
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        if (messages.isNotEmpty() && pendingReadPosition == null) {
            binding.chatList.jumpToLatestMessageBeforeNextDraw()
        }
        showFriendChat(group.name, animate)
        aiComposer.open(group.id)
        summaryPosts.openGroup(group)
        com.elon.app.articles.ArticleCardViews.openGroup(binding.groupSummaryStrip, group.id)
        pendingReadPosition?.let { binding.chatList.layoutManager?.onRestoreInstanceState(it) }
        loadMessages(group, silent = false, scrollToBottom = !restorePosition)
        startPolling()
        webAi.recover()
    }

    fun closeGroupChat() {
        activeGroup?.id?.let { id -> binding.chatList.layoutManager?.onSaveInstanceState()?.let { readPositions[id] = it } }
        while (readPositions.size > 30) readPositions.remove(readPositions.keys.first())
        showSocialChatStatus(binding, null)
        aiComposer.close()
        revisions.close()
        activeGroup = null
        mentions.setGroup(null)
        activeAdapter = null
        summaryPosts.clear()
        stopPolling()
    }

    fun isActive(): Boolean = activeGroup != null

    fun currentGroup(): AppGroup? = activeGroup

    fun showSummaryPosts(group: AppGroup? = activeGroup) {
        summaryPosts.showPosts(group)
    }

    fun clearCurrentMessages() {
        val group = activeGroup ?: return
        reader.invalidate("group:${group.id}")
        messagesByGroup[group.id]?.clear()
        activeAdapter?.notifyDataSetChanged()
        onGroupSummariesChanged()
    }

    fun resumeIfActive() {
        foreground = true
        if (!ensureOwner()) return
        val group = activeGroup ?: return
        reader.cancel()
        loadMessages(group, silent = false, scrollToBottom = false)
        startPolling()
    }

    fun handleRealtimeMessage(groupId: String): Boolean {
        val group = activeGroup ?: return false
        if (group.id != groupId) return false
        loadMessages(group, silent = true, scrollToBottom = false, allowPendingRefresh = true)
        return true
    }

    fun stopPolling() {
        foreground = false
        reader.cancel()
        showSocialChatStatus(binding, null)
        polling = false
        pollHandler.removeCallbacks(pollRunnable)
    }

    fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>, webAiConfirmed: Boolean = false,
        configuration: GroupAiConfiguration = aiComposer.configuration()): Boolean {
        val group = activeGroup ?: return false
        if (!configuration.usesWebAi && GroupWebAiFeature.mentionsAi(rawText) && !webAiConfirmed) {
            if (webAi.beginWork()) trySendMessage(rawText, pendingAttachments, true, configuration)
            return true
        }
        if (configuration.usesWebAi && GroupWebAiFeature.mentionsAi(rawText) && !webAiConfirmed) {
            webAi.confirm(group, configuration) {
                if (activeGroup?.id == group.id && binding.inputEdit.text.toString().trim() == rawText.trim()) trySendMessage(rawText, pendingAttachments, true, configuration)
                else webAi.release()
            }
            return true
        }
        val attachmentsToSend = pendingAttachments.toList()
        val localAttachments = chatAttachmentsFromPending(attachmentsToSend)
        val text = visibleTextForPendingAttachments(rawText, attachmentsToSend)
        if (text.isBlank() && attachmentsToSend.isEmpty()) return true

        val messages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
        val pending = ChatMessage(
            role = "user",
            content = text,
            attachments = localAttachments.takeIf { it.isNotEmpty() },
            sendStatus = SENDING_STATUS
        )
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        showSocialChatStatus(binding, null)
        binding.chatList.scrollToPosition(messages.lastIndex)
        binding.inputEdit.text.clear()
        clearPendingAttachments()
        inputFocusActions().collapseInputComposer()

        val sendingSession = socialSession(activity)
        thread {
            val result = runCatching {
                val attachments = uploadGroupAttachments(group, attachmentsToSend)
                postMessage(group, text, attachments, webAiConfirmed, configuration)
            }
            activity.runOnUiThread {
                if (result.isFailure && webAiConfirmed) webAi.release()
                if (sendingSession != socialSession(activity)) return@runOnUiThread
                result.onSuccess { sentMessage ->
                    sentMessage.withMissingImageAnnotationsFrom(localAttachments)
                    completeSocialChatSend(messages, pending, sentMessage)
                    reader.invalidate("group:${group.id}")
                    if (activeGroup?.id == group.id) {
                        activeAdapter?.notifyDataSetChanged()
                        loadMessages(group, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                    }
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "发送失败"
                    val index = messages.indexOf(pending)
                    if (index >= 0 && activeGroup?.id == group.id) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    fun trySendForwardedMessage(source: ChatMessage, webAiConfirmed: Boolean = false,
        configuration: GroupAiConfiguration = aiComposer.configuration()): Boolean {
        val group = activeGroup ?: return false
        if (!configuration.usesWebAi && GroupWebAiFeature.mentionsAi(source.content) && !webAiConfirmed) {
            if (webAi.beginWork()) trySendForwardedMessage(source, true, configuration)
            return true
        }
        if (configuration.usesWebAi && GroupWebAiFeature.mentionsAi(source.content) && !webAiConfirmed) {
            webAi.confirm(group, configuration) {
                if (activeGroup?.id == group.id) trySendForwardedMessage(source, true, configuration)
                else webAi.release()
            }
            return true
        }
        val text = source.content.trim()
        val attachments = source.attachments.orEmpty().map { it.copy() }
        if (text.isBlank() && attachments.isEmpty()) return true

        val messages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
        val pending = ChatMessage(
            role = "user",
            content = text,
            attachments = attachments.takeIf { it.isNotEmpty() },
            sendStatus = SENDING_STATUS
        )
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        showSocialChatStatus(binding, null)
        binding.chatList.scrollToPosition(messages.lastIndex)
        inputFocusActions().collapseInputComposer()

        val sendingSession = socialSession(activity)
        thread {
            val result = runCatching {
                postMessage(group, text, chatAttachmentRefsFromChatAttachments(attachments), webAiConfirmed, configuration)
            }
            activity.runOnUiThread {
                if (result.isFailure && webAiConfirmed) webAi.release()
                if (sendingSession != socialSession(activity)) return@runOnUiThread
                result.onSuccess { sentMessage ->
                    completeSocialChatSend(messages, pending, sentMessage)
                    reader.invalidate("group:${group.id}")
                    if (activeGroup?.id == group.id) {
                        activeAdapter?.notifyDataSetChanged()
                        loadMessages(group, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                    }
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "发送失败"
                    val index = messages.indexOf(pending)
                    if (index >= 0 && activeGroup?.id == group.id) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    fun requestAiReply(message: ChatMessage) {
        val group = activeGroup ?: return
        val messageId = message.id?.trim().takeIf { !it.isNullOrEmpty() }
        if (messageId == null) {
            Toast.makeText(activity, "消息尚未同步，稍后再试", Toast.LENGTH_SHORT).show()
            return
        }
        if (message.content.isBlank()) {
            Toast.makeText(activity, "这条消息没有可供 AI 回复的文本", Toast.LENGTH_SHORT).show()
            return
        }
        val configuration = aiComposer.configuration()
        if (configuration.usesWebAi) analyzeSelectedMessages(listOf(message)) {}
        else webAi.prepareWork(group, messageId, configuration)
    }

    fun analyzeSelectedMessages(messages: List<ChatMessage>, started: () -> Unit) {
        val group = activeGroup ?: return
        val owner = socialSession(activity)
        val configuration = aiComposer.configuration()
        if (!configuration.usesWebAi) {
            Toast.makeText(activity, "请先在 AI 设置中选择网页 AI，再分析所选消息", Toast.LENGTH_LONG).show()
            return
        }
        GroupAiSelectionPreview.show(activity, messages, configuration.engine == GroupAiEngine.CHATGPT) { source, selection, memory ->
            if (owner == socialSession(activity) && activeGroup?.id == group.id &&
                webAi.prepareSelected(group, source, selection, configuration, memory)) started()
        }
    }

    fun deleteCurrentMessage(message: ChatMessage, onDeleted: () -> Unit) {
        recallCurrentMessage(message, onDeleted)
    }

    fun recallCurrentMessage(message: ChatMessage, onRecalled: () -> Unit = {}) {
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
                            reader.invalidate("group:${group.id}")
                            markMessageRecalled(messages[index])
                            activeAdapter?.notifyMessageUpdated(index)
                        }
                        onGroupSummariesChanged()
                        onRecalled()
                        Toast.makeText(activity, "已撤回", Toast.LENGTH_SHORT).show()
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

    fun removeProjectShareCards(projectIds: Set<String>): Int {
        val ids = projectIds.map { it.trim() }.filter { it.isNotEmpty() }.toSet()
        if (ids.isEmpty()) return 0
        var removed = 0
        var activeChanged = false
        val activeId = activeGroup?.id
        messagesByGroup.forEach { (groupId, messages) ->
            val before = messages.size
            messages.removeAll { message ->
                message.role == "user" && parseChatProjectShareMessage(message.content)?.id in ids
            }
            val removedHere = before - messages.size
            if (removedHere > 0) {
                removed += removedHere
                if (groupId == activeId) activeChanged = true
            }
        }
        if (activeChanged) activeAdapter?.notifyDataSetChanged()
        if (removed > 0) onGroupSummariesChanged()
        return removed
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
        if (!foreground || !ensureOwner() || activeGroup?.id != group.id) return
        val currentMessages = messagesByGroup.getOrPut(group.id) { mutableListOf() }
        fun apply(rows: JSONArray) {
            val remote = List(rows.length()) { groupMessageFromJson(group, rows.getJSONObject(it)) }
            val merged = mergeSocialChatMessages(currentMessages, remote.withMissingImageAnnotationsFromCurrent(currentMessages))
            val changed = currentMessages != merged
            val follow = pendingReadPosition == null && (scrollToBottom || !binding.chatList.canScrollVertically(1))
            currentMessages.clear(); currentMessages.addAll(merged)
            revisions.onMessagesChanged(merged)
            if (changed) activeAdapter?.notifyDataSetChanged()
            pendingReadPosition?.let { binding.chatList.layoutManager?.onRestoreInstanceState(it); pendingReadPosition = null }
            if (follow && changed && currentMessages.isNotEmpty()) binding.chatList.jumpToLatestMessageBeforeNextDraw()
            showSocialChatStatus(binding, if (currentMessages.isEmpty()) "还没有消息" else null)
            if (changed || !silent || allowPendingRefresh) onGroupSummariesChanged()
        }
        showSocialChatStatus(binding, if (currentMessages.isEmpty()) "正在同步群聊消息…" else null)
        reader.read("group:${group.id}", "/api/me/groups/${urlPart(group.id)}/messages?limit=120&preserve_unread=false", "messages",
            hydrate = currentMessages.isEmpty(), cached = { if (it.length() > 0) apply(it) }, value = ::apply,
            error = { failure ->
                if (failure.socialAccessDenied()) { currentMessages.clear(); activeAdapter?.notifyDataSetChanged() }
                if (currentMessages.isEmpty()) showSocialChatStatus(binding, "${if (failure.socialAccessDenied()) "无法访问此会话，请检查账号或成员权限" else "同步暂时失败"}\n点击重试") { loadMessages(group, false, false) }
                else if (!silent) Toast.makeText(activity, "同步暂时失败，已保留现有消息，将自动重试", Toast.LENGTH_SHORT).show()
            })
    }

    private fun ensureOwner(): Boolean {
        val next = AuthManager.userId(activity)
        if (owner == next) return true
        messagesByGroup.values.forEach { it.clear() }; messagesByGroup.clear()
        activeAdapter?.notifyDataSetChanged(); closeGroupChat(); owner = next
        readPositions.clear(); pendingReadPosition = null
        return false
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

    private fun postMessage(group: AppGroup, text: String, attachments: JsonArray, useWebAi: Boolean = false,
        configuration: GroupAiConfiguration = GroupAiConfiguration()): ChatMessage {
        val payloadJson = JSONObject().put("content", text)
        if (attachments.size() > 0) {
            payloadJson.put("attachments", JSONArray(attachments.toString()))
        }
        if (useWebAi) return groupMessageFromJson(group, webAi.send(group, payloadJson, configuration))
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

    private fun markMessageRecalled(message: ChatMessage) {
        message.content = ""
        message.attachments = null
        message.sendStatus = null
        message.recalledAt = message.recalledAt ?: Instant.now().toString()
        message.recalledBy = message.recalledBy ?: userId()
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
            senderLabel = if (isElAssistant) ELON_CHAT_SENDER_NAME else senderName
                ?: group.members.firstOrNull { it.id == senderUserId }?.displayName,
            id = json.optString("id").trim().takeIf { it.isNotEmpty() },
            senderAvatarDataUrl = senderAvatar,
            senderUserId = senderUserId,
            groupAiReply = json.optJSONObject("ai_reply")?.toString(),
            createdAtMs = parseChatMessageCreatedAt(json.optString("created_at", "")) ?: 0L,
            recalledAt = json.cleanRecallString("recalled_at"),
            revision = json.optLong("revision", 1).coerceAtLeast(1),
            editedAt = json.cleanRecallString("edited_at"),
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
        const val SOCIAL_AI_USER_ID = "usr_elon_ai"
    }
}
