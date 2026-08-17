package com.elon.app.chatgptweb

import java.net.URI

internal class ChatGptWebSessionContinuity(
    initialAuthenticated: Boolean = false,
    private val nowMs: () -> Long = System::currentTimeMillis,
    private val loginEvidenceGraceMs: Long = DEFAULT_LOGIN_EVIDENCE_GRACE_MS,
) {
    private var authenticatedObserved = initialAuthenticated
    private var loginEvidenceSinceMs: Long? = null
    private var pendingLoginSnapshot: ChatGptWebSnapshot? = null

    fun reconcile(snapshot: ChatGptWebSnapshot): ChatGptWebSnapshot {
        return reconcileWithDecision(snapshot).snapshot
    }

    fun reconcileWithDecision(snapshot: ChatGptWebSnapshot): Reconciliation {
        if (isExplicitAuthUrl(snapshot.url)) {
            return confirmLoggedOut(snapshot)
        }
        if (snapshot.loginRequired || snapshot.pageKind == "auth") {
            if (authenticatedObserved) {
                pendingLoginSnapshot = snapshot
                val now = nowMs()
                val since = loginEvidenceSinceMs ?: now.also { loginEvidenceSinceMs = it }
                val remainingMs = (loginEvidenceGraceMs - (now - since)).coerceAtLeast(0L)
                if (remainingMs > 0L) {
                    return Reconciliation(
                        snapshot = snapshot.copy(
                            authenticated = true,
                            loginRequired = false,
                            pageKind = snapshot.pageKind.takeUnless { it == "auth" } ?: "unknown",
                        ),
                        recheckAfterMs = remainingMs,
                    )
                }
            }
            return confirmLoggedOut(snapshot)
        }
        loginEvidenceSinceMs = null
        pendingLoginSnapshot = null
        if (snapshot.authenticated) {
            authenticatedObserved = true
            return Reconciliation(snapshot)
        }
        if (snapshot.composerReady) {
            val clearHistory = authenticatedObserved
            authenticatedObserved = false
            return Reconciliation(snapshot, clearConversationHistory = clearHistory)
        }
        if (authenticatedObserved) {
            return Reconciliation(snapshot.copy(authenticated = true))
        }
        return Reconciliation(snapshot)
    }

    fun clear() {
        authenticatedObserved = false
        loginEvidenceSinceMs = null
        pendingLoginSnapshot = null
    }

    fun confirmPendingLoginEvidence(): Reconciliation? {
        val snapshot = pendingLoginSnapshot ?: return null
        val since = loginEvidenceSinceMs ?: return null
        val remainingMs = (loginEvidenceGraceMs - (nowMs() - since)).coerceAtLeast(0L)
        return if (remainingMs > 0L) {
            Reconciliation(
                snapshot = snapshot.copy(
                    authenticated = true,
                    loginRequired = false,
                    pageKind = snapshot.pageKind.takeUnless { it == "auth" } ?: "unknown",
                ),
                recheckAfterMs = remainingMs,
            )
        } else {
            confirmLoggedOut(snapshot)
        }
    }

    private fun confirmLoggedOut(snapshot: ChatGptWebSnapshot): Reconciliation {
        val clearHistory = authenticatedObserved
        authenticatedObserved = false
        loginEvidenceSinceMs = null
        pendingLoginSnapshot = null
        return Reconciliation(
            snapshot = snapshot.copy(authenticated = false, loginRequired = true),
            clearConversationHistory = clearHistory,
        )
    }

    private fun isExplicitAuthUrl(url: String): Boolean {
        val path = runCatching { URI(url).path.orEmpty().lowercase() }.getOrDefault("")
        return path == "/auth" || path.startsWith("/auth/") ||
            path == "/cdn-cgi" || path.startsWith("/cdn-cgi/")
    }

    data class Reconciliation(
        val snapshot: ChatGptWebSnapshot,
        val clearConversationHistory: Boolean = false,
        val recheckAfterMs: Long? = null,
    )

    private companion object {
        const val DEFAULT_LOGIN_EVIDENCE_GRACE_MS = 2_000L
    }
}
