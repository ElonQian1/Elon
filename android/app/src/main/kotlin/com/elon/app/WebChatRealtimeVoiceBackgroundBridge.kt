package com.elon.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

internal interface WebChatRealtimeVoiceBackgroundControlSink {
    fun pauseFromBackground(source: WebChatRealtimeVoiceBackgroundControlSource)
    fun resumeFromBackground(source: WebChatRealtimeVoiceBackgroundControlSource)
    fun hangUpFromBackground()
}

internal interface WebChatRealtimeVoiceBackgroundPort {
    fun start(state: WebChatRealtimeVoiceState): Boolean
    fun update(state: WebChatRealtimeVoiceState)
    fun setPaused(paused: Boolean, detail: String)
    fun reportControlFailure(detail: String)
    fun setHostVisible(visible: Boolean)
    fun stop()
    fun dispose()
}

internal class WebChatRealtimeVoiceBackgroundBridge(
    private val activity: AppCompatActivity,
) : WebChatRealtimeVoiceBackgroundPort {
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action != WebChatRealtimeVoiceBackgroundProtocol.ACTION_CONTROL) return
            val source = WebChatRealtimeVoiceBackgroundProtocol.source(
                intent.getStringExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_SOURCE),
            )
            when (
                WebChatRealtimeVoiceBackgroundProtocol.control(
                    intent.getStringExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_CONTROL),
                )
            ) {
                WebChatRealtimeVoiceBackgroundControl.PAUSE -> sink?.pauseFromBackground(source)
                WebChatRealtimeVoiceBackgroundControl.RESUME -> sink?.resumeFromBackground(source)
                WebChatRealtimeVoiceBackgroundControl.HANG_UP -> sink?.hangUpFromBackground()
                null -> Unit
            }
        }
    }
    private var sink: WebChatRealtimeVoiceBackgroundControlSink? = null
    private var running = false
    private var hostVisible = true
    private var registered = false

    init {
        ContextCompat.registerReceiver(
            activity,
            receiver,
            IntentFilter(WebChatRealtimeVoiceBackgroundProtocol.ACTION_CONTROL),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
        registered = true
    }

    fun attach(sink: WebChatRealtimeVoiceBackgroundControlSink) {
        this.sink = sink
    }

    override fun start(state: WebChatRealtimeVoiceState): Boolean {
        val intent = serviceIntent(WebChatRealtimeVoiceBackgroundProtocol.ACTION_START, state)
            .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_HOST_VISIBLE, hostVisible)
        return runCatching {
            ContextCompat.startForegroundService(activity, intent)
            running = true
        }.isSuccess
    }

    override fun update(state: WebChatRealtimeVoiceState) {
        if (!running) return
        startService(serviceIntent(WebChatRealtimeVoiceBackgroundProtocol.ACTION_UPDATE, state))
    }

    override fun setPaused(paused: Boolean, detail: String) {
        if (!running) return
        val status = if (paused) {
            WebChatRealtimeVoiceBackgroundStatus.PAUSED
        } else {
            WebChatRealtimeVoiceBackgroundStatus.LISTENING
        }
        startService(
            Intent(activity, WebChatRealtimeVoiceBackgroundService::class.java)
                .setAction(WebChatRealtimeVoiceBackgroundProtocol.ACTION_UPDATE)
                .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_STATUS, status.wireValue)
                .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_DETAIL, detail),
        )
    }

    override fun reportControlFailure(detail: String) {
        if (!running) return
        startService(
            Intent(activity, WebChatRealtimeVoiceBackgroundService::class.java)
                .setAction(WebChatRealtimeVoiceBackgroundProtocol.ACTION_UPDATE)
                .putExtra(
                    WebChatRealtimeVoiceBackgroundProtocol.EXTRA_STATUS,
                    WebChatRealtimeVoiceBackgroundStatus.ERROR.wireValue,
                )
                .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_DETAIL, detail),
        )
    }

    override fun setHostVisible(visible: Boolean) {
        hostVisible = visible
        if (!running) return
        startService(
            Intent(activity, WebChatRealtimeVoiceBackgroundService::class.java)
                .setAction(WebChatRealtimeVoiceBackgroundProtocol.ACTION_HOST_VISIBILITY)
                .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_HOST_VISIBLE, visible),
        )
    }

    override fun stop() {
        if (!running) return
        startService(
            Intent(activity, WebChatRealtimeVoiceBackgroundService::class.java)
                .setAction(WebChatRealtimeVoiceBackgroundProtocol.ACTION_STOP),
        )
        running = false
    }

    override fun dispose() {
        stop()
        sink = null
        if (registered) runCatching { activity.unregisterReceiver(receiver) }
        registered = false
    }

    private fun serviceIntent(action: String, state: WebChatRealtimeVoiceState): Intent =
        Intent(activity, WebChatRealtimeVoiceBackgroundService::class.java)
            .setAction(action)
            .putExtra(
                WebChatRealtimeVoiceBackgroundProtocol.EXTRA_STATUS,
                WebChatRealtimeVoiceBackgroundStatusPolicy.from(state).wireValue,
            )
            .putExtra(WebChatRealtimeVoiceBackgroundProtocol.EXTRA_DETAIL, state.detail)

    private fun startService(intent: Intent) {
        runCatching { activity.startService(intent) }
    }
}
