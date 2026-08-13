package com.elon.app

internal enum class SocialAiInteractionMode(val wireValue: String) {
    WORK("work"),
    CHAT("chat");

    companion object {
        fun fromWireValue(value: String?): SocialAiInteractionMode =
            parse(value) ?: CHAT

        fun parse(value: String?): SocialAiInteractionMode? =
            entries.firstOrNull { it.wireValue == value }
    }
}
