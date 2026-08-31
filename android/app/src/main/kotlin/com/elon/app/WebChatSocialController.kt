package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationIndexState

internal data class WebChatCommandStatus(
    val action: String,
    val ok: Boolean,
    val detail: String,
    val observedAtMs: Long,
)

internal interface WebChatSocialController {
    val providerId: WebChatProviderId

    fun activate(identity: WebChatProviderIdentity)
    fun deactivate()
    fun isActive(): Boolean
    fun currentMessages(): List<ChatMessage>
    fun stateWireValue(): String
    fun stateDetail(): String? = null
    fun currentModel(): String
    fun adapterVersion(): Int
    fun authenticated(): Boolean
    fun composerReady(): Boolean
    fun warmSessionAvailable(): Boolean = false
    fun prewarm(): Boolean = false
    fun streaming(): Boolean
    fun attachmentSupported(): Boolean
    fun attachmentSendPhase(): String
    fun pendingAttachmentCount(): Int
    fun imagePreviewState(): String = "idle"
    fun showNativeImageGallery(): Boolean = false
    fun trySendMessage(rawText: String, pendingAttachments: List<PendingAttachment>): Boolean
    fun requestModelOptions()
    fun refreshComposerModel()
    fun stopGeneration()
    fun startNewConversation(): Boolean
    fun currentConversationPath(): String?
    fun officialFallbackUrl(): String? = null
    fun supportsWebSkin(): Boolean = false
    fun showWebSkin(): Boolean = false
    fun showNativeMirror(): Boolean = false
    fun presentationMode(): String = "native"
    fun conversationIndex(): ChatGptWebConversationIndexState
    fun requestConversationIndex(projectId: String? = null): Boolean
    fun openConversation(path: String): Boolean
    fun openProject(path: String): Boolean
    fun supportsLocalProjects(): Boolean = false
    fun createLocalProject(title: String): Boolean = false
    fun assignConversationToLocalProject(path: String, projectId: String?): Boolean = false
    fun consumerPort(): WebChatConsumerPort? = null
    fun beginRealtimeVoiceBacking(): Boolean = false
    fun startManagedRealtimeVoice(): Boolean = false
    fun managedRealtimeVoiceState(): WebChatManagedRealtimeVoiceState =
        WebChatManagedRealtimeVoiceState.Unavailable
    fun setManagedRealtimeVoiceMuted(muted: Boolean): Boolean = false
    fun endRealtimeVoiceBacking(gracefulExit: Boolean) = Unit
    fun mcpPort(): WebChatSocialMcpPort? = null
    fun lastCommandStatus(): WebChatCommandStatus? = null
    fun lastSendCommandStatus(): WebChatCommandStatus? = null
    fun discardAcceptanceAttachmentSend(): Boolean = false
    fun retryGuestAccess(): Boolean = false
    fun retryConnection(): Boolean = false
    fun onHostResumed()
    fun onHostPaused()
    fun destroy()
}
