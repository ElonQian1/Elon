package com.elon.app.chatgptweb

import android.widget.ImageButton
import com.elon.app.R

internal class ChatGptNativeVoiceController(
    private val button: ImageButton,
    private val onToggle: () -> Unit,
) {
    private var bridgeReady = false
    private var snapshot: ChatGptWebSnapshot? = null
    private var manifest: ChatGptWebUiManifest? = null

    init {
        button.setOnClickListener { onToggle() }
        update()
    }

    fun render(value: ChatGptWebSnapshot) {
        snapshot = value
        update()
    }

    fun renderManifest(value: ChatGptWebUiManifest) {
        manifest = value
        update()
    }

    fun setBridgeState(state: ChatGptWebPageAdapter.State) {
        bridgeReady = state == ChatGptWebPageAdapter.State.READY
        update()
    }

    private fun update() {
        val value = snapshot
        val enabled = bridgeReady &&
            ChatGptDictationPolicy.isAvailable(value, manifest) &&
            value?.streaming != true
        button.isEnabled = enabled
        button.alpha = if (enabled) 1f else DISABLED_ALPHA
        button.isSelected = value?.dictationActive == true
        button.contentDescription = ChatGptNativeNavigationSelector.DICTATION
        button.tooltipText = button.context.getString(
            if (button.isSelected) R.string.chatgpt_native_dictation_stop else R.string.chatgpt_native_dictation_start,
        )
    }

    private companion object {
        const val DISABLED_ALPHA = 0.4f
    }
}
