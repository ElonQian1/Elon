package com.elon.app.chatgptweb

internal data class ChatGptWebUiManifest(
    val version: Int,
    val pageKind: String,
    val title: String,
    val compatibility: String,
    val controls: List<ChatGptWebUiControl>,
    val discoveredControlCount: Int = controls.size,
    val controlsTruncated: Boolean = false,
)

internal data class ChatGptWebUiControl(
    val id: String,
    val semantic: String,
    val label: String,
    val region: String,
    val role: String,
    val enabled: Boolean,
    val selected: Boolean,
    val inputKind: String? = null,
    val writable: Boolean = false,
    val contextId: String? = null,
    val inViewport: Boolean = true,
    val webXRatio: Double? = null,
    val webYRatio: Double? = null,
) {
    val supportsTextEntry: Boolean
        get() = role == "textbox" && writable && inputKind != "password"

    val accessibilityLabel: String
        get() = "chatgpt-control:$id:$label"
}

internal object ChatGptWebUiRegion {
    const val HEADER = "header"
    const val SUGGESTIONS = "suggestions"
    const val COMPOSER = "composer"
    const val OVERLAY = "overlay"
    const val MESSAGE = "message"
    const val CONTENT = "content"
}
