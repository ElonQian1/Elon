package com.elon.app

import android.util.Log
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptFriendMessageMapper
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.googleweb.GoogleWebBackgroundSession
import com.elon.app.googleweb.GoogleWebPageAdapter
import com.elon.app.googleweb.GoogleWebPendingSendState

internal class GoogleWebSocialChatController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val clearPendingSendState: () -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val onConversationIndexChanged: () -> Unit,
) : WebChatSocialController {
    override val providerId = WebChatProviderId.GOOGLE_WEB
    private val messages = mutableListOf<ChatMessage>()
    private val timestamps = linkedMapOf<String, Long>()
    private val adapter = ChatAdapter(messages, onMessageLongPress = showMessageActions)
    private val session = GoogleWebBackgroundSession(
        activity = activity,
        host = binding.chatListFrame,
        onSnapshot = ::renderSnapshot,
        onStateChanged = ::renderState,
        onCommandResult = ::handleCommandResult,
        onConversationIndexChanged = { onConversationIndexChanged() },
    )
    private var provider = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)
    private var active = false
    private val pendingSend = GoogleWebPendingSendState()
    private var pendingSendWatchdog: Runnable? = null

    override fun activate(identity: WebChatProviderIdentity) {
        provider = identity
        active = true
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        session.activate()
        session.currentSnapshot()?.let(::renderSnapshot)
        updateComposerModel()
    }

    override fun deactivate() {
        active = false
        session.deactivate()
    }

    override fun isActive(): Boolean = active

    override fun currentMessages(): List<ChatMessage> = messages.toList()

    override fun stateWireValue(): String = session.state().wireValue

    override fun currentModel(): String = session.currentSnapshot()?.currentModel.orEmpty()

    override fun adapterVersion(): Int = GoogleWebPageAdapter.ADAPTER_VERSION

    override fun authenticated(): Boolean = session.currentSnapshot()?.authenticated == true

    override fun composerReady(): Boolean = session.currentSnapshot()?.composerReady == true

    override fun attachmentSupported(): Boolean = false

    override fun attachmentSendPhase(): String = "idle"

    override fun pendingAttachmentCount(): Int = 0

    override fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>): Boolean {
        if (!active) return false
        if (pendingAttachments.isNotEmpty()) {
            Toast.makeText(activity, "Google 网页 AI 暂不支持从一龙界面上传附件", Toast.LENGTH_LONG).show()
            return true
        }
        val prompt = rawText.trim()
        if (prompt.isBlank()) return true
        if (pendingSend.prompt() != null) {
            val detail = if (pendingSend.requiresOfficialConfirmation()) {
                "上一条已发送，但回答尚未同步，请打开官方页确认"
            } else {
                "上一条消息仍在提交，请稍候"
            }
            Toast.makeText(activity, detail, Toast.LENGTH_LONG).show()
            return true
        }
        if (!session.canSend()) {
            Toast.makeText(activity, "Google 网页 AI 尚未就绪，请打开官方页确认", Toast.LENGTH_LONG).show()
            openOfficialFallback()
            return true
        }
        val sendGeneration = pendingSend.begin(prompt)
        session.currentSnapshot()?.let(::renderSnapshot)
        if (!session.sendPrompt(prompt)) {
            pendingSend.failSubmission()
            session.currentSnapshot()?.let(::renderSnapshot)
            Toast.makeText(activity, "Google 网页 AI 发送入口尚未就绪", Toast.LENGTH_LONG).show()
            return true
        }
        scheduleSubmissionConfirmationWatchdog(sendGeneration)
        binding.inputEdit.text?.clear()
        clearPendingSendState()
        collapseInputComposer()
        return true
    }

    override fun requestModelOptions() {
        openOfficialFallback()
    }

    override fun refreshComposerModel() = updateComposerModel()

    override fun stopGeneration() {
        session.stopGeneration()
    }

    override fun startNewConversation() {
        clearPendingSend()
        session.startNewConversation()
    }

    override fun currentConversationPath(): String? = session.currentConversationPath()

    override fun officialFallbackUrl(): String? = session.currentOfficialUrl()

    override fun conversationIndex() = session.conversationIndex()

    override fun requestConversationIndex(): Boolean = session.requestConversationIndex()

    override fun openConversation(path: String): Boolean {
        clearPendingSend()
        return session.openConversation(path)
    }

    override fun openProject(path: String): Boolean = session.openProject(path)

    override fun onHostResumed() = session.onHostResumed()

    override fun onHostPaused() = session.onHostPaused()

    override fun destroy() {
        clearPendingSend()
        session.destroy()
    }

    private fun renderSnapshot(snapshot: ChatGptWebSnapshot) {
        if (pendingSend.observeUserPrompt(snapshot.messages.lastOrNull { it.role == "user" }?.content)) {
            cancelPendingSendWatchdog()
        }
        val mapped = ChatGptFriendMessageMapper.map(
            snapshot = snapshot,
            provider = provider,
            pendingPrompt = pendingSend.prompt(),
            pendingAttachments = emptyList(),
            pendingSendStatus = "发送中…",
            attachmentsForMessage = { emptyList() },
            timestampFor = { id -> timestamps.getOrPut(id) { System.currentTimeMillis() } },
        )
        messages.clear()
        messages.addAll(mapped)
        if (!active) return
        adapter.notifyDataSetChanged()
        if (messages.isNotEmpty()) binding.chatList.jumpToLatestMessageBeforeNextDraw()
        updateComposerModel()
    }

    private fun renderState(state: GoogleWebBackgroundSession.State, detail: String?) {
        if (!active) return
        if (messages.isEmpty()) when (state) {
            GoogleWebBackgroundSession.State.LOADING -> renderStatus("正在连接 Google 搜索网页 AI…")
            GoogleWebBackgroundSession.State.ERROR -> renderStatus(
                detail?.takeIf(String::isNotBlank) ?: "Google 搜索网页 AI 暂时不可用。",
            )
            GoogleWebBackgroundSession.State.IDLE,
            GoogleWebBackgroundSession.State.READY -> Unit
        }
    }

    private fun handleCommandResult(event: ChatGptWebEvent.CommandResult) {
        if (event.action != "send_prompt") return
        Log.i(SEND_LOG_TAG, "action=send_prompt ok=${event.ok}")
        if (event.ok) {
            pendingSend.confirmSubmission()
            return
        }
        val failedPrompt = pendingSend.failSubmission()
        cancelPendingSendWatchdog()
        session.currentSnapshot()?.let(::renderSnapshot)
        restorePrompt(failedPrompt)
        Toast.makeText(activity, event.detail.ifBlank { "Google 网页 AI 操作失败" }, Toast.LENGTH_LONG).show()
    }

    private fun scheduleSubmissionConfirmationWatchdog(generation: Long) {
        cancelPendingSendWatchdog()
        val watchdog = Runnable {
            pendingSendWatchdog = null
            val result = pendingSend.onConfirmationTimeout(generation)
            when (result.action) {
                GoogleWebPendingSendState.TimeoutAction.IGNORE -> Unit
                GoogleWebPendingSendState.TimeoutAction.KEEP_WAITING -> {
                    session.requestConversationIndex()
                    scheduleSubmissionConfirmationWatchdog(generation)
                }
                GoogleWebPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION -> {
                    session.requestConversationIndex()
                    if (active) Toast.makeText(
                        activity,
                        "官网已确认发送，但回答同步超时，请打开官方页确认",
                        Toast.LENGTH_LONG,
                    ).show()
                }
                GoogleWebPendingSendState.TimeoutAction.RESTORE -> {
                    session.currentSnapshot()?.let(::renderSnapshot)
                    restorePrompt(result.prompt)
                    if (active) Toast.makeText(
                        activity,
                        "官网未确认发送，消息已保留，请重试",
                        Toast.LENGTH_LONG,
                    ).show()
                }
            }
        }
        pendingSendWatchdog = watchdog
        binding.root.postDelayed(watchdog, SEND_CONFIRMATION_TIMEOUT_MS)
    }

    private fun clearPendingSend() {
        pendingSend.clear()
        cancelPendingSendWatchdog()
    }

    private fun cancelPendingSendWatchdog() {
        pendingSendWatchdog?.let(binding.root::removeCallbacks)
        pendingSendWatchdog = null
    }

    private fun restorePrompt(prompt: String?) {
        if (!active || prompt.isNullOrBlank() || !binding.inputEdit.text.isNullOrBlank()) return
        binding.inputEdit.setText(prompt)
        binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
    }

    private fun renderStatus(content: String) {
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

    private fun updateComposerModel() {
        if (!active) return
        val label = currentModel().ifBlank { "Google AI 模式" }
        binding.modelButton.text = label
        binding.modelButton.contentDescription = "聊天模式；提供方：${provider.displayName}；模型：$label"
        (binding.modelButton.parent as? View)?.contentDescription = binding.modelButton.contentDescription
    }

    private companion object {
        const val SEND_CONFIRMATION_TIMEOUT_MS = 12_000L
        const val SEND_LOG_TAG = "ElonGoogleWebSend"
    }
}
