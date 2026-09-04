package com.elon.app.chatgptweb

internal class ChatGptWebMcpTestCommandPort(
    private val onInvoke: (String) -> Unit = {},
    private val onSetControlText: (String, String) -> Unit = { _, _ -> },
    private val onSetControlSelected: (String, Boolean) -> Unit = { _, _ -> },
    private val onSelectControlChoice: (String, Int) -> Unit = { _, _ -> },
    private val onSetControlSlider: (String, Double) -> Unit = { _, _ -> },
    private val onSetControlExpanded: (String, Boolean) -> Unit = { _, _ -> },
    private val onStartDictation: () -> Unit = {},
    private val onStartDictationDrafts: (String, String) -> Unit = { _, _ -> },
    private val onCancelDictation: () -> Unit = {},
    private val onSubmitDictation: () -> Unit = {},
    private val onRemoveAttachment: (String) -> Unit = {},
    private val onRequestComposerOptions: (String) -> Unit = {},
    private val onSelectComposerOption: (String, String) -> Unit = { _, _ -> },
    private val onRequestFeatures: () -> Unit = {},
    private val onSelectFeature: (String) -> Unit = {},
    private val onOpenConversation: (String) -> Unit = {},
    private val onSetConversationPinned: (String, Boolean) -> Unit = { _, _ -> },
    private val onDispatch: (String, String) -> Unit = { _, _ -> },
    private val onSetDraft: (String, String) -> Unit = { _, _ -> },
) : ChatGptWebMcpCommandPort {
    override fun setDraft(value: String, expectedDraft: String, requestId: String) {
        onSetDraft(value, expectedDraft)
        dispatch("set_draft", requestId)
    }

    override fun sendInput(requestId: String) = dispatch("send_prompt", requestId)

    override fun invokeControl(controlId: String, requestId: String) {
        onInvoke(controlId)
        dispatch("invoke_ui_control", requestId)
    }

    override fun invokeControlAfterTouchMiss(controlId: String, requestId: String) {
        onInvoke(controlId)
        dispatch("invoke_ui_control_after_touch_miss", requestId)
    }

    override fun setControlText(controlId: String, text: String, requestId: String) {
        onSetControlText(controlId, text)
        dispatch("set_ui_control_text", requestId)
    }

    override fun setControlSelected(controlId: String, selected: Boolean, requestId: String) {
        onSetControlSelected(controlId, selected)
        dispatch("set_ui_control_selected", requestId)
    }

    override fun selectControlChoice(controlId: String, choiceIndex: Int, requestId: String) {
        onSelectControlChoice(controlId, choiceIndex)
        dispatch("select_ui_control_choice", requestId)
    }

    override fun setControlSlider(controlId: String, value: Double, requestId: String) {
        onSetControlSlider(controlId, value)
        dispatch("set_ui_control_slider", requestId)
    }

    override fun setControlExpanded(controlId: String, expanded: Boolean, requestId: String) {
        onSetControlExpanded(controlId, expanded)
        dispatch("set_ui_control_expanded", requestId)
    }

    override fun newConversation(requestId: String) = dispatch("new_conversation", requestId)
    override fun stopGeneration(requestId: String) = dispatch("stop_generation", requestId)
    override fun verifyPrivateStreamWatchdog(requestId: String) =
        dispatch("verify_private_stream_watchdog", requestId)
    override fun regenerateResponse(requestId: String) = dispatch("regenerate_response", requestId)

    override fun startDictation(
        nativeDraft: String,
        expectedOfficialDraft: String,
        requestId: String,
    ) {
        onStartDictation()
        onStartDictationDrafts(nativeDraft, expectedOfficialDraft)
        dispatch("start_dictation", requestId)
    }

    override fun cancelDictation(requestId: String) {
        onCancelDictation()
        dispatch("cancel_dictation", requestId)
    }

    override fun submitDictation(requestId: String) {
        onSubmitDictation()
        dispatch("submit_dictation", requestId)
    }

    override fun removeAttachment(attachmentId: String, requestId: String) {
        onRemoveAttachment(attachmentId)
        dispatch("remove_attachment", requestId)
    }

    override fun refreshControls(requestId: String) = dispatch("snapshot_ui_manifest", requestId)
    override fun revealProjectChoice(label: String, requestId: String) =
        dispatch("reveal_project_choice", requestId)
    override fun listConversations(requestId: String) = dispatch("list_conversations", requestId)

    override fun requestComposerOptions(section: String, requestId: String) {
        onRequestComposerOptions(section)
        dispatch(if (section == "model") "list_model_options" else "list_composer_tools", requestId)
    }

    override fun dismissComposerOptions(requestId: String) =
        dispatch("dismiss_composer_menu", requestId)

    override fun selectComposerOption(section: String, optionId: String, requestId: String) {
        onSelectComposerOption(section, optionId)
        dispatch(if (section == "model") "select_model_option" else "select_composer_tool", requestId)
    }

    override fun requestFeatures(requestId: String) {
        onRequestFeatures()
        dispatch("list_navigation", requestId)
    }

    override fun dismissFeatures(requestId: String) = dispatch("dismiss_navigation", requestId)

    override fun selectFeature(featureId: String, requestId: String) {
        onSelectFeature(featureId)
        dispatch("select_navigation", requestId)
    }

    override fun openConversation(path: String, requestId: String) {
        onOpenConversation(path)
        dispatch("open_conversation", requestId)
    }

    override fun setConversationPinned(path: String, pinned: Boolean, requestId: String) {
        onSetConversationPinned(path, pinned)
        dispatch("set_conversation_pinned", requestId)
    }

    private fun dispatch(action: String, requestId: String) = onDispatch(action, requestId)
}
