package com.elon.app.chatgptweb

internal sealed interface ChatGptWebEvent {
    data class DirectoryPage(val value: ChatGptWebDirectoryPage) : ChatGptWebEvent
    data class AdapterReady(
        val capabilities: ChatGptWebCapabilities,
    ) : ChatGptWebEvent

    data class Snapshot(val value: ChatGptWebSnapshot) : ChatGptWebEvent

    data class ConversationList(
        val conversations: List<ChatGptWebConversation>,
        val collection: ChatGptWebConversationCollection =
            ChatGptWebConversationCollection.official(conversations.size),
        val projects: List<ChatGptWebProject> = emptyList(),
        val scopeProjectId: String? = null,
        val removedConversationIds: Set<String> = emptySet(),
        val deletedConversationIds: Set<String> = emptySet(),
        val requestId: String? = null,
        val continueRefresh: Boolean = false,
    ) : ChatGptWebEvent

    data class ComposerControls(
        val section: String,
        val currentModel: String,
        val options: List<ChatGptWebComposerOption>,
    ) : ChatGptWebEvent

    data class FeatureNavigation(
        val features: List<ChatGptWebFeature>,
    ) : ChatGptWebEvent

    data class UiManifest(
        val value: ChatGptWebUiManifest,
    ) : ChatGptWebEvent

    data class WebTouchRequest(
        val purpose: String,
        val xRatio: Double,
        val yRatio: Double,
        val controlId: String? = null,
    ) : ChatGptWebEvent

    data class AttachmentTransport(
        val evidence: ChatGptWebAttachmentTransportEvidence,
    ) : ChatGptWebEvent

    data class ImageAsset(
        val value: ChatGptWebImageAsset,
    ) : ChatGptWebEvent

    data class ImageGallerySnapshot(
        val value: ChatGptWebImageGallerySnapshot,
    ) : ChatGptWebEvent

    data class ConversationFiles(val value: com.elon.app.WebChatConversationFileIndex) : ChatGptWebEvent
    data class LibraryFiles(val value: com.elon.app.WebChatLibrarySnapshot) : ChatGptWebEvent

    data class CommandResult(
        val action: String,
        val ok: Boolean,
        val detail: String,
        val requestId: String? = null,
    ) : ChatGptWebEvent
}
