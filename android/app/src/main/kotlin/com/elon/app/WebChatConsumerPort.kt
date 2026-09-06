package com.elon.app

internal data class WebChatConsumerOption(
    val id: String,
    val label: String,
    val selected: Boolean,
    val semantic: String,
    val opensSubmenu: Boolean,
    val nativeSelector: String,
    val parentId: String? = null,
    val parentLabel: String? = null,
)

internal data class WebChatConsumerFeature(
    val id: String,
    val label: String,
    val kind: String,
    val selected: Boolean,
    val requiresUserConfirmation: Boolean,
    val nativeSelector: String,
)

internal enum class WebChatConsumerCommandStatus {
    PENDING,
    SUCCEEDED,
    FAILED,
    TIMED_OUT,
    UNKNOWN,
}

internal data class WebChatConsumerCommandRequest(
    val id: String,
    val status: WebChatConsumerCommandStatus,
    val detail: String? = null,
)

internal data class WebChatConsumerState(
    val streaming: Boolean,
    val dictationActive: Boolean,
    val composerSections: Map<String, List<WebChatConsumerOption>>,
    val pageKind: String,
    val pageUrl: String,
    val features: List<WebChatConsumerFeature>,
    val controls: List<WebChatConsumerControlDescriptor> = emptyList(),
    val commandRequests: List<WebChatConsumerCommandRequest>,
    val adapterCurrent: Boolean = true,
    val dictationCaptureActive: Boolean = false,
    val dictationCapturePending: Boolean = false,
    val draftPresent: Boolean = false,
    val privateReadAloudReady: Boolean = false,
    val privateReadAloudState: String = "idle",
    val privateReadAloudContextId: String = "",
)

internal data class WebChatConsumerCommandResult(
    val accepted: Boolean,
    val error: String? = null,
    val requestId: String? = null,
)

internal interface WebChatConsumerPort {
    fun state(): WebChatConsumerState
    fun conversationFiles(path: String): WebChatConversationFileIndex? = null
    fun requestConversationFiles(path: String): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun downloadConversationFile(path: String, fileId: String, downloadHandle: String): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun requestComposerOptions(section: String): WebChatConsumerCommandResult
    fun dismissComposerOptions(): WebChatConsumerCommandResult
    fun selectComposerOption(section: String, optionId: String): WebChatConsumerCommandResult
    fun requestFeatures(): WebChatConsumerCommandResult
    fun selectFeature(featureId: String, userConfirmed: Boolean): WebChatConsumerCommandResult
    fun requestControls(): WebChatConsumerCommandResult
    fun revealProjectChoice(label: String): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun invokeControl(controlId: String, userConfirmed: Boolean): WebChatConsumerCommandResult
    fun invokeControlAfterTouchMiss(
        controlId: String,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult = invokeControl(controlId, userConfirmed)
    fun toggleOfficialReadAloud(contextId: String): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun setConversationPinned(
        conversationPath: String,
        pinned: Boolean,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun setConversationArchived(
        conversationPath: String,
        archived: Boolean,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun renameConversation(
        conversationPath: String,
        title: String,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun deleteConversation(conversationPath: String, userConfirmed: Boolean): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun shareConversation(conversationPath: String, userConfirmed: Boolean): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun moveConversationToProject(
        conversationPath: String,
        conversationTitle: String,
        projectId: String,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult =
        WebChatConsumerCommandResult(false, "unsupported_consumer_command")
    fun updateControl(
        controlId: String,
        mutation: WebChatConsumerControlMutation,
    ): WebChatConsumerCommandResult
    fun executeSessionCommand(action: String): WebChatConsumerCommandResult
}
