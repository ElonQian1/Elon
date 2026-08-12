package com.elon.app.chatgptweb

import android.view.View
import android.widget.ImageButton
import com.elon.app.R

internal class ChatGptNativeRealtimeVoiceController(
    private val button: ImageButton,
    private val onStart: (String) -> Unit,
) {
    private var bridgeReady = false
    private var control: ChatGptWebUiControl? = null

    init {
        button.contentDescription = ChatGptNativeNavigationSelector.REALTIME_VOICE
        button.tooltipText = button.context.getString(R.string.chatgpt_native_realtime_voice)
        button.setOnClickListener { control?.id?.let(onStart) }
        update()
    }

    fun render(manifest: ChatGptWebUiManifest) {
        control = ChatGptRealtimeVoicePolicy.resolve(manifest)
        update()
    }

    fun setBridgeState(state: ChatGptWebPageAdapter.State) {
        bridgeReady = state == ChatGptWebPageAdapter.State.READY
        update()
    }

    fun dispose() {
        button.setOnClickListener(null)
    }

    private fun update() {
        val available = bridgeReady && control != null
        button.visibility = if (available) View.VISIBLE else View.GONE
        button.isEnabled = available
    }
}
