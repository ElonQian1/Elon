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
        "personalization",
        "pin",
        "plan",
        "read_aloud",
        "rename",
        "save_to_project",
        "share",
        "voice_mode",
    )
}
