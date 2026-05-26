package com.elon.app

import android.app.Application
import com.elon.app.mcp.*

class ElonApplication : Application() {

    /** 全局 WS 管理器，由 MainActivity 在 onResume/onStop 控制生命周期 */
    val globalWs: GlobalWsManager by lazy { GlobalWsManager(SERVER_URL) }

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

    companion object {
        const val SERVER_URL = "http://43.139.149.158:8080"
    }
}
