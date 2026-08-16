package com.elon.app

internal class SocialAiComposerDraftCoordinator(
    private val providerDrafts: WebChatProviderDraftState,
    private val readText: () -> CharSequence?,
    private val writeText: (String) -> Unit,
    private val onProviderDraftChanged: () -> Unit,
) {
    private var owner: DraftOwner? = null
    private var workModeDraft = ""
    private var workModeDraftInitialized = false
    private var applyingDraft = false

    fun activateWorkMode() = switchTo(DraftOwner.WORK)

    fun activateProvider(providerId: WebChatProviderId) = switchTo(DraftOwner.forProvider(providerId))

    fun release() {
        rememberCurrent()
        owner = null
    }

    fun onTextChanged(value: CharSequence?) {
        if (applyingDraft) return
        remember(owner, value)
    }

    fun rememberCurrent() {
        remember(owner, readText())
    }

    private fun switchTo(target: DraftOwner) {
        if (owner == target) return
        val previousOwner = owner
        rememberCurrent()
        if (target == DraftOwner.WORK && !workModeDraftInitialized) {
            workModeDraft = if (previousOwner == null) normalize(readText()) else ""
            workModeDraftInitialized = true
        }
        owner = target
        val draft = when (target) {
            DraftOwner.WORK -> workModeDraft
            DraftOwner.CHATGPT_WEB,
            DraftOwner.GOOGLE_WEB,
            -> providerDrafts.restore(requireNotNull(target.providerId))
        }
        if (readText()?.toString().orEmpty() == draft) return
        applyingDraft = true
        try {
            writeText(draft)
        } finally {
            applyingDraft = false
        }
    }

    private fun remember(target: DraftOwner?, value: CharSequence?) {
        when (target) {
            DraftOwner.WORK -> {
                workModeDraft = normalize(value)
                workModeDraftInitialized = true
            }
            DraftOwner.CHATGPT_WEB,
            DraftOwner.GOOGLE_WEB,
            -> {
                if (providerDrafts.remember(requireNotNull(target.providerId), value)) {
                    onProviderDraftChanged()
                }
            }
            null -> Unit
        }
    }

    private fun normalize(value: CharSequence?): String =
        value?.toString().orEmpty().take(WebChatProviderDraftState.MAX_DRAFT_LENGTH)

    private enum class DraftOwner(val providerId: WebChatProviderId?) {
        WORK(null),
        CHATGPT_WEB(WebChatProviderId.CHATGPT_WEB),
        GOOGLE_WEB(WebChatProviderId.GOOGLE_WEB);

        companion object {
            fun forProvider(providerId: WebChatProviderId): DraftOwner = when (providerId) {
                WebChatProviderId.CHATGPT_WEB -> CHATGPT_WEB
                WebChatProviderId.GOOGLE_WEB -> GOOGLE_WEB
            }
        }
    }
}
