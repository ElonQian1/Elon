package com.elon.app.chatgptweb

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Handler
import android.os.Looper
import androidx.core.content.ContextCompat
import com.elon.app.BuildConfig
import livekit.org.webrtc.AudioSource
import livekit.org.webrtc.AudioTrack
import livekit.org.webrtc.DataChannel
import livekit.org.webrtc.IceCandidate
import livekit.org.webrtc.MediaConstraints
import livekit.org.webrtc.MediaStream
import livekit.org.webrtc.PeerConnection
import livekit.org.webrtc.RtpReceiver
import livekit.org.webrtc.RtpTransceiver
import livekit.org.webrtc.SdpObserver
import livekit.org.webrtc.SessionDescription

internal enum class ChatGptWebNativeVoicePhase {
    IDLE,
    BOOTSTRAPPING,
    CREATING_OFFER,
    RELAYING,
    CONNECTING,
    CONNECTED,
    FAILED,
    CLOSED,
}

internal data class ChatGptWebNativeVoiceState(
    val phase: ChatGptWebNativeVoicePhase,
    val remoteAudio: Boolean = false,
    val dataChannelOpen: Boolean = false,
    val code: String? = null,
)

/** Audio-only research peer. SDP and relay answers never enter logs or durable state. */
internal class ChatGptWebNativeVoicePeer(
    private val context: Context,
    private val relay: (
        String,
        (ChatGptWebPrivateVoiceRelayResult) -> Unit,
    ) -> Boolean,
    private val schedule: (Runnable, Long) -> Unit,
    private val onState: (ChatGptWebNativeVoiceState) -> Unit,
    private val mainHandler: Handler = Handler(Looper.getMainLooper()),
) {
    private var generation = 0L
    private var phase = ChatGptWebNativeVoicePhase.IDLE
    private var peer: PeerConnection? = null
    private var audioSource: AudioSource? = null
    private var audioTrack: AudioTrack? = null
    private var dataChannel: DataChannel? = null
    private var remoteAudio = false
    private var dataChannelOpen = false

    fun start(hint: ChatGptWebPrivateVoiceDataChannelHint): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED || !canStart()) return false
        if (
            ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            fail("microphone_permission_required")
            return false
        }
        generation += 1
        val token = generation
        remoteAudio = false
        dataChannelOpen = false
        update(ChatGptWebNativeVoicePhase.CREATING_OFFER)
        val factory = runCatching { ChatGptWebNativeVoiceRuntime.factory(context) }.getOrNull()
            ?: return fail("native_runtime_unavailable")
        val configuration = PeerConnection.RTCConfiguration(emptyList()).apply {
            sdpSemantics = PeerConnection.SdpSemantics.UNIFIED_PLAN
            continualGatheringPolicy = PeerConnection.ContinualGatheringPolicy.GATHER_CONTINUALLY
        }
        val createdPeer = factory.createPeerConnection(configuration, observer(token))
            ?: return fail("peer_creation_failed")
        peer = createdPeer
        audioSource = factory.createAudioSource(MediaConstraints())
        audioTrack = factory.createAudioTrack(AUDIO_TRACK_ID, audioSource).also {
            it.setEnabled(true)
            createdPeer.addTrack(it, listOf(AUDIO_STREAM_ID))
        }
        dataChannel = createdPeer.createDataChannel(hint.label, hint.toInit()).also {
            it.registerObserver(dataChannelObserver(token))
        }
        schedule(Runnable { timeout(token) }, CONNECT_TIMEOUT_MS)
        createdPeer.createOffer(createOfferObserver(token), MediaConstraints())
        return true
    }

    fun setMuted(muted: Boolean): Boolean = audioTrack?.setEnabled(!muted) == true

    fun close() {
        generation += 1
        releasePeer()
        update(ChatGptWebNativeVoicePhase.CLOSED)
    }

    private fun canStart(): Boolean =
        phase == ChatGptWebNativeVoicePhase.IDLE ||
            phase == ChatGptWebNativeVoicePhase.FAILED ||
            phase == ChatGptWebNativeVoicePhase.CLOSED

    private fun createOfferObserver(token: Long): SdpObserver = object : SafeSdpObserver() {
        override fun onCreateSuccess(description: SessionDescription) {
            if (!current(token)) return
            peer?.setLocalDescription(localDescriptionObserver(token), description)
                ?: fail("peer_closed")
        }

        override fun onCreateFailure(error: String?) {
            if (current(token)) fail("offer_creation_failed")
        }
    }

    private fun localDescriptionObserver(token: Long): SdpObserver = object : SafeSdpObserver() {
        override fun onSetSuccess() {
            if (!current(token)) return
            val offer = peer?.localDescription?.description
                ?.takeIf(ChatGptWebPrivateVoiceRelayContract::validOffer)
            if (offer == null) {
                fail("invalid_local_offer")
                return
            }
            update(ChatGptWebNativeVoicePhase.RELAYING)
            mainHandler.post {
                if (!current(token)) return@post
                if (!relay(offer) { result -> acceptRelay(token, result) }) {
                    fail("relay_unavailable")
                }
            }
        }

        override fun onSetFailure(error: String?) {
            if (current(token)) fail("local_description_failed")
        }
    }

    private fun acceptRelay(token: Long, result: ChatGptWebPrivateVoiceRelayResult) {
        if (!current(token)) return
        when (result) {
            is ChatGptWebPrivateVoiceRelayResult.Success -> {
                val answer = SessionDescription(
                    SessionDescription.Type.ANSWER,
                    result.answer.value(),
                )
                peer?.setRemoteDescription(remoteDescriptionObserver(token), answer)
                    ?: fail("peer_closed")
            }
            is ChatGptWebPrivateVoiceRelayResult.Failure -> fail("relay_${result.code}")
        }
    }

    private fun remoteDescriptionObserver(token: Long): SdpObserver = object : SafeSdpObserver() {
        override fun onSetSuccess() {
            if (current(token)) update(ChatGptWebNativeVoicePhase.CONNECTING)
        }

        override fun onSetFailure(error: String?) {
            if (current(token)) fail("remote_description_failed")
        }
    }

    private fun observer(token: Long): PeerConnection.Observer = object : PeerConnection.Observer {
        override fun onSignalingChange(state: PeerConnection.SignalingState) = Unit
        override fun onIceConnectionChange(state: PeerConnection.IceConnectionState) {
            if (!current(token)) return
            if (state == PeerConnection.IceConnectionState.FAILED) fail("ice_failed")
        }

        override fun onIceConnectionReceivingChange(receiving: Boolean) = Unit
        override fun onIceGatheringChange(state: PeerConnection.IceGatheringState) = Unit
        override fun onIceCandidate(candidate: IceCandidate) = Unit
        override fun onIceCandidatesRemoved(candidates: Array<out IceCandidate>) = Unit
        override fun onAddStream(stream: MediaStream) = Unit
        override fun onRemoveStream(stream: MediaStream) = Unit
        override fun onDataChannel(channel: DataChannel) = Unit
        override fun onRenegotiationNeeded() = Unit

        override fun onConnectionChange(state: PeerConnection.PeerConnectionState) {
            if (!current(token)) return
            when (state) {
                PeerConnection.PeerConnectionState.CONNECTED ->
                    update(ChatGptWebNativeVoicePhase.CONNECTED)
                PeerConnection.PeerConnectionState.FAILED -> fail("peer_failed")
                PeerConnection.PeerConnectionState.CLOSED -> closeFromPeer(token)
                else -> Unit
            }
        }

        override fun onAddTrack(receiver: RtpReceiver, mediaStreams: Array<out MediaStream>) {
            acceptRemoteTrack(token, receiver.track()?.kind())
        }

        override fun onTrack(transceiver: RtpTransceiver) {
            acceptRemoteTrack(token, transceiver.receiver.track()?.kind())
        }
    }

    private fun dataChannelObserver(token: Long): DataChannel.Observer = object : DataChannel.Observer {
        override fun onBufferedAmountChange(previousAmount: Long) = Unit
        override fun onMessage(buffer: DataChannel.Buffer) = Unit
        override fun onStateChange() {
            if (!current(token)) return
            dataChannelOpen = dataChannel?.state() == DataChannel.State.OPEN
            emit()
        }
    }

    private fun acceptRemoteTrack(token: Long, kind: String?) {
        if (!current(token) || kind != "audio") return
        remoteAudio = true
        emit()
    }

    private fun timeout(token: Long) {
        if (current(token) && phase != ChatGptWebNativeVoicePhase.CONNECTED) {
            fail("connect_timeout")
        }
    }

    private fun closeFromPeer(token: Long) {
        if (!current(token)) return
        generation += 1
        releasePeer()
        update(ChatGptWebNativeVoicePhase.CLOSED)
    }

    private fun fail(code: String): Boolean {
        generation += 1
        releasePeer()
        phase = ChatGptWebNativeVoicePhase.FAILED
        onState(ChatGptWebNativeVoiceState(phase = phase, code = code))
        return false
    }

    private fun releasePeer() {
        dataChannel?.unregisterObserver()
        dataChannel?.close()
        dataChannel?.dispose()
        dataChannel = null
        peer?.setAudioRecording(false)
        peer?.setAudioPlayout(false)
        peer?.close()
        peer?.dispose()
        peer = null
        audioTrack?.dispose()
        audioTrack = null
        audioSource?.dispose()
        audioSource = null
        remoteAudio = false
        dataChannelOpen = false
    }

    private fun update(next: ChatGptWebNativeVoicePhase) {
        phase = next
        emit()
    }

    private fun emit() {
        onState(
            ChatGptWebNativeVoiceState(
                phase = phase,
                remoteAudio = remoteAudio,
                dataChannelOpen = dataChannelOpen,
            ),
        )
    }

    private fun current(token: Long): Boolean = token == generation

    private fun ChatGptWebPrivateVoiceDataChannelHint.toInit(): DataChannel.Init =
        DataChannel.Init().also { init ->
            init.ordered = ordered
            maxRetransmits?.let { init.maxRetransmits = it }
            init.protocol = protocol
            init.negotiated = negotiated
            id?.let { init.id = it }
        }

    private open class SafeSdpObserver : SdpObserver {
        override fun onCreateSuccess(description: SessionDescription) = Unit
        override fun onSetSuccess() = Unit
        override fun onCreateFailure(error: String?) = Unit
        override fun onSetFailure(error: String?) = Unit
    }

    private companion object {
        const val AUDIO_TRACK_ID = "elon_private_voice_audio"
        const val AUDIO_STREAM_ID = "elon_private_voice_stream"
        const val CONNECT_TIMEOUT_MS = 20_000L
    }
}
