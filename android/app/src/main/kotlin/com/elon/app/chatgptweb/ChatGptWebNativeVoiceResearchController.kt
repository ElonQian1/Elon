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
    private var generation = 0L
    private var officialMediaSuspended = false
    private var officialPeerReleased = false
    private var releaseInFlight = false
    private var closing = false
    private val peer = ChatGptWebNativeVoicePeer(
        context = context,
        relay = relay::exchange,
        schedule = schedule,
        onState = ::acceptPeerState,
    )

    fun start(): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED || starting) return false
        generation += 1
        val token = generation
        closing = false
        officialPeerReleased = false
        releaseInFlight = false
        starting = true
        return relay.readBootstrap { bootstrap ->
            if (token != generation) return@readBootstrap
            when (bootstrap) {
                is ChatGptWebPrivateVoiceBootstrap.Ready -> {
                    suspendOfficialMediaThenStart(token, bootstrap.dataChannel)
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
        generation += 1
        closing = true
        starting = false
        relay.cancel()
        val shouldCloseOfficial = officialMediaSuspended
        officialMediaSuspended = false
        officialPeerReleased = false
        releaseInFlight = false
        peer.close()
        if (shouldCloseOfficial) relay.closeOfficialPeer()
        closing = false
    }

    private fun suspendOfficialMediaThenStart(
        token: Long,
        dataChannel: ChatGptWebPrivateVoiceDataChannelHint,
    ) {
        val accepted = relay.setOfficialMediaEnabled(false) { result ->
            if (token != generation) return@setOfficialMediaEnabled
            when (result) {
                is ChatGptWebPrivateVoiceMediaControl.Applied -> {
                    if (result.enabled) {
                        failStart("media_handoff_not_suspended")
                        return@setOfficialMediaEnabled
                    }
                    officialMediaSuspended = true
                    if (!peer.start(dataChannel)) {
                        restoreOfficialMedia(
                            ChatGptWebNativeVoiceState(
                                phase = ChatGptWebNativeVoicePhase.FAILED,
                                code = "native_start_rejected",
                            ),
                        )
                    }
                }
                is ChatGptWebPrivateVoiceMediaControl.Unavailable ->
                    failStart("media_handoff_${result.code}")
            }
        }
        // The gateway reports a rejected WebView post through the callback.
        // Avoid emitting a second failure for the same handoff attempt.
        if (!accepted) starting = false
    }

    private fun acceptPeerState(state: ChatGptWebNativeVoiceState) {
        starting = false
        val decorated = decorate(state)
        if (
            !closing &&
            !officialPeerReleased &&
            !releaseInFlight &&
            officialMediaSuspended &&
            state.phase == ChatGptWebNativeVoicePhase.CONNECTED &&
            state.remoteAudio &&
            state.dataChannelOpen
        ) {
            releaseOfficialPeer(state)
            return
        }
        if (
            !closing &&
            officialMediaSuspended &&
            !officialPeerReleased &&
            (state.phase == ChatGptWebNativeVoicePhase.FAILED ||
                state.phase == ChatGptWebNativeVoicePhase.CLOSED)
        ) {
            restoreOfficialMedia(decorated)
        } else {
            onState(decorated)
        }
    }

    private fun restoreOfficialMedia(state: ChatGptWebNativeVoiceState) {
        val token = generation
        onState(state.copy(officialMediaSuspended = true))
        relay.setOfficialMediaEnabled(true) { result ->
            if (token != generation) return@setOfficialMediaEnabled
            if (result is ChatGptWebPrivateVoiceMediaControl.Applied && result.enabled) {
                officialMediaSuspended = false
                onState(decorate(state))
            }
        }
    }

    private fun releaseOfficialPeer(state: ChatGptWebNativeVoiceState) {
        val token = generation
        releaseInFlight = true
        onState(decorate(state))
        val accepted = relay.closeOfficialPeer { result ->
            if (token != generation) return@closeOfficialPeer
            releaseInFlight = false
            when (result) {
                is ChatGptWebPrivateVoiceMediaControl.Applied -> {
                    officialPeerReleased = result.closed
                    if (!peer.ensureAudioRoute()) {
                        peer.close()
                        onState(
                            decorate(
                                ChatGptWebNativeVoiceState(
                                    phase = ChatGptWebNativeVoicePhase.FAILED,
                                    code = "audio_route_unavailable",
                                ),
                            ),
                        )
                    } else {
                        onState(decorate(state))
                    }
                }
                is ChatGptWebPrivateVoiceMediaControl.Unavailable -> {
                    closing = true
                    peer.close()
                    closing = false
                    restoreOfficialMedia(
                        ChatGptWebNativeVoiceState(
                            phase = ChatGptWebNativeVoicePhase.FAILED,
                            code = "media_release_${result.code}",
                        ),
                    )
                }
            }
        }
        if (!accepted) {
            releaseInFlight = false
            closing = true
            peer.close()
            closing = false
            restoreOfficialMedia(
                ChatGptWebNativeVoiceState(
                    phase = ChatGptWebNativeVoicePhase.FAILED,
                    code = "media_release_unavailable",
                ),
            )
        }
    }

    private fun decorate(state: ChatGptWebNativeVoiceState): ChatGptWebNativeVoiceState =
        state.copy(
            officialMediaSuspended = officialMediaSuspended,
            officialPeerReleased = officialPeerReleased,
        )

    private fun failStart(code: String) {
        starting = false
        onState(
            ChatGptWebNativeVoiceState(
                phase = ChatGptWebNativeVoicePhase.FAILED,
                officialMediaSuspended = officialMediaSuspended,
                officialPeerReleased = officialPeerReleased,
                code = code,
            ),
        )
    }
}
