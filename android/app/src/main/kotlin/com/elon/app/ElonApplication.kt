package com.elon.app

import android.app.Application
import android.content.Intent
import androidx.core.content.ContextCompat

class ElonApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        DebugTraceStore.init(this)
        DebugTraceStore.record(
            "app_start",
            mapOf(
                "version_name" to BuildConfig.VERSION_NAME,
                "version_code" to BuildConfig.VERSION_CODE
            )
        )
        McpDebugServer.start(this)
        startMcpKeepAliveIfEnabled()
    }

    private fun startMcpKeepAliveIfEnabled() {
        if (!McpDebugKeepAliveService.shouldAutoStart(this)) return
        val intent = Intent(this, McpDebugKeepAliveService::class.java).apply {
            action = McpDebugKeepAliveService.ACTION_START
        }
        runCatching {
            ContextCompat.startForegroundService(this, intent)
        }.onSuccess {
            DebugTraceStore.record("mcp_keepalive_auto_start_requested")
        }.onFailure { error ->
            DebugTraceStore.record(
                "mcp_keepalive_auto_start_failed",
                mapOf("error" to (error.message ?: error.javaClass.simpleName))
            )
        }
    }
}
