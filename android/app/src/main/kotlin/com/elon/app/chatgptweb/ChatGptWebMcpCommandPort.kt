package com.elon.app.chatgptweb

internal interface ChatGptWebMcpCommandPort {
    fun setDraft(value: String, expectedDraft: String, requestId: String)
    fun sendInput(requestId: String)
    fun invokeControl(controlId: String, requestId: String)
    fun invokeControlAfterTouchMiss(controlId: String, requestId: String) =
        invokeControl(controlId, requestId)
    fun setControlText(controlId: String, text: String, requestId: String)
    fun setControlSelected(controlId: String, selected: Boolean, requestId: String)
    fun selectControlChoice(controlId: String, choiceIndex: Int, requestId: String)
    fun setControlSlider(controlId: String, value: Double, requestId: String)
    fun setControlExpanded(controlId: String, expanded: Boolean, requestId: String)
    fun newConversation(requestId: String)
    fun stopGeneration(requestId: String)
    fun verifyPrivateStreamWatchdog(requestId: String)
    fun privateProtocolProbe(mode: String, requestId: String) = Unit
    fun regenerateResponse(requestId: String)
    fun togglePrivateReadAloud(contextId: String, requestId: String) = Unit
    fun setConversationPinned(path: String, pinned: Boolean, requestId: String) = Unit
    fun setConversationArchived(path: String, archived: Boolean, requestId: String) = Unit
    fun renameConversation(path: String, title: String, requestId: String) = Unit
    fun moveConversationToProject(
        path: String,
        conversationTitle: String,
        projectId: String,
        requestId: String,
    ) = Unit
    fun startDictation(nativeDraft: String, expectedOfficialDraft: String, requestId: String)
    fun cancelDictation(requestId: String)
    fun submitDictation(requestId: String)
    fun removeAttachment(attachmentId: String, requestId: String)
    fun refreshControls(requestId: String)
    fun revealProjectChoice(label: String, requestId: String) = Unit
    fun listConversations(requestId: String)
    fun listConversationFiles(path: String, requestId: String) = Unit
    fun downloadConversationFile(path: String, file: com.elon.app.WebChatConversationFile, requestId: String) = Unit
    fun requestComposerOptions(section: String, requestId: String)
    fun dismissComposerOptions(requestId: String)
    fun selectComposerOption(section: String, optionId: String, requestId: String)
    fun requestFeatures(requestId: String)
    fun dismissFeatures(requestId: String)
    fun selectFeature(featureId: String, requestId: String)
    fun openConversation(path: String, requestId: String)
}
