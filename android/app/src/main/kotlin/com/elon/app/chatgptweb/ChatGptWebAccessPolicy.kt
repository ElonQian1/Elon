package com.elon.app.chatgptweb

internal object ChatGptWebAccessPolicy {
    fun requiresLogin(snapshot: ChatGptWebSnapshot): Boolean =
        snapshot.loginRequired || snapshot.accessReason == "login_required" || snapshot.pageKind == "auth"

    fun canChat(snapshot: ChatGptWebSnapshot): Boolean =
        snapshot.composerReady && !requiresLogin(snapshot) && snapshot.accessReason != "rate_limited"

    fun canNavigate(snapshot: ChatGptWebSnapshot?, adapterCurrent: Boolean): Boolean =
        adapterCurrent && snapshot != null && !requiresLogin(snapshot) &&
            ChatGptWebNavigationPolicy.supportsEnhancedMode(snapshot.url) &&
            !ChatGptWebNavigationPolicy.isAuthenticationPage(snapshot.url)

    fun canReadDirectory(snapshot: ChatGptWebSnapshot?, adapterCurrent: Boolean): Boolean =
        canNavigate(snapshot, adapterCurrent) && snapshot != null &&
            (snapshot.authenticated || canChat(snapshot))
}
