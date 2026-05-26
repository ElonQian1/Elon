package com.elon.app.mcp

import com.elon.app.*
import android.Manifest
import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.BatteryManager
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject

internal fun processStateJson(): JSONObject {
    val info = ActivityManager.RunningAppProcessInfo()
    runCatching { ActivityManager.getMyMemoryState(info) }
    val foreground = info.importance <= ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND
    val foregroundOrService = info.importance <= ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND_SERVICE
    return JSONObject()
        .put("importance", info.importance)
        .put("importance_name", processImportanceName(info.importance))
        .put("foreground", foreground)
        .put("foreground_or_service", foregroundOrService)
        .put("last_trim_level", info.lastTrimLevel)
}

private fun processImportanceName(importance: Int): String {
    return when (importance) {
        ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND -> "foreground"
        ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND_SERVICE -> "foreground_service"
        ActivityManager.RunningAppProcessInfo.IMPORTANCE_VISIBLE -> "visible"
        ActivityManager.RunningAppProcessInfo.IMPORTANCE_PERCEPTIBLE -> "perceptible"
        ActivityManager.RunningAppProcessInfo.IMPORTANCE_CACHED -> "cached"
        ActivityManager.RunningAppProcessInfo.IMPORTANCE_GONE -> "gone"
        else -> "unknown_$importance"
    }
}

internal fun notificationPermissionJson(context: Context): JSONObject {
    val requiresRuntimePermission = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU
    val granted = !requiresRuntimePermission ||
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED
    return JSONObject()
        .put("permission", Manifest.permission.POST_NOTIFICATIONS)
        .put("requires_runtime_permission", requiresRuntimePermission)
        .put("granted", granted)
}

internal fun batteryOptimizationJson(context: Context): JSONObject {
    val powerManager = context.getSystemService(PowerManager::class.java)
    val ignoring = runCatching {
        powerManager.isIgnoringBatteryOptimizations(context.packageName)
    }.getOrDefault(true)
    val powerSaveMode = runCatching { powerManager.isPowerSaveMode }.getOrDefault(false)
    return JSONObject()
        .put("ignoring", ignoring)
        .put("power_save_mode", powerSaveMode)
        .put("request_intent_action", Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS)
        .put("request_intent_data", "package:${context.packageName}")
}

internal fun memoryJson(context: Context): JSONObject {
    val runtime = Runtime.getRuntime()
    val activityManager = context.getSystemService(ActivityManager::class.java)
    val memoryInfo = ActivityManager.MemoryInfo()
    runCatching { activityManager.getMemoryInfo(memoryInfo) }
    return JSONObject()
        .put("runtime_max_bytes", runtime.maxMemory())
        .put("runtime_total_bytes", runtime.totalMemory())
        .put("runtime_free_bytes", runtime.freeMemory())
        .put("runtime_used_bytes", runtime.totalMemory() - runtime.freeMemory())
        .put("system_avail_bytes", memoryInfo.availMem)
        .put("system_total_bytes", memoryInfo.totalMem)
        .put("system_threshold_bytes", memoryInfo.threshold)
        .put("system_low_memory", memoryInfo.lowMemory)
}

internal fun batteryJson(context: Context): JSONObject {
    val intent = context.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
    val level = intent?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
    val scale = intent?.getIntExtra(BatteryManager.EXTRA_SCALE, -1) ?: -1
    val status = intent?.getIntExtra(BatteryManager.EXTRA_STATUS, -1) ?: -1
    val plugged = intent?.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0) ?: 0
    val temperatureTenths = intent?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, Int.MIN_VALUE)
        ?: Int.MIN_VALUE
    return JSONObject()
        .put("level_percent", if (level >= 0 && scale > 0) (level * 100.0 / scale) else JSONObject.NULL)
        .put("status", batteryStatusName(status))
        .put("plugged", plugged != 0)
        .put("plugged_kind", batteryPluggedKind(plugged))
        .put(
            "temperature_c",
            if (temperatureTenths != Int.MIN_VALUE) temperatureTenths / 10.0 else JSONObject.NULL
        )
        .put("voltage_mv", intent?.getIntExtra(BatteryManager.EXTRA_VOLTAGE, -1)?.takeIf { it >= 0 } ?: JSONObject.NULL)
}

private fun batteryStatusName(status: Int): String {
    return when (status) {
        BatteryManager.BATTERY_STATUS_CHARGING -> "charging"
        BatteryManager.BATTERY_STATUS_DISCHARGING -> "discharging"
        BatteryManager.BATTERY_STATUS_FULL -> "full"
        BatteryManager.BATTERY_STATUS_NOT_CHARGING -> "not_charging"
        else -> "unknown"
    }
}

private fun batteryPluggedKind(plugged: Int): String {
    return when {
        plugged and BatteryManager.BATTERY_PLUGGED_USB != 0 -> "usb"
        plugged and BatteryManager.BATTERY_PLUGGED_AC != 0 -> "ac"
        plugged and BatteryManager.BATTERY_PLUGGED_WIRELESS != 0 -> "wireless"
        else -> "none"
    }
}

internal fun networkCapabilitiesJson(context: Context): JSONObject {
    val connectivity = context.getSystemService(ConnectivityManager::class.java)
    val network = connectivity.activeNetwork
    val caps = network?.let { connectivity.getNetworkCapabilities(it) }
    return JSONObject()
        .put("active", network != null)
        .put("internet", caps?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) ?: false)
        .put("validated", caps?.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED) ?: false)
        .put("not_metered", caps?.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) ?: false)
        .put("transports", JSONArray().apply {
            if (caps?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true) put("wifi")
            if (caps?.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) == true) put("cellular")
            if (caps?.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) == true) put("ethernet")
            if (caps?.hasTransport(NetworkCapabilities.TRANSPORT_VPN) == true) put("vpn")
        })
}

internal fun buildJson(): JSONObject {
    return JSONObject()
        .put("manufacturer", Build.MANUFACTURER)
        .put("brand", Build.BRAND)
        .put("model", Build.MODEL)
        .put("device", Build.DEVICE)
        .put("sdk_int", Build.VERSION.SDK_INT)
        .put("release", Build.VERSION.RELEASE)
        .put("supported_abis", JSONArray().apply { Build.SUPPORTED_ABIS.forEach { put(it) } })
}

internal fun backgroundDebugStatusJson(context: Context): JSONObject {
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    val appForegroundRecorded = prefs.getBoolean(TaskWorkService.PREF_APP_IN_FOREGROUND, false)
    val processState = processStateJson()
    val appForeground = processState.optBoolean("foreground", appForegroundRecorded)
    val keepalive = McpDebugKeepAliveService.statusJson(context)
    val notificationPermission = notificationPermissionJson(context)
    val batteryOptimization = batteryOptimizationJson(context)
    val network = networkCapabilitiesJson(context)
    val keepaliveActive = keepalive.optBoolean("active", false)
    val caveats = JSONArray()
    val recommendations = JSONArray()

    fun warn(message: String, recommendation: String) {
        caveats.put(message)
        recommendations.put(recommendation)
    }

    if (!appForeground && !keepaliveActive) {
        warn(
            "App is backgrounded and MCP debug keepalive is not active.",
            "Call debug_keepalive with action=start before switching to another app."
        )
    }
    if (!notificationPermission.optBoolean("granted", true)) {
        warn(
            "Notification permission is denied, so the user may not see foreground debug/task status.",
            "Open the APK once and allow notifications for clearer background debugging."
        )
    }
    if (!batteryOptimization.optBoolean("ignoring", true)) {
        warn(
            "Battery optimization is still enabled for this APK.",
            "Ask the user to allow unrestricted/background battery usage if MCP becomes unreachable after long idle or lock screen."
        )
    }
    if (batteryOptimization.optBoolean("power_save_mode", false)) {
        warn(
            "System power save mode is active.",
            "Disable power save mode while collecting timing traces for more stable background behavior."
        )
    }
    if (!network.optBoolean("active", false) || !network.optBoolean("internet", false)) {
        warn(
            "No active internet-capable network is reported by Android.",
            "Reconnect Wi-Fi/cellular before testing chat latency or backend reachability."
        )
    } else if (!network.optBoolean("validated", false)) {
        warn(
            "Android reports the active network is not validated.",
            "Use network_check to separate captive-portal/phone-network issues from backend issues."
        )
    }

    val backgroundReachable = appForeground || keepaliveActive
    val reachability = when {
        !backgroundReachable -> "foreground_only"
        caveats.length() > 0 -> "at_risk"
        else -> "ready"
    }

    return JSONObject()
        .put("app_foreground", appForeground)
        .put("app_foreground_recorded", appForegroundRecorded)
        .put("process_state", processState)
        .put("background_reachable", backgroundReachable)
        .put("reachability", reachability)
        .put("keepalive", keepalive)
        .put("notification_permission", notificationPermission)
        .put("battery_optimization", batteryOptimization)
        .put("network", network)
        .put("caveats", caveats)
        .put("recommendations", recommendations)
}
