package com.elon.app.chatgptweb

import java.net.URI

internal class ChatGptWebSessionContinuity {
    private var authenticatedObserved = false

    fun reconcile(snapshot: ChatGptWebSnapshot): ChatGptWebSnapshot {
        if (snapshot.loginRequired || snapshot.pageKind == "auth" || isExplicitAuthUrl(snapshot.url)) {
            authenticatedObserved = false
            return snapshot.copy(authenticated = false, loginRequired = true)
        }
        if (snapshot.authenticated) {
            authenticatedObserved = true
            return snapshot
        }
        if (snapshot.composerReady) {
            authenticatedObserved = false
            return snapshot
        }
        if (authenticatedObserved) {
            return snapshot.copy(authenticated = true)
        }
        return snapshot
    }

    fun clear() {
        authenticatedObserved = false
    }

    private fun isExplicitAuthUrl(url: String): Boolean {
        val path = runCatching { URI(url).path.orEmpty().lowercase() }.getOrDefault("")
        return path == "/auth" || path.startsWith("/auth/") ||
            path == "/cdn-cgi" || path.startsWith("/cdn-cgi/")
    }
}
