package com.elon.app.chatgptweb

internal object ChatGptWebBridgeReadinessPolicy {
    fun canRestoreFromManifest(
        snapshot: ChatGptWebSnapshot?,
        manifest: ChatGptWebUiManifest,
    ): Boolean = snapshot?.authenticated == true &&
        snapshot.loginRequired.not() &&
        manifest.pageKind != "auth"
}
