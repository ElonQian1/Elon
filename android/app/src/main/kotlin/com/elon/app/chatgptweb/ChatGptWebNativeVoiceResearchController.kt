package com.elon.app.chatgptweb

import android.content.Context
import com.elon.app.BuildConfig

/** Coordinates the research-only native peer and atomic page-local bootstrap relay. */
internal class ChatGptWebNativeVoiceResearchController(
    context: Context,
    private val relay: ChatGptWebPrivateVoiceRelayGateway,
    schedule: (Runnable, Long) -> Unit,
    private val startOfficialVoice: () -> Boolean,
    private val requestOfficialFallback: () -> Boolean,
    private val onTranscript: (ChatGptWebNativeVoiceTranscriptEvent) -> Unit,
    private val onState: (ChatGptWebNativeVoiceState) -> Unit,
) {
    private var starting = false
    private var generation = 0L
    private var officialMediaSuspended = false
    private var closing = false
    private val peer = ChatGptWebNativeVoicePeer(
        context = context,
        relay = relay::exchange,
        schedule = schedule,
        onRelayArmed = startOfficialVoice,
        onTranscript = onTranscript,
        onState = ::acceptPeerState,
    )

    fun start(): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED || starting) return false
        generation += 1
        val token = generation
        closing = false
        officialMediaSuspended = false
        starting = true
        return relay.readBootstrap { bootstrap ->
            if (token != generation) return@readBootstrap
            when (bootstrap) {
                is ChatGptWebPrivateVoiceBootstrap.Ready -> {
                    if (!peer.start(bootstrap.dataChannel)) {
                        starting = false
                        emitFailure("native_start_rejected")
                    }
                }
                is ChatGptWebPrivateVoiceBootstrap.Unavailable -> {
                    starting = false
                    emitFailure("bootstrap_${bootstrap.code}")
                }
            }
        }.also { accepted ->
            if (!accepted) starting = false
        }
    }

    fun setMuted(muted: Boolean): Boolean = peer.setMuted(muted)

    fun close() {
        generation += 1
        closing = true
        starting = false
        relay.cancel()
        peer.close()
        officialMediaSuspended = false
        relay.resetTakeover()
        closing = false
    }

    private fun acceptPeerState(state: ChatGptWebNativeVoiceState) {
        if (
            state.phase !in setOf(
                ChatGptWebNativeVoicePhase.BOOTSTRAPPING,
                ChatGptWebNativeVoicePhase.CREATING_OFFER,
                ChatGptWebNativeVoicePhase.RELAYING,
            )
        ) {
            starting = false
        }
        if (state.phase == ChatGptWebNativeVoicePhase.APPLYING_ANSWER) {
            officialMediaSuspended = true
        }
        if (closing) {
            onState(decorate(state))
            return
        }
        if (state.phase == ChatGptWebNativeVoicePhase.FAILED) {
            if (officialMediaSuspended) {
                recoverOfficialAfterTakeover(state)
            } else if (state.code?.startsWith("relay_") == true) {
                // The page relay has already restored and sent the untouched
                // official request after rejecting the native response.
                onState(
                    decorate(state).copy(
                        phase = ChatGptWebNativeVoicePhase.OFFICIAL_FALLBACK,
                        code = "official_fallback_active",
                    ),
                )
            } else {
                onState(decorate(state))
            }
            return
        }
        onState(decorate(state))
    }

    private fun recoverOfficialAfterTakeover(state: ChatGptWebNativeVoiceState) {
        val token = generation
        onState(decorate(state))
        relay.resetTakeover { result ->
            if (token != generation) return@resetTakeover
            officialMediaSuspended = false
            val accepted = result is ChatGptWebPrivateVoiceMediaControl.Applied &&
                requestOfficialFallback()
            if (accepted) {
                onState(
                    state.copy(
                        phase = ChatGptWebNativeVoicePhase.OFFICIAL_FALLBACK,
                        officialMediaSuspended = false,
                        code = "official_fallback_reloading",
                    ),
                )
            } else {
                emitFailure("official_fallback_unavailable")
            }
        }
    }

    private fun decorate(state: ChatGptWebNativeVoiceState): ChatGptWebNativeVoiceState =
        state.copy(
            officialMediaSuspended = officialMediaSuspended,
            officialPeerReleased = false,
        )

    private fun emitFailure(code: String) {
        onState(
            ChatGptWebNativeVoiceState(
                phase = ChatGptWebNativeVoicePhase.FAILED,
                officialMediaSuspended = officialMediaSuspended,
                code = code,
            ),
        )
    }
}
