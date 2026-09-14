package com.elon.app

import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.IBinder

/** Compatibility entry: all chat background work has one owner. */
class ChatRealtimeService : Service() {
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        ChatBackgroundService.start(this)
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    companion object {
        fun ensureRunning(context: Context) = ChatBackgroundService.start(context)
    }
}
