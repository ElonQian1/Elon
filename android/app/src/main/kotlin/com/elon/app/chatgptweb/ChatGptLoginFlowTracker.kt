package com.elon.app.chatgptweb

import java.net.URI
import java.util.Locale

internal enum class ChatGptLoginStage {
    READY,
    OPENING_OFFICIAL_AUTH,
    WAITING_FOR_USER,
    COMPLETING,
    AUTHENTICATED,
    FAILED,
}

internal data class ChatGptLoginFlowSnapshot(
    val stage: ChatGptLoginStage,
    val elapsedMillis: Long,
    val isRunning: Boolean,
)

internal class ChatGptLoginFlowTracker(
    private val elapsedRealtime: () -> Long,
) {
    private var stage = ChatGptLoginStage.READY
    private var startedAt: Long? = null
    private var finishedAt: Long? = null

    fun begin(): ChatGptLoginFlowSnapshot {
        startedAt = elapsedRealtime()
        finishedAt = null
        stage = ChatGptLoginStage.OPENING_OFFICIAL_AUTH
        return snapshot()
    }

    fun onPageStarted(rawUrl: String): ChatGptLoginFlowSnapshot {
        if (!isRunning()) return snapshot()
        stage = when (classify(rawUrl)) {
            PageKind.LOGIN, PageKind.CHALLENGE, PageKind.IDENTITY -> ChatGptLoginStage.WAITING_FOR_USER
            PageKind.CHATGPT -> ChatGptLoginStage.COMPLETING
            PageKind.OTHER -> ChatGptLoginStage.OPENING_OFFICIAL_AUTH
        }
        return snapshot()
    }

    fun onPageReady(rawUrl: String): ChatGptLoginFlowSnapshot {
        if (!isRunning()) return snapshot()
        stage = when (classify(rawUrl)) {
            PageKind.LOGIN, PageKind.CHALLENGE, PageKind.IDENTITY -> ChatGptLoginStage.WAITING_FOR_USER
            PageKind.CHATGPT -> ChatGptLoginStage.COMPLETING
            PageKind.OTHER -> ChatGptLoginStage.OPENING_OFFICIAL_AUTH
        }
        return snapshot()
    }

    fun markAuthenticated(): ChatGptLoginFlowSnapshot {
        stage = ChatGptLoginStage.AUTHENTICATED
        if (startedAt != null) finishedAt = elapsedRealtime()
        return snapshot()
    }

    fun fail(): ChatGptLoginFlowSnapshot {
        if (!isRunning()) return snapshot()
        stage = ChatGptLoginStage.FAILED
        finishedAt = elapsedRealtime()
        return snapshot()
    }

    fun reset(): ChatGptLoginFlowSnapshot {
        stage = ChatGptLoginStage.READY
        startedAt = null
        finishedAt = null
        return snapshot()
    }

    fun snapshot(): ChatGptLoginFlowSnapshot {
        val start = startedAt
        val elapsed = if (start == null) 0L else ((finishedAt ?: elapsedRealtime()) - start).coerceAtLeast(0L)
        return ChatGptLoginFlowSnapshot(stage, elapsed, isRunning())
    }

    private fun isRunning(): Boolean = startedAt != null && finishedAt == null

    private fun classify(rawUrl: String): PageKind {
        val uri = runCatching { URI(rawUrl) }.getOrNull() ?: return PageKind.OTHER
        val host = uri.host?.lowercase(Locale.ROOT) ?: return PageKind.OTHER
        val path = uri.path.orEmpty().lowercase(Locale.ROOT)
        if (host == "chatgpt.com" || host.endsWith(".chatgpt.com")) {
            return when {
                path.startsWith("/auth") -> PageKind.LOGIN
                path.startsWith("/cdn-cgi") -> PageKind.CHALLENGE
                else -> PageKind.CHATGPT
            }
        }
        if (
            host.endsWith(".openai.com") ||
            ChatGptWebNavigationPolicy.isIdentityHost(host)
        ) {
            return PageKind.IDENTITY
        }
        return PageKind.OTHER
    }

    private enum class PageKind {
        LOGIN,
        CHALLENGE,
        IDENTITY,
        CHATGPT,
        OTHER,
    }
}
