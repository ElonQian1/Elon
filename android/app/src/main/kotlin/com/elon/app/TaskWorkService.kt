package com.elon.app

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder

class TaskWorkService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private val prefs by lazy { getSharedPreferences("elon", MODE_PRIVATE) }
    private var wsClient: ElonWsClient? = null
    private var pendingPayload: String? = null
    private var activeRequestIsDevelopment = true
    private var waitingForReply = false
    private var payloadSentForCurrentConnection = false
    private var reconnectAttempts = 0
    private var activeServerUrl: String? = null
    private var activeRequestStartedAtMs = 0L
    private var firstServerEventAtMs = 0L
    private var activeRequestKind = "unknown"

    override fun onCreate() {
        super.onCreate()
        createChannels()
        restorePendingWork()
        ensureClient()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START_WORK -> {
                val payload = intent.getStringExtra(EXTRA_PAYLOAD)?.takeIf { it.isNotBlank() }
                if (payload != null) {
                    startWork(
                        payload = payload,
                        isDevelopment = intent.getBooleanExtra(EXTRA_IS_DEVELOPMENT, true)
                    )
                }
            }
            ACTION_RESUME_PENDING -> {
                restorePendingWork()
                if (waitingForReply) {
                    enterForeground()
                    payloadSentForCurrentConnection = false
                    connect()
                }
            }
            ACTION_CONNECT -> {
                if (waitingForReply) enterForeground()
                connect()
            }
            ACTION_PAUSE -> pauseWork()
            ACTION_SYNC_STATE -> broadcastState()
            else -> {
                restorePendingWork()
                if (waitingForReply) {
                    enterForeground()
                    payloadSentForCurrentConnection = false
                    connect()
                } else {
                    broadcastState()
                }
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        handler.removeCallbacksAndMessages(null)
        wsClient?.disconnect()
        wsClient = null
        super.onDestroy()
    }

    private fun startWork(payload: String, isDevelopment: Boolean) {
        pendingPayload = payload
        activeRequestIsDevelopment = isDevelopment
        waitingForReply = true
        reconnectAttempts = 0
        payloadSentForCurrentConnection = false
        activeRequestStartedAtMs = System.currentTimeMillis()
        firstServerEventAtMs = 0L
        activeRequestKind = if (isDevelopment) "development" else "chat"
        Log.i(TAG, "request_start kind=$activeRequestKind payload_bytes=${payload.length}")
        DebugTraceStore.record(
            "task_start_work",
            mapOf(
                "trace_id" to extractTraceId(payload),
                "kind" to activeRequestKind,
                "payload_bytes" to payload.toByteArray().size
            )
        )
        persistActiveWork()
        enterForeground()
        connect()
    }

    private fun ensureClient(): ElonWsClient {
        val serverUrl = buildProjectWsUrl(pendingPayload)
        val existing = wsClient
        if (existing != null && activeServerUrl == serverUrl) return existing
        existing?.disconnect()
        activeServerUrl = serverUrl
        val created = ElonWsClient(
            serverUrl = serverUrl,
            onMessage = { raw -> handleServerMessage(raw) },
            onConnected = {
                reconnectAttempts = 0
                broadcastStatus("connected")
                sendPendingPayloadIfNeeded()
            },
            onDisconnected = {
                payloadSentForCurrentConnection = false
                broadcastStatus("disconnected")
                if (waitingForReply) scheduleReconnect()
            }
        )
        wsClient = created
        return created
    }

    private fun buildProjectWsUrl(payload: String?): String {
        val json = payload
            ?.let { runCatching { JSONObject(it) }.getOrNull() }
        val userId = json
            ?.optString("user_id")
            ?.takeIf { it.isNotBlank() }
            ?: prefs.getString(PREF_USER_ID, null)
            ?: "default"
        val projectId = json
            ?.optString("project_id")
            ?.takeIf { it.isNotBlank() }
            ?: prefs.getString(PREF_ACTIVE_PROJECT_ID, null)
            ?: "elon-self"
        val projectTitle = json
            ?.optString("project_title")
            ?.takeIf { it.isNotBlank() }
        val query = mutableListOf("app_version_code=${BuildConfig.VERSION_CODE}")
        projectTitle?.let { query += "title=${pathPart(it)}" }
        return "ws://43.139.149.158:8080/ws/user/${pathPart(userId)}/projects/${pathPart(projectId)}?${query.joinToString("&")}"
    }

    private fun pathPart(value: String): String {
        return URLEncoder.encode(value, "UTF-8").replace("+", "%20")
    }

    private fun connect() {
        ensureClient().connect()
        broadcastState()
    }

    private fun sendPendingPayloadIfNeeded() {
        val payload = pendingPayload ?: return
        if (!waitingForReply || payloadSentForCurrentConnection) return
        val sent = ensureClient().send(payload)
        payloadSentForCurrentConnection = sent
        if (sent) {
            Log.i(
                TAG,
                "request_sent kind=$activeRequestKind elapsed_ms=${elapsedSinceRequestStart()}"
            )
        }
        DebugTraceStore.record(
            if (sent) "task_payload_sent" else "task_payload_send_failed",
            mapOf(
                "trace_id" to extractTraceId(payload),
                "kind" to activeRequestKind,
                "elapsed_ms" to elapsedSinceRequestStart(),
                "payload_bytes" to payload.toByteArray().size
            )
        )
        if (!sent) scheduleReconnect()
    }

    private fun scheduleReconnect() {
        if (!waitingForReply) return
        reconnectAttempts += 1
        val delay = (900L * reconnectAttempts).coerceAtMost(6_000L)
        handler.removeCallbacksAndMessages(RECONNECT_TOKEN)
        handler.postAtTime(
            {
                if (!waitingForReply) return@postAtTime
                payloadSentForCurrentConnection = false
                ensureClient().connect()
            },
            RECONNECT_TOKEN,
            android.os.SystemClock.uptimeMillis() + delay
        )
    }

    private fun handleServerMessage(raw: String) {
        if (isAppInForeground()) {
            broadcastMessage(raw)
        } else {
            queueRawEvent(raw)
        }

        val parsed = runCatching { JSONObject(raw) }.getOrNull()
        DebugTraceStore.record(
            "task_server_message",
            mapOf(
                "trace_id" to extractTraceId(pendingPayload),
                "kind" to activeRequestKind,
                "type" to parsed?.optString("type")?.takeIf { it.isNotBlank() },
                "elapsed_ms" to elapsedSinceRequestStart(),
                "bytes" to raw.toByteArray().size
            )
        )
        if (firstServerEventAtMs == 0L) {
            firstServerEventAtMs = System.currentTimeMillis()
            Log.i(
                TAG,
                "first_server_event kind=$activeRequestKind type=${parsed?.optString("type").orEmpty()} elapsed_ms=${elapsedSinceRequestStart()}"
            )
            DebugTraceStore.record(
                "task_first_server_event",
                mapOf(
                    "trace_id" to extractTraceId(pendingPayload),
                    "kind" to activeRequestKind,
                    "type" to parsed?.optString("type")?.takeIf { it.isNotBlank() },
                    "elapsed_ms" to elapsedSinceRequestStart()
                )
            )
        }
        when (parsed?.optString("type")) {
            "app_update_available" -> {
                if (!isAppInForeground()) {
                    showAppUpdateNotification(parsed)
                }
            }
            "done" -> finishWork(parsed, success = true)
            "error" -> finishWork(parsed, success = false)
        }
    }

    private fun finishWork(json: JSONObject, success: Boolean) {
        Log.i(
            TAG,
            "request_finish kind=$activeRequestKind success=$success type=${json.optString("type")} elapsed_ms=${elapsedSinceRequestStart()}"
        )
        DebugTraceStore.record(
            if (success) "task_finish_done" else "task_finish_error",
            mapOf(
                "trace_id" to extractTraceId(pendingPayload),
                "kind" to activeRequestKind,
                "elapsed_ms" to elapsedSinceRequestStart(),
                "has_apk_url" to json.optString("apk_url").isNotBlank()
            )
        )
        waitingForReply = false
        pendingPayload = null
        payloadSentForCurrentConnection = false
        reconnectAttempts = 0
        clearPersistedActiveWork()
        handler.removeCallbacksAndMessages(RECONNECT_TOKEN)
        if (!isAppInForeground()) {
            notifyBackgroundTaskCompleted(
                wasDevelopment = activeRequestIsDevelopment,
                apkUrl = json.optString("apk_url").takeIf { it.isNotBlank() },
                success = success
            )
        }
        activeRequestIsDevelopment = false
        stopForeground(STOP_FOREGROUND_REMOVE)
        broadcastState()
        stopSelf()
    }

    private fun elapsedSinceRequestStart(): Long {
        if (activeRequestStartedAtMs <= 0L) return 0L
        return System.currentTimeMillis() - activeRequestStartedAtMs
    }

    private fun extractTraceId(payload: String?): String? {
        return payload
            ?.let { runCatching { JSONObject(it).optString("trace_id") }.getOrNull() }
            ?.takeIf { it.isNotBlank() }
    }

    private fun pauseWork() {
        DebugTraceStore.record("task_pause", mapOf("trace_id" to extractTraceId(pendingPayload)))
        waitingForReply = false
        pendingPayload = null
        payloadSentForCurrentConnection = false
        reconnectAttempts = 0
        activeRequestIsDevelopment = false
        clearPersistedActiveWork()
        handler.removeCallbacksAndMessages(RECONNECT_TOKEN)
        wsClient?.disconnect()
        stopForeground(STOP_FOREGROUND_REMOVE)
        broadcastStatus("paused")
        stopSelf()
    }

    private fun enterForeground() {
        startForeground(ACTIVE_WORK_NOTIFICATION_ID, buildActiveNotification())
    }

    private fun buildActiveNotification() =
        NotificationCompat.Builder(this, ACTIVE_WORK_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle("一龙正在处理任务")
            .setContentText("切到其他应用时，任务会继续在后台运行。")
            .setContentIntent(mainActivityPendingIntent())
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()

    private fun notifyBackgroundTaskCompleted(wasDevelopment: Boolean, apkUrl: String?, success: Boolean) {
        val count = prefs.getInt(PREF_COMPLETED_TASK_BADGE_COUNT, 0).coerceAtLeast(0) + 1
        prefs.edit().putInt(PREF_COMPLETED_TASK_BADGE_COUNT, count).apply()
        updateLauncherBadgeCount(count)
        showTaskCompletedNotification(count, wasDevelopment, apkUrl, success)
    }

    private fun showTaskCompletedNotification(
        count: Int,
        wasDevelopment: Boolean,
        apkUrl: String?,
        success: Boolean
    ) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        val title = when {
            !success -> "任务需要处理"
            wasDevelopment -> "开发任务已完成"
            else -> "任务已完成"
        }
        val text = if (apkUrl != null) {
            "已有 $count 个任务完成，APK 可以下载测试。"
        } else {
            "已有 $count 个任务完成，点击查看结果。"
        }
        val notification = NotificationCompat.Builder(this, TASK_COMPLETE_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle(title)
            .setContentText(text)
            .setNumber(count)
            .setBadgeIconType(NotificationCompat.BADGE_ICON_SMALL)
            .setContentIntent(mainActivityPendingIntent())
            .setAutoCancel(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()
        runCatching {
            NotificationManagerCompat.from(this).notify(TASK_COMPLETE_NOTIFICATION_ID, notification)
        }
    }

    private fun mainActivityPendingIntent(): PendingIntent {
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        return PendingIntent.getActivity(
            this,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    private fun showAppUpdateNotification(json: JSONObject) {
        val versionCode = json.optInt("versionCode", 0)
        if (versionCode <= BuildConfig.VERSION_CODE) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        val versionName = json.optString("versionName").takeIf { it.isNotBlank() } ?: "新版"
        val changelog = json.optString("changelog").takeIf { it.isNotBlank() }
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
            putExtra(EXTRA_SHOW_APP_UPDATE, true)
        }
        val pendingIntent = PendingIntent.getActivity(
            this,
            2,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val notification = NotificationCompat.Builder(this, APP_UPDATE_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle("一龙有新版本 v$versionName")
            .setContentText(changelog ?: "点击查看并安装更新")
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()
        runCatching {
            NotificationManagerCompat.from(this).notify(APP_UPDATE_NOTIFICATION_ID, notification)
        }
    }

    private fun createChannels() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.createNotificationChannel(
            NotificationChannel(
                ACTIVE_WORK_CHANNEL_ID,
                "后台任务运行",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "任务执行中保持后台连接"
                setShowBadge(false)
            }
        )
        notificationManager.createNotificationChannel(
            NotificationChannel(
                TASK_COMPLETE_CHANNEL_ID,
                "任务完成提醒",
                NotificationManager.IMPORTANCE_DEFAULT
            ).apply {
                description = "后台任务完成后显示桌面角标"
                setShowBadge(true)
            }
        )
        notificationManager.createNotificationChannel(
            NotificationChannel(
                APP_UPDATE_CHANNEL_ID,
                "应用更新提醒",
                NotificationManager.IMPORTANCE_DEFAULT
            ).apply {
                description = "一龙 APP 有新版本时提醒"
                setShowBadge(true)
            }
        )
    }

    private fun broadcastMessage(raw: String) {
        broadcastStatus("message") {
            putExtra(EXTRA_RAW_MESSAGE, raw)
        }
    }

    private fun broadcastStatus(kind: String, extras: (Intent.() -> Unit)? = null) {
        val intent = Intent(ACTION_EVENT).apply {
            setPackage(packageName)
            putExtra(EXTRA_KIND, kind)
            putExtra(EXTRA_CONNECTED, wsClient?.isConnected() == true)
            putExtra(EXTRA_WAITING, waitingForReply)
            extras?.invoke(this)
        }
        sendBroadcast(intent)
    }

    private fun broadcastState() {
        val intent = Intent(ACTION_STATE).apply {
            setPackage(packageName)
            putExtra(EXTRA_CONNECTED, wsClient?.isConnected() == true)
            putExtra(EXTRA_WAITING, waitingForReply)
        }
        sendBroadcast(intent)
    }

    private fun queueRawEvent(raw: String) {
        val queue = runCatching {
            JSONArray(prefs.getString(PREF_QUEUED_TASK_EVENTS, "[]"))
        }.getOrElse { JSONArray() }
        queue.put(raw.take(MAX_QUEUED_EVENT_LENGTH))
        while (queue.length() > MAX_QUEUED_EVENTS) {
            queue.remove(0)
        }
        prefs.edit().putString(PREF_QUEUED_TASK_EVENTS, queue.toString()).apply()
    }

    private fun isAppInForeground(): Boolean {
        return prefs.getBoolean(PREF_APP_IN_FOREGROUND, false)
    }

    private fun persistActiveWork() {
        val payload = pendingPayload
        if (!waitingForReply || payload.isNullOrBlank()) {
            clearPersistedActiveWork()
            return
        }
        prefs.edit()
            .putString(PREF_PENDING_WORK_PAYLOAD, payload)
            .putBoolean(PREF_PENDING_WORK_IS_DEVELOPMENT, activeRequestIsDevelopment)
            .putLong(PREF_PENDING_WORK_TIME, System.currentTimeMillis())
            .apply()
    }

    private fun clearPersistedActiveWork() {
        prefs.edit()
            .remove(PREF_PENDING_WORK_PAYLOAD)
            .remove(PREF_PENDING_WORK_IS_DEVELOPMENT)
            .remove(PREF_PENDING_WORK_TIME)
            .apply()
    }

    private fun restorePendingWork() {
        val payload = prefs.getString(PREF_PENDING_WORK_PAYLOAD, null)?.takeIf { it.isNotBlank() }
        if (payload == null) {
            waitingForReply = false
            pendingPayload = null
            return
        }
        val savedAt = prefs.getLong(PREF_PENDING_WORK_TIME, 0L)
        val tooOld = savedAt <= 0L || System.currentTimeMillis() - savedAt > PENDING_WORK_TTL_MS
        if (tooOld) {
            clearPersistedActiveWork()
            waitingForReply = false
            pendingPayload = null
            return
        }
        waitingForReply = true
        pendingPayload = payload
        activeRequestIsDevelopment = prefs.getBoolean(PREF_PENDING_WORK_IS_DEVELOPMENT, true)
    }

    private fun updateLauncherBadgeCount(count: Int) {
        val badge = count.coerceAtLeast(0)
        val payload = Bundle().apply {
            putString("package", packageName)
            putString("class", MainActivity::class.java.name)
            putInt("badgenumber", badge)
        }
        listOf(
            "content://com.huawei.android.launcher.settings/badge/",
            "content://com.hihonor.android.launcher.settings/badge/"
        ).forEach { badgeUri ->
            runCatching {
                contentResolver.call(Uri.parse(badgeUri), "change_badge", null, payload)
            }
        }
    }

    companion object {
        private const val TAG = "ElonTaskWork"
        const val ACTION_START_WORK = "com.elon.app.task.START_WORK"
        const val ACTION_RESUME_PENDING = "com.elon.app.task.RESUME_PENDING"
        const val ACTION_CONNECT = "com.elon.app.task.CONNECT"
        const val ACTION_PAUSE = "com.elon.app.task.PAUSE"
        const val ACTION_SYNC_STATE = "com.elon.app.task.SYNC_STATE"
        const val ACTION_EVENT = "com.elon.app.task.EVENT"
        const val ACTION_STATE = "com.elon.app.task.STATE"

        const val EXTRA_PAYLOAD = "payload"
        const val EXTRA_IS_DEVELOPMENT = "is_development"
        const val EXTRA_KIND = "kind"
        const val EXTRA_RAW_MESSAGE = "raw_message"
        const val EXTRA_CONNECTED = "connected"
        const val EXTRA_WAITING = "waiting"

        const val PREF_PENDING_WORK_PAYLOAD = "pending_work_payload"
        const val PREF_PENDING_WORK_IS_DEVELOPMENT = "pending_work_is_development"
        const val PREF_PENDING_WORK_TIME = "pending_work_time"
        const val PREF_ACTIVE_PROJECT_ID = "active_project_id"
        const val PREF_USER_ID = "user_id"
        const val PREF_COMPLETED_TASK_BADGE_COUNT = "completed_task_badge_count"
        const val PREF_APP_IN_FOREGROUND = "app_in_foreground"
        const val PREF_QUEUED_TASK_EVENTS = "queued_task_events"

        const val ACTIVE_WORK_CHANNEL_ID = "active_task_work"
        const val TASK_COMPLETE_CHANNEL_ID = "task_complete_alerts"
        const val APP_UPDATE_CHANNEL_ID = "app_update_alerts"
        const val ACTIVE_WORK_NOTIFICATION_ID = 2400
        const val TASK_COMPLETE_NOTIFICATION_ID = 2401
        const val APP_UPDATE_NOTIFICATION_ID = 2402
        const val EXTRA_SHOW_APP_UPDATE = "show_app_update"
        const val PENDING_WORK_TTL_MS = 6 * 60 * 60 * 1000L
        private const val MAX_QUEUED_EVENTS = 120
        private const val MAX_QUEUED_EVENT_LENGTH = 20_000
        private val RECONNECT_TOKEN = Any()
    }
}
