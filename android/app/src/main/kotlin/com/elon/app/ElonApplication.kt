package com.elon.app

import android.app.Application

class ElonApplication : Application() {
    override fun onCreate() {
        super.onCreate()
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
