package com.elon.app.chatgptweb

internal object ChatGptWebControlInvocationPolicy {
    enum class Risk(val wireName: String) {
        STANDARD("standard"),
        USER_CONFIRMATION("user_confirmation"),
    }

    fun risk(control: ChatGptWebUiControl): Risk = risk(control.semantic)

    fun risk(semantic: String): Risk = if (
        semantic.trim().lowercase() in USER_CONFIRMED_SEMANTICS
    ) {
        Risk.USER_CONFIRMATION
    } else {
        Risk.STANDARD
    }

    fun rejection(control: ChatGptWebUiControl, userConfirmed: Boolean): String? =
        if (risk(control) == Risk.USER_CONFIRMATION && !userConfirmed) {
            "user_confirmation_required"
        } else {
            null
        }

    fun afterTouchMissRejection(
        control: ChatGptWebUiControl,
        currentUrl: String?,
        controls: List<ChatGptWebUiControl>,
    ): String? {
        val currentIdentity = ChatGptWebConversationPath.identity(
            ChatGptWebConversationPath.fromUrl(currentUrl),
        )
        return when {
            control.semantic != "conversation_options" || control.region != ChatGptWebUiRegion.HEADER ->
                "touch_miss_fallback_not_supported"
            currentIdentity == null || control.contextId != currentIdentity ->
                "touch_miss_fallback_context_changed"
            controls.any { it.enabled && it.inViewport && it.region == ChatGptWebUiRegion.OVERLAY } ->
                "touch_miss_fallback_overlay_present"
            else -> null
        }
    }

    private val USER_CONFIRMED_SEMANTICS = setOf(
        "action",
        "archive",
        "branch",
        "confirm",
        "delete",
        "dictation",
        "download_app",
        "edit",
        "feedback",
        "logout",
        ChatGptWebUiSemantics.OPEN_LINK,
        "personalization",
        "pin",
        "plan",
        "rename",
        "save_to_project",
        "share",
        "voice_mode",
    )
}
