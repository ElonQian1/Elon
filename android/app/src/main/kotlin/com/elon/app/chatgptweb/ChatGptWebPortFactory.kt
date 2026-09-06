package com.elon.app.chatgptweb

import com.elon.app.WebChatConsumerPort
import com.elon.app.WebChatSendCoordinator
import com.elon.app.WebChatSocialMcpPort

internal class ChatGptWebPortFactory(
    private val ensureInitialized: () -> Unit,
    private val pageAdapter: () -> ChatGptWebPageAdapter?,
    private val sendOwner: ChatGptWebSendOwner,
    private val observedState: ChatGptWebObservedState,
    private val audioPermissionController: ChatGptWebAudioPermissionController,
    private val snapshot: () -> ChatGptWebSnapshot?,
    private val uiManifest: () -> ChatGptWebUiManifest?,
    private val bridgeState: () -> ChatGptWebPageAdapter.State,
    private val presentationMode: () -> ChatGptWebPresentationMode,
    private val verificationEvidenceStore: ChatGptWebVerificationEvidenceStore,
    private val requestComposerOptions: (String, String) -> Unit,
    private val dismissComposerOptions: (String?) -> Unit,
    private val refresh: () -> Unit,
    private val realtimeVoiceBacking: ChatGptRealtimeVoiceBackingController,
) {
    fun createMcpPort(
        inputText: () -> String,
        setInputText: (String) -> Unit,
        copyMessage: (String) -> ChatGptClipboardMetadata,
        selectMode: (ChatGptWebPresentationMode) -> Unit,
        revealMessage: (String, Int?, String) -> Boolean,
    ): WebChatSocialMcpPort {
        ensureInitialized()
        val adapter = checkNotNull(pageAdapter()) { "ChatGPT background session is not active" }
        val commands = ChatGptWebMcpCommandAdapter(
            pageAdapter = adapter,
            sendInputAction = { requestId ->
                val result = sendOwner.dispatchMcp(inputText().trim(), requestId)
                if (result.outcome != WebChatSendCoordinator.DispatchOutcome.DISPATCHED) {
                    observedState.failCommand(
                        requestId,
                        "send_prompt",
                        when (result.outcome) {
                            WebChatSendCoordinator.DispatchOutcome.BUSY -> "send_busy"
                            WebChatSendCoordinator.DispatchOutcome.NOT_READY -> "send_not_ready"
                            WebChatSendCoordinator.DispatchOutcome.REJECTED -> "send_rejected"
                            WebChatSendCoordinator.DispatchOutcome.DISPATCHED -> "send_dispatched"
                        },
                    )
                }
            },
            invokeControlAction = adapter::invokeUiControl,
            startDictationAction = { nativeDraft, expectedOfficialDraft, requestId ->
                audioPermissionController.runWithMicrophone(
                    action = {
                        adapter.startDictation(
                            nativeDraft = nativeDraft,
                            expectedOfficialDraft = expectedOfficialDraft,
                            requestId = requestId,
                        )
                    },
                    onPermissionDenied = {
                        observedState.failCommand(
                            requestId,
                            "start_dictation",
                            "microphone_permission_denied",
                        )
                    },
                )
            },
            requestComposerOptionsAction = requestComposerOptions,
            dismissComposerOptionsAction = dismissComposerOptions,
            deleteConversationAction = { path, requestId ->
                val error = ChatGptConversationDeletionGuard.rejection(path, snapshot(), inputText(),
                    sendOwner.prompt() != null || sendOwner.hasAttachmentSend())
                    ?: realtimeVoiceBacking.conversationDeletion.begin(requestId, realtimeVoiceBacking.isActive(), path, snapshot()?.url)
                if (error != null) observedState.failCommand(requestId, "delete_conversation", error)
                else adapter.deleteConversation(path, requestId)
            },
        )
        val officialPort = ChatGptWebMcpActions(
            snapshot = snapshot,
            uiManifest = uiManifest,
            observedState = observedState::snapshot,
            beginCommand = observedState::beginCommand,
            bridgeState = bridgeState,
            mode = presentationMode,
            inputText = inputText,
            audioPermissionState = audioPermissionController::snapshot,
            verificationEvidence = verificationEvidenceStore::snapshot,
            recordVerificationCases = verificationEvidenceStore::record,
            setInputText = setInputText,
            copyMessage = copyMessage,
            commands = commands,
            refresh = refresh,
            selectMode = selectMode,
            revealMessage = revealMessage,
            beginOpenConversationCommand = observedState::beginOpenConversationCommand,
            beginConversationFilesCommand = observedState::beginConversationFilesCommand,
        )
        return ChatGptWebNativeVoiceResearchMcpPort(
            delegate = officialPort,
            startNative = realtimeVoiceBacking::beginNativePrivateVoiceResearch,
            muteNative = realtimeVoiceBacking::muteNativePrivateVoiceResearch,
            stopNative = { realtimeVoiceBacking.end(gracefulExit = true) },
            currentState = realtimeVoiceBacking::nativePrivateVoiceState,
        )
    }

    fun createConsumerPort(mcpPort: WebChatSocialMcpPort): WebChatConsumerPort =
        ChatGptWebConsumerPortAdapter(
            snapshot = snapshot,
            uiManifest = uiManifest,
            observedState = observedState::snapshot,
            executeControl = mcpPort::control,
        )
}
