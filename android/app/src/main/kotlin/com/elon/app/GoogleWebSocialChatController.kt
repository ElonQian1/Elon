package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptFriendMessageMapper
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.googleweb.GoogleWebBackgroundSession
import com.elon.app.googleweb.GoogleWebPageAdapter

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
    private var pendingPrompt: String? = null

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
        if (!session.canSend()) {
            Toast.makeText(activity, "Google 网页 AI 尚未就绪，请打开官方页确认", Toast.LENGTH_LONG).show()
            openOfficialFallback()
            return true
        }
        pendingPrompt = prompt
        session.currentSnapshot()?.let(::renderSnapshot)
        if (!session.sendPrompt(prompt)) {
            pendingPrompt = null
            Toast.makeText(activity, "Google 网页 AI 发送入口尚未就绪", Toast.LENGTH_LONG).show()
            return true
        }
        binding.inputEdit.text?.clear()
        clearPendingSendState()
        collapseInputComposer()
        return true
    }

    override fun requestModelOptions() {
        Toast.makeText(activity, "模型由 Google AI 模式官方页面选择", Toast.LENGTH_SHORT).show()
    }

    override fun refreshComposerModel() = updateComposerModel()

    override fun stopGeneration() {
        session.stopGeneration()
    }

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

    override fun openProject(path: String): Boolean = session.openProject(path)

    override fun onHostResumed() = session.onHostResumed()

    override fun onHostPaused() = session.onHostPaused()

    override fun destroy() = session.destroy()

    private fun renderSnapshot(snapshot: ChatGptWebSnapshot) {
        val pending = pendingPrompt?.trim().orEmpty()
        if (pending.isNotEmpty() && snapshot.messages.lastOrNull { it.role == "user" }
                ?.content?.trim() == pending
        ) {
            pendingPrompt = null
        }
        val mapped = ChatGptFriendMessageMapper.map(
            snapshot = snapshot,
            provider = provider,
            pendingPrompt = pendingPrompt,
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
        if (event.action != "send_prompt" || event.ok) return
        pendingPrompt = null
        session.currentSnapshot()?.let(::renderSnapshot)
        Toast.makeText(activity, event.detail.ifBlank { "Google 网页 AI 操作失败" }, Toast.LENGTH_LONG).show()
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
}
