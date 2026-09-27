package com.elon.app

import android.content.Context
import android.content.res.Configuration
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import com.google.android.material.dialog.MaterialAlertDialogBuilder

/** Device appearance preference, deliberately independent of account/session storage. */
internal object MobileThemePreference {
    private val modes = intArrayOf(AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM,
        AppCompatDelegate.MODE_NIGHT_NO, AppCompatDelegate.MODE_NIGHT_YES)
    private val labels = arrayOf("跟随系统", "浅色", "深色")
    private fun prefs(context: Context) = context.getSharedPreferences("mobile_appearance", Context.MODE_PRIVATE)
    private fun selected(context: Context) = prefs(context).getInt("mode", modes[0]).let {
        if (it in modes) it else modes[0]
    }
    fun apply(context: Context) = AppCompatDelegate.setDefaultNightMode(selected(context))
    fun night(context: Context): Int = when (selected(context)) {
        AppCompatDelegate.MODE_NIGHT_NO -> Configuration.UI_MODE_NIGHT_NO
        AppCompatDelegate.MODE_NIGHT_YES -> Configuration.UI_MODE_NIGHT_YES
        else -> context.applicationContext.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
    }
    fun show(activity: AppCompatActivity) {
        MaterialAlertDialogBuilder(activity)
            .setTitle("外观")
            .setSingleChoiceItems(labels, modes.indexOf(selected(activity))) { dialog, index ->
                prefs(activity).edit().putInt("mode", modes[index]).apply()
                dialog.dismiss()
                apply(activity)
            }
            .setNegativeButton("取消", null)
            .show()
    }
}
