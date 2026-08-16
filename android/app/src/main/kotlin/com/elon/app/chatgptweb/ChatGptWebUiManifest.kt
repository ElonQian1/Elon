package com.elon.app.chatgptweb

import com.elon.app.WebChatConsumerControl
import com.elon.app.WebChatConsumerSlider

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
    override val id: String,
    override val semantic: String,
    override val label: String,
    override val region: String,
    override val role: String,
    override val enabled: Boolean,
    override val selected: Boolean,
    override val inputKind: String? = null,
    override val writable: Boolean = false,
    override val stateSettable: Boolean = false,
    override val choiceLabels: List<String> = emptyList(),
    override val selectedChoiceIndex: Int? = null,
    override val slider: ChatGptWebSlider? = null,
    override val expanded: Boolean? = null,
    override val expandable: Boolean = false,
    override val contextId: String? = null,
    override val inViewport: Boolean = true,
    override val webXRatio: Double? = null,
    override val webYRatio: Double? = null,
) : WebChatConsumerControl {

    val accessibilityLabel: String
        get() = "chatgpt-control:$id:$label"
}

internal data class ChatGptWebSlider(
    override val min: Double,
    override val max: Double,
    override val step: Double,
    override val value: Double,
) : WebChatConsumerSlider {
    override val stepCount: Int
        get() = kotlin.math.round((max - min) / step).toInt()
}

internal object ChatGptWebUiRegion {
    const val HEADER = "header"
    const val SUGGESTIONS = "suggestions"
    const val COMPOSER = "composer"
    const val OVERLAY = "overlay"
    const val MESSAGE = "message"
    const val CONTENT = "content"
}
