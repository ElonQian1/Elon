package com.elon.app.chatgptweb

internal object ChatGptWebAccessPolicy {
    fun requiresLogin(snapshot: ChatGptWebSnapshot): Boolean =
        snapshot.loginRequired || snapshot.accessReason == "login_required" || snapshot.pageKind == "auth"

    fun canChat(snapshot: ChatGptWebSnapshot): Boolean =
        snapshot.composerReady && !requiresLogin(snapshot) && snapshot.accessReason != "rate_limited"
}
