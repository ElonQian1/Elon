package com.elon.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * 系统广播接收器，在以下时机重新拉起聊天后台保活服务：
 *  - 设备开机完成（BOOT_COMPLETED / QUICKBOOT_POWERON）
 *
 * 原因：前台服务在进程被系统完全杀死或重启后不会自动重建，
 * 需要通过 BOOT_COMPLETED 重新启动。
 */
class WsRestartReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action ?: return
        Log.d(TAG, "收到广播: $action")
        when (action) {
            Intent.ACTION_BOOT_COMPLETED,
            "android.intent.action.QUICKBOOT_POWERON",   // 部分华为/荣耀设备的快速开机
            "com.htc.intent.action.QUICKBOOT_POWERON" -> {
                if (AuthManager.isLoggedIn(context) &&
                    ChatBackgroundPrefs.isKeepAliveEnabled(context)
                ) {
                    Log.i(TAG, "开机完成，启动聊天保活服务")
                    ChatBackgroundService.start(context)
                }
            }
        }
    }

    companion object {
        private const val TAG = "WsRestartReceiver"
    }
}
