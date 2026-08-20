package com.elon.app

internal object SocialAiToolbarActionPolicy {
    fun showVoiceCall(
        directSocialAiChatActive: Boolean,
        webChatModeActive: Boolean,
    ): Boolean = directSocialAiChatActive && !webChatModeActive
}
