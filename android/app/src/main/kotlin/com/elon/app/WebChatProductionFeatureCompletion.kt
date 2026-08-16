package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebNavigationPolicy
import com.elon.app.chatgptweb.ChatGptWebProductCapabilityCatalog
import java.net.URI

internal enum class WebChatProductionFeatureCompletionDecision {
    WAITING,
    OPEN_OFFICIAL,
    FAILED,
}

internal object WebChatProductionFeatureCompletionPolicy {
    fun requiresOfficialCompletion(@Suppress("UNUSED_PARAMETER") kind: String): Boolean = true

    fun evaluate(
        feature: WebChatProductionFeature,
        requestId: String,
        state: WebChatConsumerState,
    ): WebChatProductionFeatureCompletionDecision {
        return when (commandStatus(state, requestId)) {
            WebChatConsumerCommandStatus.FAILED -> WebChatProductionFeatureCompletionDecision.FAILED
            WebChatConsumerCommandStatus.SUCCEEDED -> if (pageSettled(feature, state)) {
                WebChatProductionFeatureCompletionDecision.OPEN_OFFICIAL
            } else {
                WebChatProductionFeatureCompletionDecision.WAITING
            }
            else -> WebChatProductionFeatureCompletionDecision.WAITING
        }
    }

    fun pageSettled(feature: WebChatProductionFeature, state: WebChatConsumerState): Boolean {
        val expectedKind = feature.kind.trim().lowercase()
        val url = state.pageUrl.trim()
        if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) return false

        val pageKind = state.pageKind.trim().lowercase()
        if (expectedKind.isNotBlank() && pageKind == expectedKind) return true

        val path = runCatching { URI(url).path.orEmpty() }.getOrDefault("")
        val prefixes = ChatGptWebProductCapabilityCatalog.PAGE_FEATURES
            .firstOrNull { it.protocolKind == expectedKind }
            ?.restorablePrefixes
            .orEmpty()
        return prefixes.any { prefix -> path == prefix || path.startsWith("$prefix/") }
    }

    private fun commandStatus(
        state: WebChatConsumerState,
        requestId: String,
    ): WebChatConsumerCommandStatus? = state.commandRequests
        .firstOrNull { it.id == requestId }
        ?.status
}
