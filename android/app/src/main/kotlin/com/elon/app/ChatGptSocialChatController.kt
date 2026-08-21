package com.elon.app

import android.graphics.Rect
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
import com.elon.app.chatgptweb.ChatGptWebPresentationMode
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
    private val onConversationIndexChanged: () -> Unit,
    private val onComposerStateChanged: () -> Unit,
    private val interactionCache: WebChatProductionInteractionCache,
    audioPermissionController: ChatGptWebAudioPermissionController,
) : WebChatSocialController {
    override val providerId = WebChatProviderId.CHATGPT_WEB
    private val transcript = WebChatProductionTranscript(
        list = binding.chatList,
        setChatAdapter = setChatAdapter,
        onMessageLongPress = showMessageActions,
        onMessageAction = ::handleWebChatMessageAction,
        onContentOpen = { _, _ -> openOfficialFallback() },
    )
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
    private val skinPresentation = ChatGptWebSkinPresentationController(binding, session)
    private var provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
    private var active = false
    private var pendingPrompt: String? = null
    private var pendingAttachments = emptyList<PendingAttachment>()
    private val sentAttachments = linkedMapOf<String, List<ChatAttachment>>()
    private var waitingForAttachmentCompletion = false
    private var latestCommandStatus: WebChatCommandStatus? = null
    private var latestStateDetail: String? = null
    private var modelPopup: WebChatModelControlPopupHandle? = null
    private var modelOptionById = emptyMap<String, WebChatConsumerOption>()
    private var modelLiveOptionIds = emptySet<String>()
    private var modelRangeSelectionById = emptyMap<String, WebChatModelRangeSelection>()
    private var modelPickerActive = false
    private var pendingPresetModelLabel: String? = null
    private var realtimeVoiceBackingStarted = false
    private var realtimeVoiceExitRecoveryActive = false
    private var realtimeVoiceOriginPath: String? = null
    private var realtimeVoiceHadTranscript = false
    private val socialMcpPort: WebChatSocialMcpPort by lazy {
        session.createMcpPort(
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
            revealMessage = ::revealMessageFromMcp,
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
        resetRealtimeVoiceExitPresentation()
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
            transcript.requestFollowLatest()
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
        pendingPrompt = prompt
        transcript.requestFollowLatest()
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

    override fun startNewConversation() {
        resetRealtimeVoiceExitPresentation()
        pendingPrompt = null
        transcript.requestFollowLatest()
        session.startNewConversation()
    }

    override fun currentConversationPath(): String? = session.currentConversationPath()

    override fun officialFallbackUrl(): String? = session.currentOfficialUrl()

    override fun supportsWebSkin(): Boolean = true

    override fun showWebSkin(): Boolean = active && skinPresentation.enter()

    override fun showNativeMirror(): Boolean = skinPresentation.exit()

    override fun presentationMode(): String = session.presentationMode().name.lowercase()

    override fun conversationIndex() = session.conversationIndex()

    override fun requestConversationIndex(): Boolean = session.requestConversationIndex()

    override fun openConversation(path: String): Boolean {
        resetRealtimeVoiceExitPresentation()
        pendingPrompt = null
        transcript.requestFollowLatest()
        return session.openConversation(path).also { opened ->
            if (!opened) transcript.cancelFollowLatest()
        }
    }

    override fun openProject(path: String): Boolean {
        resetRealtimeVoiceExitPresentation()
        pendingPrompt = null
        return session.openProject(path)
    }

    override fun mcpPort(): WebChatSocialMcpPort = socialMcpPort

    override fun consumerPort(): WebChatConsumerPort = socialConsumerPort

    override fun beginRealtimeVoiceBacking(): Boolean {
        val started = session.beginRealtimeVoiceBacking()
        if (!started) return false
        realtimeVoiceBackingStarted = true
        realtimeVoiceExitRecoveryActive = false
        realtimeVoiceOriginPath = session.currentConversationPath()
        realtimeVoiceHadTranscript = transcript.hasMessages()
        return true
    }

    override fun endRealtimeVoiceBacking() {
        if (realtimeVoiceBackingStarted) {
            realtimeVoiceBackingStarted = false
            realtimeVoiceExitRecoveryActive = true
            if (!realtimeVoiceHadTranscript) {
                renderStatusMessage("正在同步语音会话…")
            }
        }
        session.endRealtimeVoiceBacking()
    }

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

    override fun retryGuestAccess(): Boolean = session.retryGuestAccess()

    override fun retryConnection(): Boolean = session.retryConnection()

    override fun onHostResumed() = session.onHostResumed()

    override fun onHostPaused() = session.onHostPaused()

    override fun destroy() {
        skinPresentation.destroy()
        session.destroy()
    }

    private fun setInputTextFromMcp(value: String) {
        binding.inputEdit.setText(value)
        binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
    }

    private fun revealMessageFromMcp(messageId: String, partIndex: Int?, target: String): Boolean {
        val nativeId = "${provider.id.wireValue}:$messageId"
        val index = transcript.indexOfMessageId(nativeId)
        if (index < 0) return false
        val message = transcript.messageAt(index) ?: return false
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
        if (WebChatRealtimeVoiceExitPresentationPolicy.shouldHoldCurrentTranscript(
                recoveryActive = realtimeVoiceExitRecoveryActive,
                originConversationPath = realtimeVoiceOriginPath,
                hadTranscriptBeforeVoice = realtimeVoiceHadTranscript,
                incoming = snapshot,
            )) {
            if (active) {
                updateComposerModel(snapshot.currentModel)
                onComposerStateChanged()
            }
            return
        }
        resetRealtimeVoiceExitPresentation()
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
            messageActionContextIds = WebChatProductionMessageActionControls.messageContextIds(
                socialConsumerPort.state().controls,
            ),
            timestampFor = transcript::timestampFor,
        )
        val presented = WebChatProductionHistoryNotice.prepend(
            snapshot = snapshot,
            provider = provider,
            messages = mapped,
            timestampFor = transcript::timestampFor,
        )
        transcript.submit(presented, active)
        if (!active) return
        updateComposerModel(snapshot.currentModel)
        onComposerStateChanged()
    }

    private fun resetRealtimeVoiceExitPresentation() {
        realtimeVoiceBackingStarted = false
        realtimeVoiceExitRecoveryActive = false
        realtimeVoiceOriginPath = null
        realtimeVoiceHadTranscript = false
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
        transcript.showStatus(provider, content)
    }

    private fun handleWebChatMessageAction(message: ChatMessage, action: WebChatMessageAction) {
        productionMessageActions.handle(message, action)
    }

    private fun showModelOptions(options: List<ChatGptWebComposerOption>) {
        if (!active) return
        val observed = socialConsumerPort.state().composerSections[MODEL_SECTION].orEmpty()
            .ifEmpty { options.mapNotNull(::consumerModelOption) }
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

    private fun consumerModelOption(option: ChatGptWebComposerOption): WebChatConsumerOption? {
        val id = option.id.trim()
        val label = option.label.trim()
        if (id.isBlank() || label.isBlank()) return null
        return WebChatConsumerOption(
            id = id,
            label = label,
            selected = option.selected,
            semantic = option.semantic,
            opensSubmenu = option.opensSubmenu,
            nativeSelector = "web-chat-model-option:" +
                ChatGptNativeControlPresentation.stableContextId(id),
            parentId = option.parentId,
            parentLabel = option.parentLabel,
        )
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
        const val MODEL_SECTION = "model"
        const val MAX_REVEAL_ATTEMPTS = 8
        const val REVEAL_RETRY_DELAY_MS = 80L
    }
}
