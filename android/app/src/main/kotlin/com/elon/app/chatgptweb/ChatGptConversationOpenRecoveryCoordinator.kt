package com.elon.app.chatgptweb

import android.webkit.WebView
import com.elon.app.DebugTraceStore

internal class ChatGptConversationOpenRecoveryCoordinator(
    private val currentUrl: () -> String?,
    private val navigationPending: () -> Boolean,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancelTask: (Runnable) -> Unit,
    private val onRecovery: (Boolean) -> Unit,
    private val loadUrl: (String) -> Unit,
) {
    private var recoveryTask: Runnable? = null

    fun schedule(path: String) {
        cancel()
        val normalized = ChatGptWebConversationPath.normalize(path) ?: return
        val task = Runnable {
            recoveryTask = null
            if (!navigationPending()) return@Runnable
            onRecovery(ChatGptWebConversationPath.fromUrl(currentUrl()) == normalized)
            loadUrl(ChatGptWebNavigationPolicy.START_URL.removeSuffix("/") + normalized)
        }
        recoveryTask = task
        schedule(task, RECOVERY_DELAY_MS)
    }

    fun cancel() {
        recoveryTask?.let(cancelTask)
        recoveryTask = null
    }

    private companion object {
        const val RECOVERY_DELAY_MS = 2_000L
    }
}

internal fun createChatGptConversationOpenRecoveryCoordinator(
    webView: () -> WebView?,
    navigationPending: () -> Boolean,
    schedule: (Runnable, Long) -> Unit,
    cancelTask: (Runnable) -> Unit,
    interactionRequested: () -> Unit,
): ChatGptConversationOpenRecoveryCoordinator =
    ChatGptConversationOpenRecoveryCoordinator(
        currentUrl = { webView()?.url },
        navigationPending = navigationPending,
        schedule = schedule,
        cancelTask = cancelTask,
        onRecovery = { actualPathMatches ->
            DebugTraceStore.record(
                "web_chat_conversation_navigation_fallback",
                mapOf("actual_path_matches_target" to actualPathMatches),
            )
        },
        loadUrl = { url ->
            interactionRequested()
            webView()?.loadUrl(url)
        },
    )
