package com.elon.app

internal interface WebChatConsumerSlider {
    val min: Double
    val max: Double
    val step: Double
    val value: Double
    val stepCount: Int
}

internal interface WebChatConsumerControl {
    val id: String
    val semantic: String
    val label: String
    val region: String
    val role: String
    val enabled: Boolean
    val selected: Boolean
    val inputKind: String?
    val writable: Boolean
    val stateSettable: Boolean
    val choiceLabels: List<String>
    val selectedChoiceIndex: Int?
    val slider: WebChatConsumerSlider?
    val expanded: Boolean?
    val expandable: Boolean
    val contextId: String?
    val inViewport: Boolean
    val webXRatio: Double?
    val webYRatio: Double?

    val supportsTextEntry: Boolean
        get() = role == "textbox" && writable && inputKind != "password"

    val supportsSelectedState: Boolean
        get() = stateSettable && enabled && (
            role in setOf(
                "checkbox", "radio", "menuitemcheckbox", "menuitemradio", "switch", "tab",
            ) || semantic == "temporary_chat"
            )

    val supportsChoiceSelection: Boolean
        get() = enabled && role == "combobox" && inputKind == "select" && choiceLabels.isNotEmpty()

    val supportsSliderValue: Boolean
        get() = enabled && role == "slider" && inputKind == "range" && slider != null

    val supportsExpandedState: Boolean
        get() = enabled && expandable && expanded != null
}
