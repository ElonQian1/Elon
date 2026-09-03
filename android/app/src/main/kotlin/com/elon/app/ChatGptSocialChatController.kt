package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptBackgroundSession
import com.elon.app.chatgptweb.ChatGptFriendMessageMapper
import com.elon.app.chatgptweb.ChatGptMessageClipboard
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation
import com.elon.app.chatgptweb.ChatGptWebConnectionMessagePolicy
import com.elon.app.chatgptweb.ChatGptWebAudioPermissionController
import com.elon.app.chatgptweb.ChatGptWebAttachmentSendUpdate
import com.elon.app.chatgptweb.ChatGptWebComposerOption
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebNativeVoiceTranscriptEvent
import com.elon.app.chatgptweb.ChatGptWebPresentationMode
import com.elon.app.chatgptweb.ChatGptWebPrivateTextReceiptPolicy
import com.elon.app.chatgptweb.ChatGptWebPrivateDictationTransport
import com.elon.app.chatgptweb.ChatGptWebSendOrigin
import com.elon.app.chatgptweb.ChatGptWebSendReceipt
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.chatgptweb.ChatGptWebSkinPresentationController
import com.elon.app.databinding.ActivityMainBinding

internal class ChatGptSocialChatController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val clearPendingSendState: () -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val openProviderPicker: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val onCreateImageRequested: () -> Unit,
    private val onConversationIndexChanged: () -> Unit,
    private val onComposerStateChanged: () -> Unit,
    private val onConsumerStateObserved: (WebChatConsumerState) -> Unit,
    private val onDictationCommandResult: (String, Boolean) -> Unit,
    private val interactionCache: WebChatProductionInteractionCache,
    audioPermissionController: ChatGptWebAudioPermissionController,
) : WebChatSocialController {
    override val providerId = WebChatProviderId.CHATGPT_WEB
    private val transcript = WebChatProductionTranscript(
        list = binding.chatList,
        setChatAdapter = setChatAdapter,
        onMessageLongPress = showMessageActions,
        onMessageAction = ::handleWebChatMessageAction,
        onContentOpen = { _, part -> imageContent.open(part) },
    )
    private val messageReveal = ChatGptSocialMessageRevealCoordinator(
        binding.chatList,
        { provider.id },
        transcript,
    )
    private val messageClipboard = ChatGptMessageClipboard(activity)
    private val session = ChatGptBackgroundSession(
        activity = activity,
        host = binding.chatListFrame,
        onSnapshot = ::renderSnapshot,
        onStateChanged = ::renderState,
        onComposerOptions = ::showModelOptions,
        onCommandResult = ::handleCommandResult,
        onSendTerminalTimeout = ::handlePendingSendTimeout,
        onAttachmentSendChanged = ::handleAttachmentSendUpdate,
        onConversationIndexChanged = { onConversationIndexChanged() },
        audioPermissionController = audioPermissionController,
        onRealtimeVoiceTranscript = ::handleRealtimeVoiceTranscript,
    )
    private val imageContent by lazy(LazyThreadSafetyMode.NONE) { ChatGptSocialImageContentController(activity, session, openOfficialFallback) }
    private val skinPresentation = ChatGptWebSkinPresentationController(binding, session)
    private var provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
    private var active = false
    private var pendingAttachmentPrompt: String? = null
    private var pendingAttachments = emptyList<PendingAttachment>()
    private var lastMessageSnapshot: ChatGptWebSnapshot? = null
    private val sentAttachments = linkedMapOf<String, List<ChatAttachment>>()
    private var waitingForAttachmentCompletion = false
    private var latestAttachmentCompletedCount = 0
    private var latestCommandStatus: WebChatCommandStatus? = null
    private var latestSendCommandStatus: WebChatCommandStatus? = null
    private var latestStateDetail: String? = null
    private var modelPopup: WebChatModelControlPopupHandle? = null
    private var modelOptionById = emptyMap<String, WebChatConsumerOption>()
    private var modelLiveOptionIds = emptySet<String>()
    private var modelRangeSelectionById = emptyMap<String, WebChatModelRangeSelection>()
    private var modelPickerActive = false
    private var pendingPresetModelLabel: String? = null
    private var pendingOfficialDictationDraft = false
    private val realtimeVoiceTranscript = WebChatRealtimeVoiceTranscriptContinuity()
    private val privateDictation: WebChatPrivateDictationPort =
        ChatGptWebPrivateDictationTransport(
            enabled = BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED,
            readyCheck = {
                session.state() == ChatGptBackgroundSession.State.READY &&
                    session.currentSnapshot()?.composerReady == true
            },
            currentOfficialDraft = { session.currentSnapshot()?.draft },
            readDraft = { binding.inputEdit.text?.toString().orEmpty() },
            writeDraft = ::setInputTextFromMcp,
            dispatchStart = session::startPrivateDictation,
            dispatchSubmit = session::submitPrivateDictation,
            dispatchCancel = session::cancelPrivateDictation,
            onFailure = { message -> Toast.makeText(activity, message, Toast.LENGTH_SHORT).show() },
        )
    private val socialMcpPort: WebChatSocialMcpPort by lazy {
        WebChatPrivateDictationMcpPort(
            delegate = session.createMcpPort(
                inputText = { binding.inputEdit.text?.toString().orEmpty() },
                setInputText = ::setInputTextFromMcp,
                copyMessage = messageClipboard::copy,
                selectMode = { mode ->
                    when (mode) {
                        ChatGptWebPresentationMode.NATIVE -> skinPresentation.exit()
                        ChatGptWebPresentationMode.SKIN -> skinPresentation.enter()
                        ChatGptWebPresentationMode.QUICK,
                        ChatGptWebPresentationMode.WEB -> openOfficialFallback()
                    }
                },
                revealMessage = messageReveal::reveal,
            ),
            dictation = privateDictation,
            readyCheck = privateDictation::ready,
            enabled = BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED,
        )
    }
    private val socialConsumerPort: WebChatConsumerPort by lazy {
        session.createConsumerPort(socialMcpPort)
    }
    private val productionMessageActions by lazy {
        WebChatProductionMessageActionCoordinator(
            activity = activity,
            consumerPort = { socialConsumerPort },
            openOfficialFallback = openOfficialFallback,
        )
    }

    override fun activate(identity: WebChatProviderIdentity) {
        provider = identity
        active = true
        transcript.activate()
        session.activate()
        session.currentSnapshot()?.let(::renderSnapshot)
        updateComposerModel(session.currentSnapshot()?.currentModel.orEmpty())
    }

    override fun deactivate() {
        active = false
        session.pauseSendWatchdog()
        if (!session.realtimeVoiceActive()) realtimeVoiceTranscript.reset()
        skinPresentation.exit()
        modelPopup?.dismiss()
        modelPopup = null
        modelOptionById = emptyMap()
        modelLiveOptionIds = emptySet()
        modelRangeSelectionById = emptyMap()
        modelPickerActive = false
        pendingPresetModelLabel = null
        session.dismissComposerOptions()
        session.deactivate()
    }

    override fun isActive(): Boolean = active

    override fun currentMessages(): List<ChatMessage> = transcript.currentMessages()

    override fun stateWireValue(): String = session.state().wireValue

    override fun stateDetail(): String? = latestStateDetail

    override fun currentModel(): String = session.currentSnapshot()?.currentModel.orEmpty()

    override fun adapterVersion(): Int = com.elon.app.chatgptweb.ChatGptWebPageAdapter.ADAPTER_VERSION

    override fun authenticated(): Boolean = session.currentSnapshot()?.authenticated == true

    override fun composerReady(): Boolean = session.currentSnapshot()?.composerReady == true

    override fun warmSessionAvailable(): Boolean = session.warmSessionAvailable()

    override fun prewarm(): Boolean {
        if (active || !session.warmSessionAvailable()) return false
        session.activate()
        return true
    }

    override fun streaming(): Boolean = session.currentSnapshot()?.streaming == true

    override fun attachmentSupported(): Boolean = session.currentSnapshot()?.capabilities
        ?.supports(com.elon.app.chatgptweb.ChatGptWebCapabilityId.ATTACHMENTS) == true

    override fun refreshComposerModel() = updateComposerModel(currentModel())

    override fun attachmentSendPhase(): String = session.attachmentSendPhase()

    override fun pendingAttachmentCount(): Int = maxOf(session.pendingAttachmentCount(), pendingAttachments.size)
    override fun completedAttachmentCount(): Int = latestAttachmentCompletedCount
    override fun imagePreviewState(): String = session.imagePreviewState().name.lowercase()

    override fun showNativeImageGallery(): Boolean = session.showImageGallery(onCreateImageRequested)

    override fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>): Boolean {
        if (!active) return false
        if (session.pendingSendPrompt() != null) {
            val detail = if (session.pendingSendRequiresOfficialConfirmation()) {
                "上一条已发送，但回答尚未同步，请打开官网功能确认"
            } else {
                "上一条消息仍在处理，请稍候"
            }
            Toast.makeText(activity, detail, Toast.LENGTH_LONG).show()
            return true
        }
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
            pendingAttachmentPrompt = prompt
            this.pendingAttachments = pendingAttachments.toList()
            waitingForAttachmentCompletion = true
            transcript.requestFollowLatest()
            renderSnapshot(session.currentSnapshot() ?: return true)
            if (!session.sendAttachments(prompt, pendingAttachments)) {
                pendingAttachmentPrompt = null
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
        if (!session.sendReady()) {
            when (WebChatSendFallbackPolicy.decide(
                loginRequired = session.state() == ChatGptBackgroundSession.State.LOGIN_REQUIRED,
            )) {
                WebChatSendFallbackPolicy.Action.RETRY_GUEST_ACCESS -> {
                    if (!session.retryGuestAccess()) session.onHostResumed()
                    Toast.makeText(activity, R.string.web_chat_guest_retrying, Toast.LENGTH_LONG).show()
                }
                WebChatSendFallbackPolicy.Action.RETRY_IN_PLACE -> {
                    session.onHostResumed()
                    Toast.makeText(activity, R.string.web_chat_not_ready, Toast.LENGTH_LONG).show()
                }
            }
            return true
        }
        transcript.requestFollowLatest()
        val result = session.dispatchSocialPrompt(prompt)
        when (result.outcome) {
            WebChatSendCoordinator.DispatchOutcome.DISPATCHED -> {
                binding.inputEdit.text?.clear()
                clearPendingSendState()
                collapseInputComposer()
            }
            WebChatSendCoordinator.DispatchOutcome.REJECTED -> {
                renderAfterPendingSendFailure()
                restorePrompt(result.prompt)
                Toast.makeText(activity, R.string.web_chat_not_ready, Toast.LENGTH_LONG).show()
            }
            WebChatSendCoordinator.DispatchOutcome.BUSY,
            WebChatSendCoordinator.DispatchOutcome.NOT_READY -> Unit
        }
        return true
    }

    override fun requestModelOptions() {
        if (!active) return
        modelPickerActive = true
        val observed = socialConsumerPort.state().composerSections[MODEL_SECTION].orEmpty()
        presentModelOptions(
            options = readModelOptions(observed),
            liveOptionIds = observed.mapTo(linkedSetOf(), WebChatConsumerOption::id),
        )
        if (interactionCache.needsComposerRefresh(provider.id, MODEL_SECTION)) {
            session.requestModelOptions()
        }
    }

    override fun stopGeneration() = session.stopGeneration()

    override fun startNewConversation(): Boolean {
        if (!session.startNewConversation()) return false
        realtimeVoiceTranscript.reset()
        clearPendingSend()
        latestSendCommandStatus = null
        pendingAttachmentPrompt = null
        lastMessageSnapshot = null
        transcript.requestFollowLatest()
        return true
    }

    override fun currentConversationPath(): String? = session.currentConversationPath()

    override fun officialFallbackUrl(): String? = session.currentOfficialUrl()

    override fun supportsWebSkin(): Boolean = true

    override fun showWebSkin(): Boolean = active && skinPresentation.enter()

    override fun showNativeMirror(): Boolean = skinPresentation.exit()

    override fun presentationMode(): String = session.presentationMode().name.lowercase()

    override fun conversationIndex() = session.conversationIndex()

    override fun requestConversationIndex(projectId: String?): Boolean =
        session.requestConversationIndex(projectId)

    fun probeConversationProject(path: String, projectId: String): Boolean =
        session.probeConversationProject(path, projectId)

    fun suspendConversationRefreshForUserAction() =
        session.suspendConversationRefreshForUserAction()

    fun resumeConversationRefreshAfterUserAction() =
        session.resumeConversationRefreshAfterUserAction()

    override fun openConversation(path: String): Boolean {
        realtimeVoiceTranscript.reset()
        clearPendingSend()
        latestSendCommandStatus = null
        pendingAttachmentPrompt = null
        lastMessageSnapshot = null
        transcript.requestFollowLatest()
        return session.openConversation(path).also { opened ->
            if (!opened) transcript.cancelFollowLatest()
        }
    }

    override fun openProject(path: String): Boolean {
        realtimeVoiceTranscript.reset()
        clearPendingSend()
        latestSendCommandStatus = null
        pendingAttachmentPrompt = null
        lastMessageSnapshot = null
        return session.openProject(path)
    }

    override fun mcpPort(): WebChatSocialMcpPort = socialMcpPort

    override fun consumerPort(): WebChatConsumerPort = socialConsumerPort

    fun privateDictationPort(): WebChatPrivateDictationPort = privateDictation

    override fun beginRealtimeVoiceBacking(): Boolean {
        val started = session.beginRealtimeVoiceBacking()
        if (!started) return false
        realtimeVoiceTranscript.begin(session.currentSnapshot())
        return true
    }

    override fun startManagedRealtimeVoice(): Boolean {
        val started = session.startManagedRealtimeVoice()
        if (!started) return false
        realtimeVoiceTranscript.begin(session.currentSnapshot())
        return true
    }

    override fun managedRealtimeVoiceState(): WebChatManagedRealtimeVoiceState =
        session.managedRealtimeVoiceState()

    override fun setManagedRealtimeVoiceMuted(muted: Boolean): Boolean =
        session.setManagedRealtimeVoiceMuted(muted)

    private fun handleRealtimeVoiceTranscript(event: ChatGptWebNativeVoiceTranscriptEvent) {
        realtimeVoiceTranscript.applyLive(event)?.let(::presentSnapshot)
    }

    override fun endRealtimeVoiceBacking(gracefulExit: Boolean) {
        val immediate = realtimeVoiceTranscript.end(session.currentSnapshot())
        immediate?.let(::renderSnapshot)
        if (immediate == null && !transcript.hasMessages()) {
            renderStatusMessage("正在整理语音记录…")
        }
        session.endRealtimeVoiceBacking(gracefulExit)
    }

    override fun lastCommandStatus(): WebChatCommandStatus? = latestCommandStatus

    override fun lastSendCommandStatus(): WebChatCommandStatus? = latestSendCommandStatus

    override fun discardAcceptanceAttachmentSend(): Boolean {
        if (waitingForAttachmentCompletion) return false
        val hadFixture = pendingAttachments.any {
            ChatGptWebAcceptanceAttachmentFixture.matches(activity.cacheDir, it)
        }
        if (!hadFixture) return false
        pendingAttachmentPrompt = null
        pendingAttachments = emptyList()
        session.currentSnapshot()?.let(::renderSnapshot)
        return true
    }

    override fun retryGuestAccess(): Boolean = session.retryGuestAccess()

    override fun retryConnection(): Boolean = session.retryConnection()

    override fun onHostResumed() = session.onHostResumed()

    override fun onHostPaused() = session.onHostPaused()
    override fun destroy() {
        clearPendingSend()
        privateDictation.destroy()
        productionMessageActions.release()
        skinPresentation.destroy()
        session.destroy()
    }

    private fun setInputTextFromMcp(value: String) {
        binding.inputEdit.setText(value)
        binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
    }

    private fun renderSnapshot(snapshot: ChatGptWebSnapshot) {
        privateDictation.observeOfficialDraft(snapshot.draft)
        if (pendingOfficialDictationDraft) {
            setInputTextFromMcp(snapshot.draft)
            pendingOfficialDictationDraft = false
        }
        onConsumerStateObserved(socialConsumerPort.state())
        val voicePresentation = realtimeVoiceTranscript.resolve(snapshot)
        if (voicePresentation == null) {
            if (active) {
                updateComposerModel(snapshot.currentModel)
                onComposerStateChanged()
            }
            return
        }
        presentSnapshot(voicePresentation)
    }

    private fun presentSnapshot(voicePresentation: ChatGptWebSnapshot) {
        val pendingTextPrompt = session.pendingSendPrompt()
        val presentationSnapshot = WebChatPendingSendSnapshotPresentation.resolve(
            previous = lastMessageSnapshot,
            incoming = voicePresentation,
            pending = pendingTextPrompt != null,
        )
        if (voicePresentation.messages.isNotEmpty()) lastMessageSnapshot = voicePresentation
        val pendingStatus = if (pendingAttachments.isNotEmpty()) {
            when (session.attachmentSendPhase()) {
                "uploading" -> "上传中…"
                "failed" -> "发送失败，请重新点击发送"
                else -> "发送中…"
            }
        } else {
            session.pendingSendStatus() ?: "发送中…"
        }
        val displayedPendingPrompt = pendingAttachmentPrompt ?: pendingTextPrompt
        val mapped = ChatGptFriendMessageMapper.map(
            snapshot = presentationSnapshot,
            provider = provider,
            pendingPrompt = displayedPendingPrompt,
            pendingAttachments = chatAttachmentsFromPending(pendingAttachments),
            pendingSendStatus = pendingStatus,
            attachmentsForMessage = { id -> sentAttachments[id].orEmpty() },
            messageActionContextIds = WebChatProductionMessageActionControls.messageContextIds(
                socialConsumerPort.state().controls,
            ),
            imagePreviewPath = session::imagePreviewPath,
            timestampFor = transcript::timestampFor,
        )
        if (pendingTextPrompt != null) {
            mapped.lastOrNull { message ->
                message.role == "user" && message.content.trim() == pendingTextPrompt.trim()
            }?.sendStatus = pendingStatus
        }
        val presented = WebChatProductionHistoryNotice.prepend(
            snapshot = presentationSnapshot,
            provider = provider,
            messages = mapped,
            timestampFor = transcript::timestampFor,
        )
        transcript.submit(presented, active)
        if (!active) return
        updateComposerModel(voicePresentation.currentModel)
        onComposerStateChanged()
    }

    private fun renderState(state: ChatGptBackgroundSession.State, detail: String?) {
        latestStateDetail = detail?.takeIf(String::isNotBlank)
            ?.takeIf { state == ChatGptBackgroundSession.State.ERROR }
        if (!active) return
        if (ChatGptWebConnectionMessagePolicy.shouldShow(
                state = state,
                hasMessages = transcript.hasMessages(),
                conversationNavigationActive = session.conversationNavigationActive(),
                warmSessionAvailable = session.warmSessionAvailable(),
            )) {
            renderStatusMessage("正在连接 ChatGPT 网页 AI…")
        }
        if (!transcript.hasMessages()) when (state) {
            ChatGptBackgroundSession.State.LOGIN_REQUIRED -> renderStatusMessage(
                "当前页面需要登录。可打开“官网功能”登录，也可在官网支持时直接匿名聊天。",
            )
            ChatGptBackgroundSession.State.ERROR -> renderStatusMessage(
                detail?.takeIf(String::isNotBlank) ?: "ChatGPT 网页 AI 暂时不可用。",
            )
            ChatGptBackgroundSession.State.IDLE,
            ChatGptBackgroundSession.State.LOADING,
            ChatGptBackgroundSession.State.READY -> Unit
        }
        if (state == ChatGptBackgroundSession.State.ERROR && !detail.isNullOrBlank()) {
            Toast.makeText(activity, detail, Toast.LENGTH_LONG).show()
        }
        onComposerStateChanged()
    }

    private fun handleCommandResult(
        event: ChatGptWebEvent.CommandResult,
        sendReceipt: ChatGptWebSendReceipt?,
    ) {
        privateDictation.onCommandResult(event.action, event.ok, event.detail)
        if (event.action == "submit_dictation" && event.ok) pendingOfficialDictationDraft = true
        if (event.action == "cancel_dictation" && event.ok) pendingOfficialDictationDraft = false
        val status = WebChatCommandStatus(
            action = event.action,
            ok = event.ok,
            detail = event.detail,
            observedAtMs = System.currentTimeMillis(),
        )
        latestCommandStatus = status
        if (event.action == "send_prompt") latestSendCommandStatus = status
        if (event.action in DICTATION_COMMAND_RESULTS) {
            onDictationCommandResult(event.action, event.ok)
        }
        if (sendReceipt?.origin != ChatGptWebSendOrigin.SOCIAL) return
        if (sendReceipt.indeterminate) {
            session.currentSnapshot()?.let(::renderSnapshot)
            Toast.makeText(
                activity,
                ChatGptWebPrivateTextReceiptPolicy.userDetail(event),
                Toast.LENGTH_LONG,
            ).show()
            return
        }
        if (event.ok) {
            session.currentSnapshot()?.let(::renderSnapshot)
            return
        }
        renderAfterPendingSendFailure()
        restorePrompt(sendReceipt.failedPrompt)
        Toast.makeText(
            activity,
            event.detail.ifBlank { activity.getString(R.string.chatgpt_native_command_failed) },
            Toast.LENGTH_LONG,
        ).show()
    }

    private fun handleAttachmentSendUpdate(update: ChatGptWebAttachmentSendUpdate) {
        latestAttachmentCompletedCount = update.completedAttachmentCount
        when (update.phase) {
            "completed" -> {
                update.userMessageId?.let { id ->
                    sentAttachments[id] = chatAttachmentsFromPending(pendingAttachments)
                }
                pendingAttachmentPrompt = null
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
        if (active) onComposerStateChanged()
    }

    private fun handlePendingSendTimeout(result: WebChatPendingSendState.TimeoutResult) {
        when (result.action) {
            WebChatPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION -> {
                session.currentSnapshot()?.let(::renderSnapshot)
                if (active) Toast.makeText(
                    activity,
                    "官网已确认发送，但回答同步较慢，可继续等待或打开官网功能确认",
                    Toast.LENGTH_LONG,
                ).show()
            }
            WebChatPendingSendState.TimeoutAction.REQUIRE_RECONCILIATION -> {
                session.currentSnapshot()?.let(::renderSnapshot)
                if (active) Toast.makeText(
                    activity,
                    "发送结果暂未确认，为避免重复发送，请打开官网功能核对",
                    Toast.LENGTH_LONG,
                ).show()
            }
            WebChatPendingSendState.TimeoutAction.RESTORE -> {
                renderAfterPendingSendFailure()
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

    private fun clearPendingSend() = session.clearPendingSend()

    private fun renderAfterPendingSendFailure() {
        (lastMessageSnapshot ?: session.currentSnapshot())?.let(::renderSnapshot)
    }

    private fun restorePrompt(prompt: String?) {
        if (!active || prompt.isNullOrBlank() || !binding.inputEdit.text.isNullOrBlank()) return
        binding.inputEdit.setText(prompt)
        binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
    }

    private fun renderStatusMessage(content: String) {
        transcript.showStatus(provider, content)
    }

    private fun handleWebChatMessageAction(message: ChatMessage, action: WebChatMessageAction) {
        productionMessageActions.handle(message, action)
    }

    private fun showModelOptions(options: List<ChatGptWebComposerOption>) {
        if (!active) return
        val observed = socialConsumerPort.state().composerSections[MODEL_SECTION].orEmpty()
            .ifEmpty { options.mapNotNull(ChatGptConsumerModelOptionMapper::map) }
        val resolved = interactionCache.replaceComposerOptions(provider.id, MODEL_SECTION, observed)
        if (modelPickerActive) {
            presentModelOptions(
                options = resolved,
                liveOptionIds = observed.mapTo(linkedSetOf(), WebChatConsumerOption::id),
            )
            resolvePendingPresetModel(resolved)
        }
    }

    private fun readModelOptions(
        observed: List<WebChatConsumerOption>,
    ): List<WebChatConsumerOption> =
        interactionCache.composerOptions(
            provider.id,
            MODEL_SECTION,
            observed,
        )

    private fun presentModelOptions(
        options: List<WebChatConsumerOption>,
        liveOptionIds: Set<String>,
    ) {
        val selectable = options.filter { it.id.isNotBlank() && it.label.isNotBlank() }
        val rangeBinding = WebChatModelRangePolicy.resolve(
            socialConsumerPort.state().controls.map(WebChatConsumerControlDescriptor::control),
        )
        val displayed = rangeBinding?.options ?: selectable
        val presentation = WebChatModelControlPolicy.resolve(displayed, currentModel())
        modelRangeSelectionById = rangeBinding?.selections.orEmpty()
        modelLiveOptionIds = liveOptionIds
        modelOptionById = (displayed + listOfNotNull(presentation.advanced))
            .associateBy(WebChatConsumerOption::id)
        modelPopup?.let {
            it.update(displayed, currentModel())
            return
        }
        val anchor = (binding.modelButton.parent as? View) ?: binding.modelButton
        modelPopup = WebChatModelControlPopup.show(
            activity = activity,
            anchor = anchor,
            options = displayed,
            currentModel = currentModel(),
            onOptionSelected = ::selectModelOption,
            onProviderSwitch = openProviderPicker,
            onDismissed = {
                modelPopup = null
                modelOptionById = emptyMap()
                modelLiveOptionIds = emptySet()
                modelRangeSelectionById = emptyMap()
                modelPickerActive = false
                pendingPresetModelLabel = null
                socialConsumerPort.dismissComposerOptions()
            },
        )
        if (modelPopup == null) {
            modelPickerActive = false
            modelOptionById = emptyMap()
            modelLiveOptionIds = emptySet()
            modelRangeSelectionById = emptyMap()
            pendingPresetModelLabel = null
        }
    }

    private fun selectModelOption(option: WebChatConsumerOption) {
        modelRangeSelectionById[option.id]?.let { selection ->
            socialConsumerPort.updateControl(
                selection.controlId,
                WebChatConsumerControlMutation.Slider(selection.value),
            )
            return
        }
        if (WebChatProductionBuiltInCatalog.isPresetId(option.id)) {
            if (
                !option.opensSubmenu &&
                WebChatModelControlPolicy.compactLabel(option.label) ==
                WebChatModelControlPolicy.compactLabel(currentModel())
            ) {
                modelPopup?.dismiss()
                return
            }
            pendingPresetModelLabel = option.label.takeUnless { option.opensSubmenu }
            session.requestModelOptions()
            return
        }
        if (option.opensSubmenu && option.id !in modelLiveOptionIds) {
            socialConsumerPort.dismissComposerOptions()
            session.requestModelOptions()
            return
        }
        modelOptionById[option.id]?.let { session.selectModel(it.id) }
    }

    private fun resolvePendingPresetModel(options: List<WebChatConsumerOption>) {
        val expected = pendingPresetModelLabel ?: return
        pendingPresetModelLabel = null
        val compactExpected = WebChatModelControlPolicy.compactLabel(expected)
        val live = options.firstOrNull { option ->
            option.id in modelLiveOptionIds &&
                WebChatModelControlPolicy.compactLabel(option.label) == compactExpected
        } ?: return
        session.selectModel(live.id)
        modelPopup?.dismiss()
    }

    private fun updateComposerModel(model: String) {
        if (!active) return
        WebChatComposerProviderPresentation.applyChatGptModelLevel(
            binding.modelButton,
            provider,
            model.ifBlank { "ChatGPT 自动" },
        )
    }

    private companion object {
        val DICTATION_COMMAND_RESULTS = setOf(
            "start_dictation",
            "submit_dictation",
            "cancel_dictation",
        )
        const val MODEL_SECTION = "model"
    }
}
