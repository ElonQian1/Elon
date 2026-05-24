package com.elon.app

import android.app.Application

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
    }
}
