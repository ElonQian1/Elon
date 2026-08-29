package com.elon.app

internal class WebChatManagedRealtimeVoiceLaunchCoordinator(
    private val startTransport: () -> Boolean,
    private val transportState: () -> WebChatManagedRealtimeVoiceState,
    private val setMuted: (Boolean) -> Boolean,
    private val schedule: (Runnable, Long) -> Unit,
    private val isCurrent: (Int) -> Boolean,
) {
    private var attempted = false

    fun reset() {
        attempted = false
    }

    fun isManaged(): Boolean = transportState().managed

    fun ownsNativeMedia(): Boolean = transportState().phase in setOf(
        WebChatManagedRealtimeVoicePhase.STARTING,
        WebChatManagedRealtimeVoicePhase.ACTIVE,
    )

    fun setMutedIfActive(muted: Boolean): Boolean =
        transportState().phase == WebChatManagedRealtimeVoicePhase.ACTIVE && setMuted(muted)

    fun start(
        generation: Int,
        onConnecting: () -> Unit,
        onActive: () -> Unit,
        onOfficialFallback: () -> Unit,
        onUnavailable: () -> Unit,
        onTimeout: () -> Unit,
    ): Boolean {
        val before = transportState()
        if (before.phase == WebChatManagedRealtimeVoicePhase.UNAVAILABLE) return false
        if (!attempted && before.managed) {
            attempted = true
        } else if (!attempted) {
            if (!startTransport()) return false
            attempted = true
        }
        onConnecting()
        observe(
            generation = generation,
            attempt = 0,
            onActive = onActive,
            onOfficialFallback = onOfficialFallback,
            onUnavailable = onUnavailable,
            onTimeout = onTimeout,
        )
        return true
    }

    private fun observe(
        generation: Int,
        attempt: Int,
        onActive: () -> Unit,
        onOfficialFallback: () -> Unit,
        onUnavailable: () -> Unit,
        onTimeout: () -> Unit,
    ) {
        if (!attempted || !isCurrent(generation)) return
        when (transportState().phase) {
            WebChatManagedRealtimeVoicePhase.ACTIVE -> onActive()
            WebChatManagedRealtimeVoicePhase.OFFICIAL_FALLBACK -> onOfficialFallback()
            WebChatManagedRealtimeVoicePhase.FAILED,
            WebChatManagedRealtimeVoicePhase.CLOSED,
            WebChatManagedRealtimeVoicePhase.UNAVAILABLE -> {
                attempted = false
                onUnavailable()
            }
            WebChatManagedRealtimeVoicePhase.IDLE,
            WebChatManagedRealtimeVoicePhase.STARTING -> if (attempt >= MAX_POLLS) {
                attempted = false
                onTimeout()
            } else {
                schedule(
                    Runnable {
                        observe(
                            generation,
                            attempt + 1,
                            onActive,
                            onOfficialFallback,
                            onUnavailable,
                            onTimeout,
                        )
                    },
                    POLL_DELAY_MS,
                )
            }
        }
    }

    private companion object {
        const val POLL_DELAY_MS = 250L
        const val MAX_POLLS = 100
    }
}
