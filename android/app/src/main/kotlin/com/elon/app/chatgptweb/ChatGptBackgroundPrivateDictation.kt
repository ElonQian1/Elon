package com.elon.app.chatgptweb

internal class ChatGptBackgroundPrivateDictation(
    private val adapter: () -> ChatGptWebPageAdapter?,
    private val ready: () -> Boolean,
    private val audioPermissionController: ChatGptWebAudioPermissionController,
    private val interactionRequested: () -> Unit,
) {
    fun start(
        nativeDraft: String,
        expectedOfficialDraft: String,
        onPermissionDenied: () -> Unit,
    ): Boolean {
        val activeAdapter = adapter() ?: return false
        if (!ready()) return false
        audioPermissionController.runWithMicrophone(
            action = {
                activeAdapter.startPrivateDictation(nativeDraft, expectedOfficialDraft)
                interactionRequested()
            },
            onPermissionDenied = onPermissionDenied,
        )
        return true
    }

    fun submit(): Boolean = dispatch(ChatGptWebPageAdapter::submitPrivateDictation)

    fun cancel(): Boolean = dispatch(ChatGptWebPageAdapter::cancelPrivateDictation)

    private fun dispatch(action: (ChatGptWebPageAdapter) -> Unit): Boolean {
        val activeAdapter = adapter() ?: return false
        action(activeAdapter)
        interactionRequested()
        return true
    }
}
