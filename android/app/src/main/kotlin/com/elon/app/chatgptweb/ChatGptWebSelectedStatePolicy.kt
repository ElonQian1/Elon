package com.elon.app.chatgptweb

internal object ChatGptWebSelectedStatePolicy {
    fun rejection(control: ChatGptWebUiControl, selected: Boolean): String? {
        if (!control.supportsSelectedState) return "control_state_not_settable"
        if (selected) return null
        return when (control.role) {
            "radio", "menuitemradio" -> "radio_cannot_be_cleared"
            "tab" -> "tab_cannot_be_cleared"
            else -> null
        }
    }
}
