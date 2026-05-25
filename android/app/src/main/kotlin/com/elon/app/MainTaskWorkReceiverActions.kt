package com.elon.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

internal class MainTaskWorkReceiverActions(
    private val activity: AppCompatActivity,
    private val handleTaskWorkEvent: (Intent) -> Unit
) {
    private var registered = false
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            handleTaskWorkEvent(intent)
        }
    }

    val isRegistered: Boolean
        get() = registered

    fun registerTaskWorkReceiver() {
        if (registered) return
        val filter = IntentFilter().apply {
            addAction(TaskWorkService.ACTION_EVENT)
            addAction(TaskWorkService.ACTION_STATE)
        }
        ContextCompat.registerReceiver(
            activity,
            receiver,
            filter,
            ContextCompat.RECEIVER_NOT_EXPORTED
        )
        registered = true
    }

    fun unregisterTaskWorkReceiver() {
        if (!registered) return
        activity.unregisterReceiver(receiver)
        registered = false
    }
}
