package com.elon.app.chatgptweb

import android.content.Context
import com.elon.app.BuildConfig

/** Coordinates the research-only native peer and the page-local bootstrap relay. */
internal class ChatGptWebNativeVoiceResearchController(
    context: Context,
    private val relay: ChatGptWebPrivateVoiceRelayGateway,
    schedule: (Runnable, Long) -> Unit,
    private val onState: (ChatGptWebNativeVoiceState) -> Unit,
) {
    private var starting = false
    private val peer = ChatGptWebNativeVoicePeer(
        context = context,
        relay = relay::exchange,
        schedule = schedule,
        onState = { state ->
            starting = false
            onState(state)
        },
    )

    fun start(): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED || starting) return false
        starting = true
        return relay.readBootstrap { bootstrap ->
            when (bootstrap) {
                is ChatGptWebPrivateVoiceBootstrap.Ready -> {
                    if (!peer.start(bootstrap.dataChannel)) {
                        starting = false
                    }
                }
                is ChatGptWebPrivateVoiceBootstrap.Unavailable -> {
                    starting = false
                    onState(
                        ChatGptWebNativeVoiceState(
                            phase = ChatGptWebNativeVoicePhase.FAILED,
                            code = "bootstrap_${bootstrap.code}",
                        ),
                    )
                }
            }
        }.also { accepted ->
            if (!accepted) starting = false
        }
    }

    fun setMuted(muted: Boolean): Boolean = peer.setMuted(muted)

    fun close() {
        starting = false
        relay.cancel()
        peer.close()
    }
}
