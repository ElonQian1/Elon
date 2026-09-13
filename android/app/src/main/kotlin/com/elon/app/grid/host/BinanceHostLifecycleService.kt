package com.elon.app.grid.host

import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder

/** Process lifetime only. Binding exposes no account, WebView, grant or business command. */
class BinanceHostLifecycleService : Service() {
    private val marker = Binder()
    override fun onBind(intent: Intent?): IBinder? = marker.takeIf {
        intent?.action == ACTION && intent.data == null && intent.clipData == null &&
            intent.selector == null && intent.categories.isNullOrEmpty() &&
            (intent.extras == null || intent.extras!!.isEmpty)
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        stopSelf(startId)
        return START_NOT_STICKY
    }
    companion object {
        const val SCHEMA = "yilong.binance_host_lifecycle.v1"
        const val ACTION = "com.elon.app.grid.BIND_HOST_LIFECYCLE_V1"
    }
}
