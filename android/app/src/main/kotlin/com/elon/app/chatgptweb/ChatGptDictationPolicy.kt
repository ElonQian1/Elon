package com.elon.app.chatgptweb

internal object ChatGptDictationPolicy {
    const val SEMANTIC = ChatGptWebUiSemantics.DICTATION

    fun resolve(manifest: ChatGptWebUiManifest?): ChatGptWebUiControl? =
        manifest?.controls?.firstOrNull { control ->
            control.semantic == SEMANTIC && control.enabled
        }

    fun isAvailable(snapshot: ChatGptWebSnapshot?, manifest: ChatGptWebUiManifest?): Boolean =
        snapshot?.dictationActive == true ||
            snapshot?.capabilities?.supports(ChatGptWebCapabilityId.DICTATION) == true ||
            resolve(manifest) != null

    /**
     * A missing DOM control is not proof that dictation is unsupported. The composer may be
     * showing a send button for a stale draft, so an explicit user action may reconcile it first.
     */
    fun canAttemptStart(snapshot: ChatGptWebSnapshot?, manifest: ChatGptWebUiManifest?): Boolean =
        isAvailable(snapshot, manifest) || (
            snapshot?.authenticated == true &&
                snapshot.composerReady &&
                !snapshot.loginRequired &&
                !snapshot.streaming
            )
}
