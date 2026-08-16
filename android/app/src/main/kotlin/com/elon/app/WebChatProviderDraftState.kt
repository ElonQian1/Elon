package com.elon.app

internal class WebChatProviderDraftState {
    private val drafts = mutableMapOf<WebChatProviderId, String>()

    fun remember(providerId: WebChatProviderId, value: CharSequence?) {
        val draft = value?.toString().orEmpty().take(MAX_DRAFT_LENGTH)
        if (draft.isBlank()) drafts.remove(providerId) else drafts[providerId] = draft
    }

    fun restore(providerId: WebChatProviderId): String = drafts[providerId].orEmpty()

    private companion object {
        const val MAX_DRAFT_LENGTH = 12_000
    }
}
