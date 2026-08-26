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

internal class GoogleWebSocialChatController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val clearPendingSendState: () -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val onConversationIndexChanged: () -> Unit,
    private val onComposerStateChanged: () -> Unit,
) : WebChatSocialController {
    override val providerId = WebChatProviderId.GOOGLE_WEB
    private val transcript = WebChatProductionTranscript(
        list = binding.chatList,
        setChatAdapter = setChatAdapter,
        onMessageLongPress = showMessageActions,
        onMessageAction = ::handleWebChatMessageAction,
        onContentOpen = { _, _ -> openOfficialFallback() },
    )
    private val session = GoogleWebBackgroundSession(
        activity = activity,
        host = binding.chatListFrame,
        onSnapshot = ::renderSnapshot,
        onStateChanged = ::renderState,
        onCommandResult = ::handleCommandResult,
        onConversationIndexChanged = { onConversationIndexChanged() },
    )
    private val sendTransport = OfficialPageWebChatSendTransport(
        ready = session::canSend,
        sendPrompt = session::sendPrompt,
        requestReconciliation = { session.requestConversationIndex() },
    )
    private val sendCoordinator = WebChatSendCoordinator(
        transport = sendTransport,
        postDelayed = { task, delayMs -> binding.root.postDelayed(task, delayMs) },
        removeCallbacks = { task -> binding.root.removeCallbacks(task) },
        onTerminalTimeout = ::handlePendingSendTimeout,
    )
    private var provider = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)
    private val productionMessageActions by lazy {
        WebChatProductionMessageActionCoordinator(
            activity = activity,
            consumerPort = { null },
            openOfficialFallback = openOfficialFallback,
        )
    }
    private var active = false
    private var latestCommandStatus: WebChatCommandStatus? = null
    private var latestStateDetail: String? = null

    override fun activate(identity: WebChatProviderIdentity) {
        provider = identity
        active = true
        transcript.activate()
        session.activate()
        session.currentSnapshot()?.let(::renderSnapshot)
        updateComposerModel()
    }

    override fun deactivate() {
        active = false
        sendCoordinator.pauseWatchdog()
        session.deactivate()
    }

    override fun isActive(): Boolean = active

    override fun currentMessages(): List<ChatMessage> = transcript.currentMessages()

    override fun stateWireValue(): String = session.state().wireValue

    override fun stateDetail(): String? = latestStateDetail

    override fun currentModel(): String = session.currentSnapshot()?.currentModel.orEmpty()

    override fun adapterVersion(): Int = GoogleWebPageAdapter.ADAPTER_VERSION

    override fun authenticated(): Boolean = session.currentSnapshot()?.authenticated == true

    override fun composerReady(): Boolean = session.currentSnapshot()?.composerReady == true

    override fun warmSessionAvailable(): Boolean = session.warmSessionAvailable()

    override fun prewarm(): Boolean {
        if (active || !session.warmSessionAvailable()) return false
        session.activate()
        return true
    }

    override fun streaming(): Boolean = session.currentSnapshot()?.streaming == true

    override fun attachmentSupported(): Boolean = false

    override fun attachmentSendPhase(): String = "idle"

    override fun pendingAttachmentCount(): Int = 0

    override fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>): Boolean {
        if (!active) return false
        if (pendingAttachments.isNotEmpty()) {
            Toast.makeText(
                activity,
                "当前一龙 Google 网页 AI 适配尚未接入附件上传，可在官方页使用",
                Toast.LENGTH_LONG,
            ).show()
            return true
        }
        val prompt = rawText.trim()
        if (prompt.isBlank()) return true
        if (sendCoordinator.prompt() != null) {
            val detail = if (sendCoordinator.requiresOfficialConfirmation()) {
                "上一条已发送，但回答尚未同步，请打开官方页确认"
            } else {
                "上一条消息仍在提交，请稍候"
            }
            Toast.makeText(activity, detail, Toast.LENGTH_LONG).show()
            return true
        }
        if (!sendCoordinator.isReady()) {
            when (WebChatSendFallbackPolicy.decide(loginRequired = false)) {
                WebChatSendFallbackPolicy.Action.RETRY_IN_PLACE,
                WebChatSendFallbackPolicy.Action.RETRY_GUEST_ACCESS -> {
                    session.onHostResumed()
                    Toast.makeText(activity, "Google 网页 AI 正在连接，请稍后重试", Toast.LENGTH_LONG).show()
                }
            }
            return true
        }
        val result = sendCoordinator.dispatch(prompt, session.currentSnapshot()) {
            transcript.requestFollowLatest()
            session.currentSnapshot()?.let(::renderSnapshot)
        }
        when (result.outcome) {
            WebChatSendCoordinator.DispatchOutcome.DISPATCHED -> {
                binding.inputEdit.text?.clear()
                clearPendingSendState()
                collapseInputComposer()
            }
            WebChatSendCoordinator.DispatchOutcome.REJECTED -> {
                session.currentSnapshot()?.let(::renderSnapshot)
                restorePrompt(result.prompt)
                Toast.makeText(
                    activity,
                    "Google 网页 AI 发送入口尚未就绪",
                    Toast.LENGTH_LONG,
                ).show()
            }
            WebChatSendCoordinator.DispatchOutcome.BUSY,
            WebChatSendCoordinator.DispatchOutcome.NOT_READY -> Unit
        }
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
        transcript.requestFollowLatest()
        session.startNewConversation()
    }

    override fun currentConversationPath(): String? = session.currentConversationPath()

    override fun officialFallbackUrl(): String? = session.currentOfficialUrl()

    override fun conversationIndex() = session.conversationIndex()

    override fun requestConversationIndex(projectId: String?): Boolean = session.requestConversationIndex()

    override fun openConversation(path: String): Boolean {
        clearPendingSend()
        transcript.requestFollowLatest()
        return session.openConversation(path).also { opened ->
            if (!opened) transcript.cancelFollowLatest()
        }
    }

    override fun openProject(path: String): Boolean = session.openProject(path)

    override fun supportsLocalProjects(): Boolean = true

    override fun createLocalProject(title: String): Boolean = session.createLocalProject(title)

    override fun assignConversationToLocalProject(path: String, projectId: String?): Boolean =
        session.assignConversationToLocalProject(path, projectId)

    override fun lastCommandStatus(): WebChatCommandStatus? = latestCommandStatus

    override fun retryConnection(): Boolean = session.retryConnection()

    override fun onHostResumed() = session.onHostResumed()

    override fun onHostPaused() = session.onHostPaused()

    override fun destroy() {
        clearPendingSend()
        session.destroy()
    }

    private fun renderSnapshot(snapshot: ChatGptWebSnapshot) {
        when (sendCoordinator.observeSnapshot(snapshot)) {
            WebChatSendCoordinator.Observation.SUBMISSION_CONFIRMED ->
                session.onSubmissionObserved()
            WebChatSendCoordinator.Observation.NONE,
            WebChatSendCoordinator.Observation.TURN_COMPLETED -> Unit
        }
        val pendingStatus = sendCoordinator.status()
        val pendingPrompt = sendCoordinator.prompt()
        val mapped = ChatGptFriendMessageMapper.map(
            snapshot = snapshot,
            provider = provider,
            pendingPrompt = pendingPrompt,
            pendingAttachments = emptyList(),
            pendingSendStatus = pendingStatus ?: "发送中…",
            attachmentsForMessage = { emptyList() },
            timestampFor = transcript::timestampFor,
        )
        if (pendingStatus != null) {
            mapped.lastOrNull { message ->
                message.role == "user" && message.content.trim() == pendingPrompt?.trim()
            }?.sendStatus = pendingStatus
        }
        val presented = WebChatProductionHistoryNotice.prepend(
            snapshot = snapshot,
            provider = provider,
            messages = mapped,
            timestampFor = transcript::timestampFor,
        )
        transcript.submit(presented, active)
        if (!active) return
        updateComposerModel()
        onComposerStateChanged()
    }

    private fun renderState(state: GoogleWebBackgroundSession.State, detail: String?) {
        latestStateDetail = detail?.takeIf(String::isNotBlank)
            ?.takeIf { state == GoogleWebBackgroundSession.State.ERROR }
        if (!active) return
        if (!transcript.hasMessages()) when (state) {
            GoogleWebBackgroundSession.State.LOADING -> renderStatus("正在连接 Google 搜索网页 AI…")
            GoogleWebBackgroundSession.State.ERROR -> renderStatus(
                detail?.takeIf(String::isNotBlank) ?: "Google 搜索网页 AI 暂时不可用。",
            )
            GoogleWebBackgroundSession.State.IDLE,
            GoogleWebBackgroundSession.State.READY -> Unit
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
        if (event.action != "send_prompt") return
        Log.i(SEND_LOG_TAG, "action=send_prompt ok=${event.ok}")
        val failedPrompt = sendCoordinator.acceptCommandResult(event.ok)
        if (event.ok) {
            session.currentSnapshot()?.let(::renderSnapshot)
            return
        }
        session.currentSnapshot()?.let(::renderSnapshot)
        restorePrompt(failedPrompt)
        Toast.makeText(activity, event.detail.ifBlank { "Google 网页 AI 操作失败" }, Toast.LENGTH_LONG).show()
    }

    private fun handlePendingSendTimeout(result: WebChatPendingSendState.TimeoutResult) {
        when (result.action) {
            WebChatPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION -> {
                session.currentSnapshot()?.let(::renderSnapshot)
                if (active) Toast.makeText(
                    activity,
                    "官网已确认发送，但回答同步超时，请打开官方页确认",
                    Toast.LENGTH_LONG,
                ).show()
            }
            WebChatPendingSendState.TimeoutAction.RESTORE -> {
                session.currentSnapshot()?.let(::renderSnapshot)
                restorePrompt(result.prompt)
                if (active) Toast.makeText(
                    activity,
                    "官网未确认发送，消息已保留，请重试",
                    Toast.LENGTH_LONG,
                ).show()
            }
            WebChatPendingSendState.TimeoutAction.IGNORE,
            WebChatPendingSendState.TimeoutAction.KEEP_WAITING -> Unit
        }
    }

    private fun clearPendingSend() = sendCoordinator.clear()

    private fun restorePrompt(prompt: String?) {
        if (!active || prompt.isNullOrBlank() || !binding.inputEdit.text.isNullOrBlank()) return
        binding.inputEdit.setText(prompt)
        binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
    }

    private fun renderStatus(content: String) {
        transcript.showStatus(provider, content)
    }

    private fun handleWebChatMessageAction(message: ChatMessage, action: WebChatMessageAction) {
        productionMessageActions.handle(message, action)
    }

    private fun updateComposerModel() {
        if (!active) return
        val label = currentModel().ifBlank { "Google AI 模式" }
        WebChatComposerProviderPresentation.apply(binding.modelButton, provider, label)
    }

    private companion object {
        const val SEND_LOG_TAG = "ElonGoogleWebSend"
    }
}
