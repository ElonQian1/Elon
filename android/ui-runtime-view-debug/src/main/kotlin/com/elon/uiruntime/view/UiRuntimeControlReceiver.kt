package com.elon.uiruntime.view

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

class UiRuntimeControlReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            ACTION_START -> {
                val sessionId = intent.getStringExtra(EXTRA_SESSION_ID).orEmpty()
                val token = intent.getStringExtra(EXTRA_SESSION_TOKEN).orEmpty()
                val port = intent.getIntExtra(EXTRA_DEVICE_PORT, DEFAULT_PORT)
                if (sessionId.isBlank() || token.length < 16 || port !in 1..65535) {
                    Log.e(TAG, "Invalid Live UI start request")
                    return
                }
                UiRuntimeController.start(context, sessionId, token, port)
            }
            ACTION_STOP -> UiRuntimeController.stop(
                intent.getStringExtra(EXTRA_SESSION_ID)?.takeIf { it.isNotBlank() },
            )
        }
    }

    companion object {
        private const val TAG = "YilongUiRuntime"
        private const val ACTION_START = "com.elon.uiruntime.START"
        private const val ACTION_STOP = "com.elon.uiruntime.STOP"
        private const val EXTRA_SESSION_ID = "session_id"
        private const val EXTRA_SESSION_TOKEN = "session_token"
        private const val EXTRA_DEVICE_PORT = "device_port"
        private const val DEFAULT_PORT = 38_917
    }
}
