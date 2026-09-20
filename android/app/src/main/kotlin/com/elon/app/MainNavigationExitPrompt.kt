package com.elon.app

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class MainNavigationExitPrompt(private val activity: AppCompatActivity) {
    private var dialog: AlertDialog? = null

    fun show() {
        if (dialog?.isShowing == true) return
        dialog = AlertDialog.Builder(activity)
            .setTitle("退出应用")
            .setMessage("确定要退出一龙吗？")
            .setNegativeButton("取消", null)
            .setPositiveButton("退出") { _, _ -> activity.finish() }
            .create()
        dialog?.show()
    }
}
