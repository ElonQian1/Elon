package com.elon.app

import android.Manifest
import android.content.pm.PackageManager
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

internal interface WebChatNativeDictationPort {
    fun start(
        onStateChanged: (WebChatNativeDictationState) -> Unit,
        onUnavailableBeforeCapture: () -> Boolean = { false },
    ): Boolean
    fun submit(): Boolean
    fun cancel(): Boolean
    fun state(): WebChatNativeDictationState
    fun destroy()
}

internal class MainWebChatDictationActions(
    private val activity: AppCompatActivity,
    private val speechPermissionRequest: Int,
    private val bridge: () -> AgentVoiceBridge,
    private val readDraft: () -> String,
    private val writeDraft: (String) -> Unit,
) : WebChatNativeDictationPort {
    private var stateListener: (WebChatNativeDictationState) -> Unit = {}
    private var unavailableFallback: () -> Boolean = { false }
    private var unavailableUntilMs = 0L
    private val sessionDelegate = lazy {
        WebChatNativeDictationSession(
            bridge = { AgentVoiceDictationEngine(bridge()) },
            readDraft = readDraft,
            writeDraft = writeDraft,
            onStateChanged = { stateListener(it) },
            onUnavailable = ::onUnavailable,
        )
    }

    override fun start(
        onStateChanged: (WebChatNativeDictationState) -> Unit,
        onUnavailableBeforeCapture: () -> Boolean,
    ): Boolean {
        stateListener = onStateChanged
        unavailableFallback = onUnavailableBeforeCapture
        if (ContextCompat.checkSelfPermission(activity, Manifest.permission.RECORD_AUDIO) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            DebugTraceStore.record(
                "web_chat_dictation_shared_permission_requested",
                mapOf("permission" to "record_audio"),
            )
            ActivityCompat.requestPermissions(
                activity,
                arrayOf(Manifest.permission.RECORD_AUDIO),
                speechPermissionRequest,
            )
            return true
        }
        if (System.currentTimeMillis() < unavailableUntilMs) {
            DebugTraceStore.record(
                "web_chat_dictation_shared_skipped",
                mapOf("reason" to "cooldown"),
            )
            return false
        }
        return sessionDelegate.value.start().also { accepted ->
            DebugTraceStore.record(
                "web_chat_dictation_shared_start",
                mapOf("accepted" to accepted),
            )
        }
    }

    override fun submit(): Boolean = initializedSession()?.submit() ?: false

    override fun cancel(): Boolean = initializedSession()?.cancel() ?: false

    override fun state(): WebChatNativeDictationState =
        initializedSession()?.state() ?: WebChatNativeDictationState()

    override fun destroy() {
        initializedSession()?.destroy()
        stateListener = {}
        unavailableFallback = { false }
    }

    private fun initializedSession(): WebChatNativeDictationSession? =
        sessionDelegate.takeIf { it.isInitialized() }?.value

    private fun onUnavailable(message: String) {
        val emptyRecognition = message == EMPTY_RECOGNITION
        if (!emptyRecognition) {
            unavailableUntilMs = System.currentTimeMillis() + FALLBACK_COOLDOWN_MS
        }
        val fallback = unavailableFallback
        unavailableFallback = { false }
        val fallbackAccepted = !emptyRecognition && runCatching(fallback).getOrDefault(false)
        DebugTraceStore.record(
            "web_chat_dictation_shared_unavailable",
            mapOf(
                "reason" to if (emptyRecognition) "empty_recognition" else "engine_unavailable",
                "fallback_accepted" to fallbackAccepted,
            ),
        )
        if (!fallbackAccepted) {
            Toast.makeText(
                activity,
                if (emptyRecognition) "没听清，请重试" else "语音输入暂时不可用，请稍后重试",
                Toast.LENGTH_SHORT,
            ).show()
        }
    }

    private companion object {
        const val EMPTY_RECOGNITION = "没有识别到语音"
        const val FALLBACK_COOLDOWN_MS = 60_000L
    }
}
