package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import android.widget.Toast
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient

internal class MainInputActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val speechPermissionRequest: Int,
    private val userId: () -> String,
    private val projects: MutableList<AppProject>,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val uiTools: () -> MainUiTools,
    private val modelActions: () -> MainModelActions,
    private val projectStateActions: () -> MainProjectStateActions,
    private val conversationTaskRegistryActions: () -> MainConversationTaskRegistryActions,
    private val workflowActions: () -> MainWorkflowActions,
    private val preparedMessageActions: () -> MainPreparedMessageActions,
    private val activeWorkControlActions: () -> MainActiveWorkControlActions,
    private val messageActions: () -> MainMessageActions,
    private val navigationController: () -> MainNavigationController,
    private val stageHintShimmer: () -> MainStageHintShimmer,
    private val isFriendChatActive: () -> Boolean,
    private val isSocialAiChatActive: () -> Boolean,
    private val trySendFriendMessage: (String, List<PendingAttachment>) -> Boolean,
    private val forkForRunningInput: (String, String) -> ForkedConversation,
    private val startTaskWorkService: (String, String?, Boolean, String?) -> Boolean
) {
    private val pendingAttachments = mutableListOf<PendingAttachment>()
    private var inputComposerViews: MainInputComposerViews? = null
    private var pendingAttachmentPreviewStrip: PendingAttachmentPreviewStrip? = null
    private var voiceMode = false
    private var inputCanSend = true
    private var runningInputMode = RunningInputMode.REMIND_CURRENT
    private var suppressInputFocusAnimation = false
    private var speechInputActions: MainSpeechInputActions? = null
    private var keyboardInsetsAnimationActions: MainKeyboardInsetsAnimationActions? = null
    private var fullScreenEditorOverlay: FullScreenEditorOverlay? = null

    fun setupInputComposer() {
        fullScreenEditorOverlay = FullScreenEditorOverlay(
            activity = activity,
            dp = uiTools()::dp,
            getInputText = { binding.inputEdit.text.toString() },
            setInputText = { text ->
                binding.inputEdit.setText(text)
                binding.inputEdit.setSelection(text.length)
            },
            onSend = { sendMessageActions.sendMessage() }
        )

        emojiActions.setupEmojiLaunchers()
        val views = MainInputComposerSetup(
            activity = activity,
            binding = binding,
            dp = uiTools()::dp,
            currentModelLabel = { modelActions().currentModelLabel },
            isVoiceMode = { voiceMode },
            shouldAnimateInputFocus = { !suppressInputFocusAnimation },
            isAttachmentPanelOpen = { attachmentPanelActions.isOpen },
            isEmojiPanelOpen = { emojiActions.isOpen },
            toggleVoiceMode = { voiceModeActions.toggleVoiceMode() },
            focusInputComposer = { inputFocusActions.focusInputComposer() },
            startSpeechToText = { speechInputActions().startSpeechToText() },
            stopSpeechToText = { speechInputActions().stopSpeechToText() },
            cancelSpeechToText = { speechInputActions().cancelSpeechToText() },
            onVoiceTouchMove = { rawX, rawY -> speechInputActions().onVoiceTouchMove(rawX, rawY) },
            showModelPopupOrLoad = { modelActions().showModelPopupOrLoad() },
            sendMessage = { sendMessageActions.sendMessage() },
            toggleAttachmentPanel = { attachmentPanelActions.toggleAttachmentPanel() },
            toggleEmojiPanel = { emojiActions.toggleEmojiPanel() },
            buildAttachmentPanel = { attachmentPanelActions.buildAttachmentPanel() },
            buildEmojiPanel = { emojiActions.buildEmojiPanel() },
            collapseAttachmentPanel = { attachmentPanelActions.collapseAttachmentPanel() },
            collapseEmojiPanel = { emojiActions.collapseEmojiPanel() },
            collapseInputComposer = { inputFocusActions.collapseInputComposer() },
            updateCollapsedInputPreview = { collapsedInputPreviewActions.updateCollapsedInputPreview() },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = { adaptiveInputHeightActions.updateAdaptiveInputHeight() },
            selectRunningInputMode = { mode ->
                runningInputMode = mode
                updateRunningInputModeStrip()
            },
            showFullScreenEditor = { fullScreenEditorOverlay?.show() }
        ).setup()

        inputComposerViews = views
        pendingAttachmentPreviewStrip = PendingAttachmentPreviewStrip(activity, pendingAttachments) {
            collapsedInputPreviewActions.updateCollapsedInputPreview()
            updateSendButtonVisual()
        }
        binding.inputLayout.addView(requireNotNull(pendingAttachmentPreviewStrip).view, 1)
        voiceModeActions.applyVoiceMode()
        collapsedInputPreviewActions.updateCollapsedInputPreview()
        updateRunningInputModeStrip()
        updateSendButtonVisual()
        adaptiveInputHeightActions.updateAdaptiveInputHeight()
        keyboardInsetsAnimationActions = MainKeyboardInsetsAnimationActions(binding).also { it.install() }
    }

    fun inputComposerViewsOrNull(): MainInputComposerViews? = inputComposerViews

    /** 若全屏编辑器正在显示则关闭并返回 true，否则返回 false。用于返回键拦截。 */
    fun hideFullScreenEditorForBack(): Boolean {
        val overlay = fullScreenEditorOverlay ?: return false
        if (!overlay.isShowing()) return false
        overlay.hide()
        return true
    }

    fun hideInputOverlaysForBack(): Boolean {
        if (hideFullScreenEditorForBack()) return true
        if (emojiActions.collapseEmojiPanelForBack()) return true
        if (attachmentPanelActions.isOpen) {
            attachmentPanelActions.collapseAttachmentPanel()
            return true
        }
        return inputFocusActions.collapseInputComposerForBack()
    }

    fun updateRunningInputModeStrip() {
        inputComposerViewsOrNull()?.runtimeInputModeStrip?.refresh(
            visible = conversationTaskRegistryActions().isActiveConversationWorking() && !isFriendChatActive(),
            mode = runningInputMode
        )
    }

    fun destroySpeechInput() {
        speechInputActions?.destroy()
        speechInputActions = null
    }

    fun showVoiceAttachmentActions(message: ChatMessage, attachment: ChatAttachment) {
        speechInputActions().showVoiceAttachmentActions(message, attachment)
    }

    private fun sendVoiceAttachment(attachment: PendingAttachment, message: String) {
        if (pendingAttachments.size >= MAX_PENDING_ATTACHMENTS) {
            attachment.file.delete()
            Toast.makeText(activity, "一次最多发送 $MAX_PENDING_ATTACHMENTS 个附件", Toast.LENGTH_SHORT).show()
            return
        }
        pendingAttachments.add(attachment)
        binding.inputEdit.setText(message)
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
        if (voiceMode) {
            voiceMode = false
            voiceModeActions.applyVoiceMode()
        }
        refreshPendingAttachmentPreview()
        sendMessageActions.sendMessage()
    }

    private fun addCustomEmojiAttachment(item: CustomEmojiItem): Boolean {
        if (pendingAttachments.size >= MAX_PENDING_ATTACHMENTS) {
            Toast.makeText(activity, "一次最多发送 $MAX_PENDING_ATTACHMENTS 个附件", Toast.LENGTH_SHORT).show()
            return false
        }
        val attachment = CustomEmojiStore.toPendingAttachment(activity, item, pendingAttachments.size + 1)
        if (attachment == null) {
            Toast.makeText(activity, "这个表情文件已不存在", Toast.LENGTH_SHORT).show()
            return false
        }
        pendingAttachments.add(attachment)
        if (voiceMode) {
            voiceMode = false
            voiceModeActions.applyVoiceMode()
        }
        refreshPendingAttachmentPreview()
        return true
    }

    val adaptiveInputHeightActions: MainAdaptiveInputHeightActions by lazy {
        MainAdaptiveInputHeightActions(
            binding = binding,
            dp = uiTools()::dp,
            inputCenterContainer = { inputComposerViewsOrNull()?.inputCenterContainer },
            inputBarContainer = { inputComposerViewsOrNull()?.inputBarContainer },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            isVoiceMode = { voiceMode }
        )
    }

    val inputFocusActions: MainInputFocusActions by lazy {
        MainInputFocusActions(
            activity = activity,
            binding = binding,
            activeConversation = projectStateActions()::activeConversation,
            isFriendChatActive = isFriendChatActive,
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = { voiceModeActions.applyVoiceMode() },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            requestKeyboardLift = { keyboardInsetsAnimationActions?.requestKeyboardLift() },
            releaseKeyboardLift = { keyboardInsetsAnimationActions?.releaseKeyboardLift() },
            setSuppressInputFocusAnimation = { suppressInputFocusAnimation = it },
            collapseEmojiPanel = { emojiActions.collapseEmojiPanel() },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = { adaptiveInputHeightActions.updateAdaptiveInputHeight() }
        )
    }

    val collapsedInputPreviewActions: MainCollapsedInputPreviewActions by lazy {
        MainCollapsedInputPreviewActions(
            binding = binding,
            pendingAttachments = { pendingAttachments },
            collapsedInputPreview = { inputComposerViewsOrNull()?.collapsedInputPreview }
        )
    }

    private fun refreshPendingAttachmentPreview() {
        pendingAttachmentPreviewStrip?.refresh() ?: return
        collapsedInputPreviewActions.updateCollapsedInputPreview()
        updateSendButtonVisual()
    }

    val attachmentPickerActions: MainAttachmentPickerActions by lazy {
        MainAttachmentPickerActions(
            activity = activity,
            activeConversation = projectStateActions()::activeConversation,
            attachPickedFile = { kind, uri, fallbackName ->
                pendingAttachmentActions.attachPickedFile(kind, uri, fallbackName)
            }
        )
    }

    val sendMessageActions: MainSendMessageActions by lazy {
        MainSendMessageActions(
            binding = binding,
            pendingAttachments = pendingAttachments,
            collapseAttachmentPanel = {
                attachmentPanelActions.collapseAttachmentPanel()
                emojiActions.collapseEmojiPanel()
            },
            isActiveConversationWorking = { conversationTaskRegistryActions().isActiveConversationWorking() },
            runningInputMode = { runningInputMode },
            activeProject = projectStateActions()::activeProject,
            activeConversation = projectStateActions()::activeConversation,
            appendMessage = workflowActions().messageAppendActions::appendMessage,
            collapseInputComposer = { inputFocusActions.collapseInputComposer() },
            uploadAttachmentsThenSend = { visibleText, outgoingText, target ->
                attachmentSendActions.uploadAttachmentsThenSend(visibleText, outgoingText, target)
            },
            startPreparedMessage = preparedMessageActions()::startPreparedMessage,
            handleRunningInput = runningInputActions::handleRunningInput,
            trySendFriendMessage = trySendFriendMessage
        )
    }

    val runningInputActions: MainRunningInputActions by lazy {
        MainRunningInputActions(
            activity = activity,
            projects = { projects },
            activeConversation = projectStateActions()::activeConversation,
            activeConversationTask = conversationTaskRegistryActions()::activeConversationTask,
            appendMessage = workflowActions().messageAppendActions::appendMessage,
            updateMessage = workflowActions().messageAppendActions::updateMessage,
            startPreparedMessageAfterUserBubble = preparedMessageActions()::startPreparedMessageAfterUserBubble,
            startTaskWorkService = startTaskWorkService,
            forkForRunningInput = forkForRunningInput,
            expandOutgoing = ::expandShortDevelopmentCommand
        )
    }

    val sendTargetRestoreActions: MainSendTargetRestoreActions by lazy {
        MainSendTargetRestoreActions(
            binding = binding,
            projects = projects,
            setActiveProjectIndex = setActiveProjectIndex,
            setChatAdapter = setChatAdapter,
            pauseCurrentWork = { activeWorkControlActions().pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions().showMessageActions(anchor, message) },
            retryFailedAttachmentMessage = ::retryFailedAttachmentMessage,
            showChat = { navigationController().showChat() }
        )
    }

    val attachmentSendActions: MainAttachmentSendActions by lazy {
        MainAttachmentSendActions(
            activity = activity,
            http = http,
            serverUrl = serverUrl,
            userId = userId,
            pendingAttachments = { pendingAttachments.toList() },
            setSendEnabled = sendEnabledActions::setSendEnabled,
            appendMessage = workflowActions().messageAppendActions::appendMessage,
            updateMessage = workflowActions().messageAppendActions::updateMessage,
            startPreparedMessageAfterUserBubble = preparedMessageActions()::startPreparedMessageAfterUserBubble
        )
    }

    val pendingAttachmentActions: MainPendingAttachmentActions by lazy {
        MainPendingAttachmentActions(
            activity = activity,
            pendingAttachments = pendingAttachments,
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = { voiceModeActions.applyVoiceMode() },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            refreshPendingAttachmentPreview = ::refreshPendingAttachmentPreview
        )
    }

    val attachmentPanelActions: MainAttachmentPanelActions by lazy {
        MainAttachmentPanelActions(
            activity = activity,
            dp = uiTools()::dp,
            selectableForeground = uiTools()::selectableForeground,
            activeConversation = projectStateActions()::activeConversation,
            attachmentPanel = { inputComposerViewsOrNull()?.attachmentPanel },
            attachmentButton = { inputComposerViewsOrNull()?.attachmentButton },
            collapseInputComposer = { inputFocusActions.collapseInputComposer() },
            collapseEmojiPanel = { emojiActions.collapseEmojiPanel() },
            openCameraAttachment = { attachmentPickerActions.openCameraAttachment() },
            openPhotoAttachment = { attachmentPickerActions.openPhotoAttachment() },
            openDocumentAttachment = { attachmentPickerActions.openDocumentAttachment() }
        )
    }

    val emojiActions: MainEmojiActions by lazy {
        MainEmojiActions(
            activity = activity,
            binding = binding,
            dp = uiTools()::dp,
            selectableForeground = uiTools()::selectableForeground,
            activeConversation = projectStateActions()::activeConversation,
            emojiPanel = { inputComposerViewsOrNull()?.emojiPanel },
            emojiButton = { inputComposerViewsOrNull()?.emojiButton },
            collapseAttachmentPanel = { attachmentPanelActions.collapseAttachmentPanel() },
            addPendingEmojiAttachment = ::addCustomEmojiAttachment,
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = { adaptiveInputHeightActions.updateAdaptiveInputHeight() }
        )
    }

    val voiceModeActions: MainVoiceModeActions by lazy {
        MainVoiceModeActions(
            activity = activity,
            binding = binding,
            inputModeButton = { inputComposerViewsOrNull()?.inputModeButton },
            emojiButton = { inputComposerViewsOrNull()?.emojiButton },
            voiceHoldButton = { inputComposerViewsOrNull()?.voiceHoldButton },
            inputCenterContainer = { inputComposerViewsOrNull()?.inputCenterContainer },
            expandedInputContainer = { inputComposerViewsOrNull()?.expandedInputContainer },
            collapsedInputPreview = { inputComposerViewsOrNull()?.collapsedInputPreview },
            modelButtonShell = { inputComposerViewsOrNull()?.modelButtonShell },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            collapseAttachmentPanel = { attachmentPanelActions.collapseAttachmentPanel() },
            collapseEmojiPanel = { emojiActions.collapseEmojiPanel() },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = { adaptiveInputHeightActions.updateAdaptiveInputHeight() }
        )
    }

    private fun updateSendButtonVisual() {
        sendButtonVisualActions.updateSendButtonVisual()
    }

    val sendButtonVisualActions: MainSendButtonVisualActions by lazy {
        MainSendButtonVisualActions(
            activity = activity,
            binding = binding,
            dp = uiTools()::dp,
            attachmentButton = { inputComposerViewsOrNull()?.attachmentButton },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            isVoiceMode = { voiceMode },
            hasPendingAttachments = { pendingAttachments.isNotEmpty() },
            inputCanSend = { inputCanSend },
            activeConversation = projectStateActions()::activeConversation,
            isFriendChatActive = isFriendChatActive
        )
    }

    private fun speechInputActions(): MainSpeechInputActions {
        speechInputActions?.let { return it }
        return MainSpeechInputActions(
            activity = activity,
            binding = binding,
            http = http,
            serverUrl = serverUrl,
            speechPermissionRequest = speechPermissionRequest,
            userId = userId,
            selectedAgent = { modelActions().selectedAgentForRequest() },
            activeConversation = projectStateActions()::activeConversation,
            activeProject = projectStateActions()::activeProject,
            voiceHoldButton = { requireNotNull(inputComposerViews).voiceHoldButton },
            sendVoiceAttachment = ::sendVoiceAttachment,
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = { voiceModeActions.applyVoiceMode() },
            isFriendChatActive = isFriendChatActive,
            isSocialAiChatActive = isSocialAiChatActive,
            sendTextDirect = { text ->
                binding.inputEdit.setText(text)
                binding.inputEdit.setSelection(text.length)
                sendMessageActions.sendMessage()
            }
        ).also { speechInputActions = it }
    }

    val sendEnabledActions: MainSendEnabledActions by lazy {
        MainSendEnabledActions(
            binding = binding,
            activeConversation = projectStateActions()::activeConversation,
            isFriendChatActive = isFriendChatActive,
            setInputCanSend = { inputCanSend = it },
            inputModeButton = { inputComposerViewsOrNull()?.inputModeButton },
            emojiButton = { inputComposerViewsOrNull()?.emojiButton },
            voiceHoldButton = { inputComposerViewsOrNull()?.voiceHoldButton },
            modelButtonShell = { inputComposerViewsOrNull()?.modelButtonShell },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateStageHintShimmer = { stageHintShimmer().update() },
            isActiveConversationWorking = { conversationTaskRegistryActions().isActiveConversationWorking() },
            updateRunningInputModeStrip = ::updateRunningInputModeStrip
        )
    }

    fun retryFailedAttachmentMessage(message: ChatMessage) {
        if (conversationTaskRegistryActions().isActiveConversationWorking()) {
            Toast.makeText(activity, "当前会话正在执行，完成后再重试", Toast.LENGTH_SHORT).show()
            return
        }
        val project = projectStateActions().activeProject()
        val conversation = projectStateActions().activeConversation()
        if (conversation.ended) {
            Toast.makeText(activity, "这个会话已结束，请新建会话继续", Toast.LENGTH_SHORT).show()
            return
        }
        val visibleText = message.content.trim().ifBlank { "请看这张图片。" }
        val outgoingText = expandShortDevelopmentCommand(visibleText, conversation.messages)
        val target = SendTarget(project.id, project.title, conversation.id, conversation.title)
        attachmentSendActions.retryFailedAttachmentMessage(message, visibleText, outgoingText, target)
    }
}
