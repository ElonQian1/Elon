package com.elon.app

import android.graphics.Rect
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptBackgroundSession
import com.elon.app.chatgptweb.ChatGptFriendMessageMapper
import com.elon.app.chatgptweb.ChatGptMessageClipboard
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation
import com.elon.app.chatgptweb.ChatGptWebAudioPermissionController
import com.elon.app.chatgptweb.ChatGptWebAttachmentSendUpdate
import com.elon.app.chatgptweb.ChatGptWebComposerOption
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebModeController
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.databinding.ActivityMainBinding

internal class ChatGptSocialChatController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val clearPendingSendState: () -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val onConversationIndexChanged: () -> Unit,
    private val onComposerStateChanged: () -> Unit,
    audioPermissionController: ChatGptWebAudioPermissionController,
) : WebChatSocialController {
    override val providerId = WebChatProviderId.CHATGPT_WEB
    private val messages = mutableListOf<ChatMessage>()
    private val timestamps = linkedMapOf<String, Long>()
    private val adapter = ChatAdapter(messages, onMessageLongPress = showMessageActions).apply {
        onWebChatMessageAction = ::handleWebChatMessageAction
        onWebChatContentOpen = { _, _ -> openOfficialFallback() }
    }
    private val messageClipboard = ChatGptMessageClipboard(activity)
    private val session = ChatGptBackgroundSession(
        activity = activity,
        host = binding.chatListFrame,
        onSnapshot = ::renderSnapshot,
        onStateChanged = ::renderState,
        onComposerOptions = ::showModelOptions,
        onCommandResult = ::handleCommandResult,
        onAttachmentSendChanged = ::handleAttachmentSendUpdate,
        onConversationIndexChanged = { onConversationIndexChanged() },
        audioPermissionController = audioPermissionController,
    )
    private var provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
    private var active = false
    private var pendingPrompt: String? = null
    private var pendingAttachments = emptyList<PendingAttachment>()
    private val sentAttachments = linkedMapOf<String, List<ChatAttachment>>()
    private var waitingForAttachmentCompletion = false
    private var latestCommandStatus: WebChatCommandStatus? = null
    private val socialMcpPort: WebChatSocialMcpPort by lazy {
        session.createMcpPort(
            inputText = { binding.inputEdit.text?.toString().orEmpty() },
            setInputText = ::setInputTextFromMcp,
            copyMessage = messageClipboard::copy,
            selectMode = { mode ->
                if (mode != ChatGptWebModeController.Mode.NATIVE) openOfficialFallback()
            },
            revealMessage = ::revealMessageFromMcp,
        )
    }
    private val productionMessageActions by lazy {
        WebChatProductionMessageActionCoordinator(
            activity = activity,
            mcpPort = { socialMcpPort },
            openOfficialFallback = openOfficialFallback,
        )
    }

    override fun activate(identity: WebChatProviderIdentity) {
        provider = identity
        active = true
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        if (messages.isNotEmpty()) binding.chatList.jumpToLatestMessageBeforeNextDraw()
        session.activate()
        session.currentSnapshot()?.let(::renderSnapshot)
        updateComposerModel(session.currentSnapshot()?.currentModel.orEmpty())
    }

    override fun deactivate() {
        active = false
    }

    override fun isActive(): Boolean = active

    override fun currentMessages(): List<ChatMessage> = messages.toList()

    override fun stateWireValue(): String = session.state().wireValue

    override fun currentModel(): String = session.currentSnapshot()?.currentModel.orEmpty()

    override fun adapterVersion(): Int = com.elon.app.chatgptweb.ChatGptWebPageAdapter.ADAPTER_VERSION

    override fun authenticated(): Boolean = session.currentSnapshot()?.authenticated == true

    override fun composerReady(): Boolean = session.currentSnapshot()?.composerReady == true

    override fun streaming(): Boolean = session.currentSnapshot()?.streaming == true

    override fun attachmentSupported(): Boolean = session.currentSnapshot()?.capabilities
        ?.supports(com.elon.app.chatgptweb.ChatGptWebCapabilityId.ATTACHMENTS) == true

    override fun refreshComposerModel() = updateComposerModel(currentModel())

    override fun attachmentSendPhase(): String = session.attachmentSendPhase()

    override fun pendingAttachmentCount(): Int = maxOf(session.pendingAttachmentCount(), pendingAttachments.size)

    override fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>): Boolean {
        if (!active) return false
        if (pendingAttachments.isNotEmpty()) {
            if (waitingForAttachmentCompletion) {
                Toast.makeText(activity, R.string.web_chat_attachment_uploading, Toast.LENGTH_SHORT).show()
                return true
            }
            if (!session.canSend()) {
                Toast.makeText(activity, R.string.web_chat_not_ready, Toast.LENGTH_LONG).show()
                return true
            }
            val prompt = rawText.trim()
            this.pendingPrompt = prompt
            this.pendingAttachments = pendingAttachments.toList()
            waitingForAttachmentCompletion = true
            renderSnapshot(session.currentSnapshot() ?: return true)
            if (!session.sendAttachments(prompt, pendingAttachments)) {
                this.pendingPrompt = null
                this.pendingAttachments = emptyList()
                waitingForAttachmentCompletion = false
                session.currentSnapshot()?.let(::renderSnapshot)
                Toast.makeText(activity, R.string.web_chat_attachment_upload_failed, Toast.LENGTH_LONG).show()
            } else {
                collapseInputComposer()
            }
            return true
        }
        val prompt = rawText.trim()
        if (prompt.isBlank()) return true
        if (!session.canSend()) {
            when (WebChatSendFallbackPolicy.decide(
                loginRequired = session.state() == ChatGptBackgroundSession.State.LOGIN_REQUIRED,
            )) {
                WebChatSendFallbackPolicy.Action.OPEN_OFFICIAL_AUTHENTICATION -> {
                    Toast.makeText(activity, R.string.web_chat_login_required, Toast.LENGTH_LONG).show()
                    openOfficialFallback()
                }
                WebChatSendFallbackPolicy.Action.RETRY_IN_PLACE -> {
                    session.onHostResumed()
                    Toast.makeText(activity, R.string.web_chat_not_ready, Toast.LENGTH_LONG).show()
                }
            }
            return true
        }
        pendingPrompt = prompt
        renderSnapshot(session.currentSnapshot() ?: return true)
        if (!session.sendPrompt(prompt)) {
            pendingPrompt = null
            Toast.makeText(activity, R.string.web_chat_not_ready, Toast.LENGTH_LONG).show()
            return true
        }
        binding.inputEdit.text?.clear()
        clearPendingSendState()
        collapseInputComposer()
        return true
    }

    override fun requestModelOptions() {
        if (!session.requestModelOptions()) {
            Toast.makeText(activity, R.string.web_chat_not_ready, Toast.LENGTH_SHORT).show()
        }
    }

    override fun stopGeneration() = session.stopGeneration()

    override fun startNewConversation() {
        pendingPrompt = null
        session.startNewConversation()
    }

    override fun currentConversationPath(): String? = session.currentConversationPath()

    override fun conversationIndex() = session.conversationIndex()

    override fun requestConversationIndex(): Boolean = session.requestConversationIndex()

    override fun openConversation(path: String): Boolean {
        pendingPrompt = null
        return session.openConversation(path)
    }

    override fun openProject(path: String): Boolean {
        pendingPrompt = null
        return session.openProject(path)
    }

    override fun mcpPort(): WebChatSocialMcpPort = socialMcpPort

    override fun lastCommandStatus(): WebChatCommandStatus? = latestCommandStatus

    override fun discardAcceptanceAttachmentSend(): Boolean {
        if (waitingForAttachmentCompletion) return false
        val hadFixture = pendingAttachments.any {
            ChatGptWebAcceptanceAttachmentFixture.matches(activity.cacheDir, it)
        }
        if (!hadFixture) return false
        pendingPrompt = null
        pendingAttachments = emptyList()
        session.currentSnapshot()?.let(::renderSnapshot)
        return true
    }

    override fun onHostResumed() = session.onHostResumed()

    override fun onHostPaused() = session.onHostPaused()

    override fun destroy() = session.destroy()

    private fun setInputTextFromMcp(value: String) {
        binding.inputEdit.setText(value)
        binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
    }

    private fun revealMessageFromMcp(messageId: String, partIndex: Int?, target: String): Boolean {
        val nativeId = "${provider.id.wireValue}:$messageId"
        val index = messages.indexOfFirst { it.id == nativeId }
        if (index < 0) return false
        val message = messages[index]
        if (partIndex != null && partIndex !in message.webChatMessage?.contentParts.orEmpty().indices) {
            return false
        }
        val requiredAction = when (target) {
            "copy" -> WebChatMessageAction.COPY
            "regenerate" -> WebChatMessageAction.REGENERATE
            "actions" -> WebChatMessageAction.MORE
            else -> null
        }
        if (requiredAction != null && requiredAction !in message.webChatMessage?.actions.orEmpty()) {
            return false
        }
        binding.chatList.scrollToPosition(index)
        revealMessageTarget(index, messageId, partIndex, target, attempt = 0)
        return true
    }

    private fun revealMessageTarget(
        index: Int,
        messageId: String,
        partIndex: Int?,
        target: String,
        attempt: Int,
    ) {
        binding.chatList.postDelayed({
            val itemView = binding.chatList.findViewHolderForAdapterPosition(index)?.itemView
            val targetView = itemView?.let { row ->
                partIndex?.let {
                    row.findViewById<android.widget.LinearLayout>(R.id.webChatMessagePartList)
                        ?.getChildAt(it)
                } ?: when (target) {
                    "copy" -> row.findViewById<View>(R.id.webChatMessageCopy)
                    "regenerate" -> row.findViewById<View>(R.id.webChatMessageRegenerate)
                    "actions" -> row.findViewById<View>(R.id.webChatMessageMore)
                    else -> row
                }
            }
            if (itemView == null || targetView == null || targetView.visibility != View.VISIBLE) {
                retryRevealMessageTarget(index, messageId, partIndex, target, attempt)
                return@postDelayed
            }

            itemView.contentDescription = "web-chat-message:${provider.id.wireValue}:" +
                com.elon.app.chatgptweb.ChatGptNativeControlPresentation.stableContextId(messageId)
            val targetRect = Rect(0, 0, targetView.width.coerceAtLeast(1), targetView.height.coerceAtLeast(1))
            binding.chatList.offsetDescendantRectToMyCoords(targetView, targetRect)
            val itemRect = Rect(0, 0, itemView.width.coerceAtLeast(1), itemView.height.coerceAtLeast(1))
            binding.chatList.offsetDescendantRectToMyCoords(itemView, itemRect)
            targetRect.offset(-itemRect.left, -itemRect.top)
            binding.chatList.requestChildRectangleOnScreen(itemView, targetRect, true)
            targetView.requestFocus()
            val visibleRect = Rect()
            val fullyVisible = targetView.getGlobalVisibleRect(visibleRect) &&
                visibleRect.width() >= targetView.width &&
                visibleRect.height() >= targetView.height
            if (!fullyVisible) {
                retryRevealMessageTarget(index, messageId, partIndex, target, attempt)
            }
        }, if (attempt == 0) 0L else REVEAL_RETRY_DELAY_MS)
    }

    private fun retryRevealMessageTarget(
        index: Int,
        messageId: String,
        partIndex: Int?,
        target: String,
        attempt: Int,
    ) {
        if (attempt >= MAX_REVEAL_ATTEMPTS) return
        revealMessageTarget(index, messageId, partIndex, target, attempt + 1)
    }

    private fun renderSnapshot(snapshot: ChatGptWebSnapshot) {
        val cleanPending = pendingPrompt?.trim().orEmpty()
        if (
            cleanPending.isNotEmpty() &&
            snapshot.messages.lastOrNull { it.role == "user" }?.content?.trim() == cleanPending
        ) {
            pendingPrompt = null
        }
        val mapped = ChatGptFriendMessageMapper.map(
            snapshot = snapshot,
            provider = provider,
            pendingPrompt = pendingPrompt,
            pendingAttachments = chatAttachmentsFromPending(pendingAttachments),
            pendingSendStatus = when (session.attachmentSendPhase()) {
                "uploading" -> "上传中…"
                "failed" -> "发送失败，请重新点击发送"
                else -> "发送中…"
            },
            attachmentsForMessage = { id -> sentAttachments[id].orEmpty() },
            messageActionContextIds = WebChatProductionMessageActionJson.messageContextIds(
                socialMcpPort.uiState(),
            ),
            timestampFor = { id -> timestamps.getOrPut(id) { System.currentTimeMillis() } },
        )
        messages.clear()
        messages.addAll(mapped)
        if (!active) return
        adapter.notifyDataSetChanged()
        if (messages.isNotEmpty()) binding.chatList.jumpToLatestMessageBeforeNextDraw()
        updateComposerModel(snapshot.currentModel)
        onComposerStateChanged()
    }

    private fun renderState(state: ChatGptBackgroundSession.State, detail: String?) {
        if (!active) return
        if (messages.isEmpty()) when (state) {
            ChatGptBackgroundSession.State.LOADING -> renderStatusMessage("正在连接 ChatGPT 网页 AI…")
            ChatGptBackgroundSession.State.LOGIN_REQUIRED -> renderStatusMessage(
                "当前页面需要登录。可打开“官网功能”登录，也可在官网支持时直接匿名聊天。",
            )
            ChatGptBackgroundSession.State.ERROR -> renderStatusMessage(
                detail?.takeIf(String::isNotBlank) ?: "ChatGPT 网页 AI 暂时不可用。",
            )
            ChatGptBackgroundSession.State.IDLE, ChatGptBackgroundSession.State.READY -> Unit
        }
        if (state == ChatGptBackgroundSession.State.ERROR && !detail.isNullOrBlank()) {
            Toast.makeText(activity, detail, Toast.LENGTH_LONG).show()
        }
        onComposerStateChanged()
    }

    private fun handleCommandResult(event: ChatGptWebEvent.CommandResult) {
        latestCommandStatus = WebChatCommandStatus(
            action = event.action,
            ok = event.ok,
            detail = event.detail,
            observedAtMs = System.currentTimeMillis(),
        )
        if (pendingAttachments.isNotEmpty()) return
        if (event.action != "send_prompt" || event.ok) return
        pendingPrompt = null
        session.currentSnapshot()?.let(::renderSnapshot)
        Toast.makeText(
            activity,
            event.detail.ifBlank { activity.getString(R.string.chatgpt_native_command_failed) },
            Toast.LENGTH_LONG,
        ).show()
    }

    private fun handleAttachmentSendUpdate(update: ChatGptWebAttachmentSendUpdate) {
        when (update.phase) {
            "completed" -> {
                update.userMessageId?.let { id ->
                    sentAttachments[id] = chatAttachmentsFromPending(pendingAttachments)
                }
                pendingPrompt = null
                pendingAttachments = emptyList()
                waitingForAttachmentCompletion = false
                binding.inputEdit.text?.clear()
                clearPendingSendState()
                session.currentSnapshot()?.let(::renderSnapshot)
            }
            "failed" -> {
                waitingForAttachmentCompletion = false
                Toast.makeText(
                    activity,
                    update.detail ?: activity.getString(R.string.web_chat_attachment_upload_failed),
                    Toast.LENGTH_LONG,
                ).show()
                session.currentSnapshot()?.let(::renderSnapshot)
            }
        }
    }

    private fun renderStatusMessage(content: String) {
        val id = "${provider.id.wireValue}:status"
        messages.clear()
        messages += ChatMessage(
            role = "friend",
            content = content,
            senderLabel = provider.displayName,
            senderAvatarResId = provider.avatarResId,
            id = id,
            createdAtMs = timestamps.getOrPut(id) { System.currentTimeMillis() },
        )
        adapter.notifyDataSetChanged()
    }

    private fun handleWebChatMessageAction(message: ChatMessage, action: WebChatMessageAction) {
        productionMessageActions.handle(message, action)
    }

    private fun showModelOptions(options: List<ChatGptWebComposerOption>) {
        if (!active) return
        val selectable = options.filter { it.id.isNotBlank() }
        if (selectable.isEmpty()) {
            Toast.makeText(activity, R.string.web_chat_model_options_empty, Toast.LENGTH_SHORT).show()
            return
        }
        val byId = selectable.associateBy(ChatGptWebComposerOption::id)
        WebChatActionSheet.show(
            activity = activity,
            title = activity.getString(R.string.web_chat_model_picker_title),
            items = selectable.map { option ->
                WebChatActionSheetItem(
                    id = option.id,
                    title = option.label,
                    subtitle = if (option.selected) "当前模型" else null,
                    selected = option.selected,
                    contentDescription = "web-chat-model-option:" +
                        ChatGptNativeControlPresentation.stableContextId(option.id),
                )
            },
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = activity.getString(R.string.web_chat_open_official),
                    contentDescription = "web-chat-model-official",
                    action = openOfficialFallback,
                ),
            ),
        ) { item -> byId[item.id]?.let { session.selectModel(it.id) } }
    }

    private fun updateComposerModel(model: String) {
        if (!active) return
        val label = model.ifBlank { provider.displayName }
        binding.modelButton.text = label
        binding.modelButton.contentDescription = "聊天模式；提供方：${provider.displayName}；模型：$label"
        (binding.modelButton.parent as? View)?.contentDescription = binding.modelButton.contentDescription
    }

    private companion object {
        const val MAX_REVEAL_ATTEMPTS = 8
        const val REVEAL_RETRY_DELAY_MS = 80L
    }
}
