package com.elon.app.chatgptweb

import android.content.Context
import livekit.org.webrtc.PeerConnectionFactory

/** Process-wide native WebRTC factory, loaded only by the explicit research build. */
internal object ChatGptWebNativeVoiceRuntime {
    @Volatile
    private var factory: PeerConnectionFactory? = null

    fun factory(context: Context): PeerConnectionFactory {
        factory?.let { return it }
        return synchronized(this) {
            factory ?: createFactory(context.applicationContext).also { factory = it }
        }
    }

    private fun createFactory(context: Context): PeerConnectionFactory {
        PeerConnectionFactory.initialize(
            PeerConnectionFactory.InitializationOptions.builder(context)
                .setEnableInternalTracer(false)
                .createInitializationOptions(),
        )
        return PeerConnectionFactory.builder().createPeerConnectionFactory()
    }
}
