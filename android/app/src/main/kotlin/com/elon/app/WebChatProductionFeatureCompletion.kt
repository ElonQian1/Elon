package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebNavigationPolicy
import com.elon.app.chatgptweb.ChatGptWebProductCapabilityCatalog
import org.json.JSONObject
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
        uiState: JSONObject,
    ): WebChatProductionFeatureCompletionDecision {
        return when (commandState(uiState, requestId)) {
            CommandState.FAILED -> WebChatProductionFeatureCompletionDecision.FAILED
            CommandState.SUCCEEDED -> if (pageSettled(feature, uiState)) {
                WebChatProductionFeatureCompletionDecision.OPEN_OFFICIAL
            } else {
                WebChatProductionFeatureCompletionDecision.WAITING
            }
            CommandState.MISSING,
            CommandState.PENDING -> WebChatProductionFeatureCompletionDecision.WAITING
        }
    }

    fun pageSettled(feature: WebChatProductionFeature, uiState: JSONObject): Boolean {
        val expectedKind = feature.kind.trim().lowercase()
        val url = uiState.optJSONObject("conversation")
            ?.optString("url")
            ?.trim()
            .orEmpty()
        if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(url)) return false

        val pageKind = uiState.optString("page_kind").trim().lowercase()
        if (expectedKind.isNotBlank() && pageKind == expectedKind) return true

        val path = runCatching { URI(url).path.orEmpty() }.getOrDefault("")
        val prefixes = ChatGptWebProductCapabilityCatalog.PAGE_FEATURES
            .firstOrNull { it.protocolKind == expectedKind }
            ?.restorablePrefixes
            .orEmpty()
        return prefixes.any { prefix -> path == prefix || path.startsWith("$prefix/") }
    }

    fun requestId(response: JSONObject): String? {
        val receipt = response.optJSONObject("command_receipt") ?: return null
        if (!receipt.has("request_id") || receipt.isNull("request_id")) return null
        return receipt.optString("request_id")
            .trim()
            .takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun commandState(uiState: JSONObject, requestId: String): CommandState {
        val requests = uiState.optJSONArray("command_requests") ?: return CommandState.MISSING
        for (index in 0 until requests.length()) {
            val request = requests.optJSONObject(index) ?: continue
            if (request.optString("request_id") != requestId) continue
            return when (request.optString("status").trim().lowercase()) {
                "succeeded" -> CommandState.SUCCEEDED
                "failed" -> CommandState.FAILED
                else -> CommandState.PENDING
            }
        }
        return CommandState.MISSING
    }

    private enum class CommandState {
        MISSING,
        PENDING,
        SUCCEEDED,
        FAILED,
    }
}
