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
}
