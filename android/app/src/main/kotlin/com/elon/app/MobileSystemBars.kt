package com.elon.app

import android.app.Activity
import android.app.Application
import android.os.Bundle
import androidx.core.view.WindowCompat

/** Reconcile icon contrast after each screen has configured its window; preserve visibility and insets. */
internal object MobileSystemBars : Application.ActivityLifecycleCallbacks {
    override fun onActivityResumed(activity: Activity) {
        activity.window.decorView.post {
            if (!activity.isDestroyed) {
                val light = activity.resources.getBoolean(R.bool.mobile_light_system_bars)
                WindowCompat.getInsetsController(activity.window, activity.window.decorView).apply {
                    isAppearanceLightStatusBars = light
                    isAppearanceLightNavigationBars = light
                }
                @Suppress("DEPRECATION")
                activity.window.statusBarColor = activity.getColor(R.color.mobile_surface)
                @Suppress("DEPRECATION")
                activity.window.navigationBarColor = activity.getColor(R.color.mobile_surface)
            }
        }
    }
    override fun onActivityCreated(activity: Activity, state: Bundle?) = Unit
    override fun onActivityStarted(activity: Activity) = Unit
    override fun onActivityPaused(activity: Activity) = Unit
    override fun onActivityStopped(activity: Activity) = Unit
    override fun onActivitySaveInstanceState(activity: Activity, state: Bundle) = Unit
    override fun onActivityDestroyed(activity: Activity) = Unit
}
