package com.elon.app.chatgptweb

internal class ChatGptWebMcpCommandAdapter(
    private val pageAdapter: ChatGptWebPageAdapter,
    private val sendInputAction: (String) -> Unit,
    private val invokeControlAction: (String, String) -> Unit,
    private val startDictationAction: (String, String, String) -> Unit,
    private val requestComposerOptionsAction: (String, String) -> Unit,
    private val dismissComposerOptionsAction: (String?) -> Unit,
) : ChatGptWebMcpCommandPort {
    override fun setDraft(value: String, expectedDraft: String, requestId: String) =
        pageAdapter.setDraft(value, expectedDraft, requestId)

    override fun sendInput(requestId: String) = sendInputAction(requestId)

    override fun invokeControl(controlId: String, requestId: String) =
        invokeControlAction(controlId, requestId)

    override fun invokeControlAfterTouchMiss(controlId: String, requestId: String) =
        pageAdapter.invokeUiControlAfterTouchMiss(controlId, requestId)

    override fun setControlText(controlId: String, text: String, requestId: String) =
        pageAdapter.setUiControlText(controlId, text, requestId)

    override fun setControlSelected(controlId: String, selected: Boolean, requestId: String) =
        pageAdapter.setUiControlSelected(controlId, selected, requestId)

    override fun selectControlChoice(controlId: String, choiceIndex: Int, requestId: String) =
        pageAdapter.selectUiControlChoice(controlId, choiceIndex, requestId)

    override fun setControlSlider(controlId: String, value: Double, requestId: String) =
        pageAdapter.setUiControlSlider(controlId, value, requestId)

    override fun setControlExpanded(controlId: String, expanded: Boolean, requestId: String) =
        pageAdapter.setUiControlExpanded(controlId, expanded, requestId)

    override fun newConversation(requestId: String) = pageAdapter.startNewConversation(requestId)
    override fun stopGeneration(requestId: String) = pageAdapter.stopGeneration(requestId)
    override fun verifyPrivateStreamWatchdog(requestId: String) =
        pageAdapter.verifyPrivateStreamWatchdog(requestId)
    override fun regenerateResponse(requestId: String) = pageAdapter.regenerateResponse(requestId)
    override fun togglePrivateReadAloud(contextId: String, requestId: String) =
        pageAdapter.togglePrivateReadAloud(contextId, requestId)
    override fun startDictation(
        nativeDraft: String,
        expectedOfficialDraft: String,
        requestId: String,
    ) = startDictationAction(nativeDraft, expectedOfficialDraft, requestId)
    override fun cancelDictation(requestId: String) = pageAdapter.cancelDictation(requestId)
    override fun submitDictation(requestId: String) = pageAdapter.submitDictation(requestId)
    override fun removeAttachment(attachmentId: String, requestId: String) =
        pageAdapter.removeAttachment(attachmentId, requestId)

    override fun refreshControls(requestId: String) = pageAdapter.requestUiManifest(requestId)
    override fun revealProjectChoice(label: String, requestId: String) =
        pageAdapter.revealProjectChoice(label, requestId)
    override fun listConversations(requestId: String) = pageAdapter.listConversations(requestId)
    override fun requestComposerOptions(section: String, requestId: String) =
        requestComposerOptionsAction(section, requestId)

    override fun dismissComposerOptions(requestId: String) =
        dismissComposerOptionsAction(requestId)

    override fun selectComposerOption(section: String, optionId: String, requestId: String) {
        if (section == "model") pageAdapter.selectModelOption(optionId, requestId)
        else pageAdapter.selectComposerTool(optionId, requestId)
    }

    override fun requestFeatures(requestId: String) = pageAdapter.listFeatures(requestId)
    override fun dismissFeatures(requestId: String) = pageAdapter.dismissFeatures(requestId)
    override fun selectFeature(featureId: String, requestId: String) =
        pageAdapter.selectFeature(featureId, requestId)

    override fun openConversation(path: String, requestId: String) =
        pageAdapter.openConversation(path, requestId)
}
