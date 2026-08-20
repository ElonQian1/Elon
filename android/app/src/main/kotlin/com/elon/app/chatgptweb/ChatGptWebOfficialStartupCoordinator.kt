package com.elon.app.chatgptweb

internal enum class ChatGptWebOfficialStartupAction(val wireValue: String) {
    REALTIME_VOICE("realtime_voice");

    companion object {
        fun fromWireValue(raw: String?): ChatGptWebOfficialStartupAction? =
            entries.firstOrNull { it.wireValue == raw?.trim() }
    }
}

internal enum class ChatGptWebOfficialStartupFeedback {
    CONNECTING,
    REQUESTING_MICROPHONE,
    STARTING,
    STARTED,
    MICROPHONE_DENIED,
    UNAVAILABLE,
}

internal class ChatGptWebOfficialStartupCoordinator(
    private val action: ChatGptWebOfficialStartupAction?,
    private val requestManifest: () -> Unit,
    private val requestMicrophone: (onGranted: () -> Unit, onDenied: () -> Unit) -> Unit,
    private val invokeControl: (controlId: String, requestId: String) -> Unit,
    private val schedule: (delayMs: Long, action: () -> Unit) -> Unit,
    private val requestId: () -> String,
    private val onFeedback: (ChatGptWebOfficialStartupFeedback) -> Unit,
) {
    private enum class State {
        IDLE,
        WAITING_PAGE,
        WAITING_MANIFEST,
        WAITING_PERMISSION,
        INVOKING,
        STARTED,
        FAILED,
    }

    private var state = if (action == null) State.IDLE else State.WAITING_PAGE
    private var hostResumed = false
    private var microphoneGranted = false
    private var startupClaimed = false
    private var discoveryGeneration = 0
    private var discoveryStarted = false
    private var manifestRequestCount = 0
    private var invocationRequestId: String? = null

    fun onPageStarted() {
        if (state == State.WAITING_MANIFEST && !startupClaimed) {
            invalidateDiscovery()
            state = State.WAITING_PAGE
        }
    }

    fun onPageReady(enhancedModeSupported: Boolean) {
        if (state != State.WAITING_PAGE) return
        // Authentication can temporarily leave chatgpt.com. Keep the one-shot
        // request pending until the enhanced page is available again.
        if (!enhancedModeSupported) return
        state = State.WAITING_MANIFEST
        onFeedback(ChatGptWebOfficialStartupFeedback.CONNECTING)
        beginManifestDiscovery()
    }

    fun onAdapterReady() {
        if (state == State.WAITING_MANIFEST && hostResumed) {
            if (discoveryStarted) {
                requestManifestWithRetry(discoveryGeneration)
            } else {
                beginManifestDiscovery()
            }
        }
    }

    fun onHostResumed() {
        hostResumed = true
        if (state == State.WAITING_MANIFEST) beginManifestDiscovery()
    }

    fun onHostPaused() {
        hostResumed = false
        if (state == State.WAITING_MANIFEST) invalidateDiscovery()
    }

    fun dispose() {
        hostResumed = false
        invalidateDiscovery()
        invocationRequestId = null
        state = State.IDLE
    }

    fun onEvent(event: ChatGptWebEvent) {
        when (event) {
            is ChatGptWebEvent.UiManifest -> onManifest(event.value)
            is ChatGptWebEvent.CommandResult -> onCommandResult(event)
            else -> Unit
        }
    }

    fun onTouchDispatchFailed(controlId: String?) {
        if (state == State.INVOKING && !controlId.isNullOrBlank()) fail()
    }

    fun requestConsumed(): Boolean = startupClaimed || state in setOf(State.STARTED, State.FAILED)

    private fun onManifest(manifest: ChatGptWebUiManifest) {
        if (state != State.WAITING_MANIFEST || action != ChatGptWebOfficialStartupAction.REALTIME_VOICE) {
            return
        }
        val control = ChatGptRealtimeVoicePolicy.resolve(manifest)
        if (control == null) {
            requestManifestWithRetry(discoveryGeneration)
            return
        }
        if (microphoneGranted) {
            invoke(control.id)
            return
        }
        startupClaimed = true
        state = State.WAITING_PERMISSION
        invalidateDiscovery()
        onFeedback(ChatGptWebOfficialStartupFeedback.REQUESTING_MICROPHONE)
        requestMicrophone(
            {
                if (state == State.WAITING_PERMISSION) {
                    microphoneGranted = true
                    state = State.WAITING_MANIFEST
                    beginManifestDiscovery()
                }
            },
            {
                if (state == State.WAITING_PERMISSION) {
                    state = State.FAILED
                    onFeedback(ChatGptWebOfficialStartupFeedback.MICROPHONE_DENIED)
                }
            },
        )
    }

    private fun invoke(controlId: String) {
        if (state != State.WAITING_MANIFEST || !microphoneGranted || !hostResumed) return
        invalidateDiscovery()
        state = State.INVOKING
        val id = requestId()
        invocationRequestId = id
        onFeedback(ChatGptWebOfficialStartupFeedback.STARTING)
        invokeControl(controlId, id)
        schedule(INVOCATION_TIMEOUT_MS) {
            if (state == State.INVOKING && invocationRequestId == id) fail()
        }
    }

    private fun onCommandResult(event: ChatGptWebEvent.CommandResult) {
        if (
            state != State.INVOKING ||
            event.requestId.isNullOrBlank() ||
            event.requestId != invocationRequestId
        ) {
            return
        }
        if (!event.ok) return fail()
        state = State.STARTED
        onFeedback(ChatGptWebOfficialStartupFeedback.STARTED)
    }

    private fun beginManifestDiscovery() {
        if (state != State.WAITING_MANIFEST || !hostResumed || discoveryStarted) return
        discoveryStarted = true
        manifestRequestCount = 0
        val generation = ++discoveryGeneration
        requestManifestWithRetry(generation)
        schedule(DISCOVERY_TIMEOUT_MS) {
            if (
                state == State.WAITING_MANIFEST &&
                hostResumed &&
                discoveryStarted &&
                discoveryGeneration == generation
            ) {
                fail()
            }
        }
    }

    private fun requestManifestWithRetry(generation: Int) {
        if (
            state != State.WAITING_MANIFEST ||
            !hostResumed ||
            !discoveryStarted ||
            generation != discoveryGeneration ||
            manifestRequestCount >= MAX_MANIFEST_REQUESTS
        ) {
            return
        }
        manifestRequestCount += 1
        requestManifest()
        schedule(MANIFEST_RETRY_DELAY_MS) {
            requestManifestWithRetry(generation)
        }
    }

    private fun invalidateDiscovery() {
        discoveryStarted = false
        discoveryGeneration += 1
    }

    private fun fail() {
        if (state == State.STARTED || state == State.FAILED || state == State.IDLE) return
        state = State.FAILED
        onFeedback(ChatGptWebOfficialStartupFeedback.UNAVAILABLE)
    }

    private companion object {
        const val MAX_MANIFEST_REQUESTS = 8
        const val MANIFEST_RETRY_DELAY_MS = 450L
        const val DISCOVERY_TIMEOUT_MS = 12_000L
        const val INVOCATION_TIMEOUT_MS = 8_000L
    }
}
