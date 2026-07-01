package com.elon.app.mcp

import com.elon.app.DebugTraceStore
import android.app.Activity
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.core.content.ContextCompat

class McpDebugControlReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent?) {
        val action = intent?.action ?: ACTION_STATUS
        DebugTraceStore.init(context)
        McpDebugServer.start(context)

        var serviceStarted = false
        var error: String? = null
        when (action) {
            ACTION_START -> {
                val startIntent = Intent(context, McpDebugKeepAliveService::class.java).apply {
                    this.action = McpDebugKeepAliveService.ACTION_START
                }
                val startError = runCatching {
                    ContextCompat.startForegroundService(context, startIntent)
                }.exceptionOrNull()
                serviceStarted = startError == null
                error = startError?.message ?: startError?.javaClass?.simpleName
            }
            ACTION_STOP -> {
                val stopIntent = Intent(context, McpDebugKeepAliveService::class.java).apply {
                    this.action = McpDebugKeepAliveService.ACTION_STOP
                }
                context.stopService(stopIntent)
            }
        }

        val result = McpNativeControlBridge.bootstrapJson(context, action, serviceStarted, error)
        setResultCode(if (error == null) Activity.RESULT_OK else Activity.RESULT_CANCELED)
        setResultData(result.toString())
        setResultExtras(Bundle().apply { putString("json", result.toString()) })
        DebugTraceStore.record(
            "mcp_adb_bootstrap",
            mapOf(
                "action" to action,
                "service_started" to serviceStarted,
                "error" to error,
                "port" to 8787
            )
        )
        Log.i(TAG, "MCP bootstrap action=$action endpoint=http://127.0.0.1:8787/mcp")
    }

    companion object {
        private const val TAG = "ElonMcpBootstrap"
        const val ACTION_START = "com.elon.app.mcp.START_DEBUG"
        const val ACTION_STOP = "com.elon.app.mcp.STOP_DEBUG"
        const val ACTION_STATUS = "com.elon.app.mcp.STATUS"
    }
}
