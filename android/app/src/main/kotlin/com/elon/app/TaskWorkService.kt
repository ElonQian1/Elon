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
    private val activeTasks = linkedMapOf<String, RunningTask>()
    private var nextClientGeneration = 0

    private data class RunningTask(
        val traceId: String,
        val projectId: String?,
        val conversationId: String?,
        var payload: String,
        var isDevelopment: Boolean,
        var waitingForReply: Boolean = true,
        var payloadSentForCurrentConnection: Boolean = false,
        var reconnectAttempts: Int = 0,
        var serverUrl: String? = null,
        var clientGeneration: Int = 0,
        var startedAtMs: Long = System.currentTimeMillis(),
        var firstServerEventAtMs: Long = 0L,
        var firstChatReplyAtMs: Long = 0L,
        var wsClient: ElonWsClient? = null,
        var reconnectRunnable: Runnable? = null
    ) {
        val requestKind: String
            get() = if (isDevelopment) "development" else "chat"
    }

    override fun onCreate() {
        super.onCreate()
        createChannels()
        restorePendingWork()
        if (hasActiveTasks()) connectAll()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        val payload = intent?.getStringExtra(EXTRA_PAYLOAD)?.takeIf { it.isNotBlank() }
        val traceId = intent?.getStringExtra(EXTRA_TRACE_ID)?.takeIf { it.isNotBlank() }
            ?: extractTraceId(payload)
        DebugTraceStore.record(
            "task_service_command",
            mapOf(
                "action" to action,
                "start_id" to startId,
                "flags" to flags,
                "trace_id" to traceId,
                "has_payload" to (payload != null),
                "force" to (intent?.getBooleanExtra(EXTRA_FORCE_START, false) ?: false),
                "active_task_count" to activeTasks.size,
                "active_trace_ids" to activeTasks.keys.joinToString(",")
            )
        )
        when (action) {
            ACTION_START_WORK -> {
                if (payload != null) {
                    startWork(
                        payload = payload,
                        isDevelopment = intent.getBooleanExtra(EXTRA_IS_DEVELOPMENT, true),
                        force = intent.getBooleanExtra(EXTRA_FORCE_START, false)
                    )
                } else {
                    DebugTraceStore.record(
                        "task_start_missing_payload",
                        mapOf("action" to action, "start_id" to startId)
                    )
                }
            }
            ACTION_RESUME_PENDING -> {
                restorePendingWork()
                val task = traceId?.let { activeTasks[it] }
                val tasksToResume = task?.let { listOf(it) } ?: activeTasks.values.toList()
                if (tasksToResume.isNotEmpty()) {
                    DebugTraceStore.record(
                        "task_resume_pending",
                        mapOf(
                            "trace_id" to traceId,
                            "task_count" to tasksToResume.size,
                            "pending_age_ms" to pendingWorkAgeMs(tasksToResume)
                        )
                    )
                    enterForeground()
                    tasksToResume.forEach {
                        it.payloadSentForCurrentConnection = false
                        connect(it)
                    }
                } else {
                    broadcastState()
                }
            }
            ACTION_CONNECT -> {
                if (hasActiveTasks()) enterForeground()
                traceId?.let { activeTasks[it] }?.let { connect(it) } ?: connectAll()
            }
            ACTION_PAUSE -> pauseWork(traceId)
            ACTION_SYNC_STATE -> broadcastState()
            else -> {
                restorePendingWork()
                if (hasActiveTasks()) {
                    enterForeground()
                    activeTasks.values.forEach { it.payloadSentForCurrentConnection = false }
                    connectAll()
                } else {
                    broadcastState()
                }
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        handler.removeCallbacksAndMessages(null)
        activeTasks.values.forEach { it.wsClient?.disconnect() }
        activeTasks.clear()
        super.onDestroy()
    }

    private fun startWork(payload: String, isDevelopment: Boolean, force: Boolean = false) {
        val traceId = extractTraceId(payload) ?: "task_${System.currentTimeMillis()}"
        if (force) cleanupTask(traceId, disconnect = true)
        val task = activeTasks[traceId] ?: RunningTask(
            traceId = traceId,
            projectId = extractPayloadString(payload, "project_id"),
            conversationId = extractPayloadString(payload, "conversation_id"),
            payload = payload,
            isDevelopment = isDevelopment
        ).also { activeTasks[traceId] = it }
        task.payload = payload
        task.isDevelopment = isDevelopment
        task.waitingForReply = true
        task.reconnectAttempts = 0
        task.payloadSentForCurrentConnection = false
        task.startedAtMs = System.currentTimeMillis()
        task.firstServerEventAtMs = 0L
        task.firstChatReplyAtMs = 0L
        Log.i(TAG, "request_start trace=$traceId kind=${task.requestKind} payload_bytes=${payload.length}")
        DebugTraceStore.record(
            "task_start_work",
            mapOf(
                "trace_id" to traceId,
                "project_id" to task.projectId,
                "conversation_id" to task.conversationId,
                "kind" to task.requestKind,
                "force" to force,
                "parallel_task_count" to activeTasks.size,
                "payload_bytes" to payload.toByteArray().size
            )
        )
        persistActiveWork()
        enterForeground()
        connect(task)
    }

    private fun ensureClient(task: RunningTask): ElonWsClient {
        val serverUrl = buildProjectWsUrl(task.payload)
        val existing = task.wsClient
        if (existing != null && task.serverUrl == serverUrl) return existing
        nextClientGeneration += 1
        val generation = nextClientGeneration
        existing?.disconnect()
        task.serverUrl = serverUrl
        task.clientGeneration = generation
        val created = ElonWsClient(
            serverUrl = serverUrl,
            onMessage = { raw -> handleServerMessage(task.traceId, raw) },
            onConnected = onConnected@{
                val current = activeTasks[task.traceId] ?: return@onConnected
                if (generation != current.clientGeneration) {
                    DebugTraceStore.record(
                        "task_ws_stale_connected_ignored",
                        mapOf("trace_id" to task.traceId, "server_url" to serverUrl)
                    )
                    return@onConnected
                }
                DebugTraceStore.record(
                    "task_ws_connected",
                    mapOf(
                        "trace_id" to current.traceId,
                        "kind" to current.requestKind,
                        "elapsed_ms" to elapsedSinceRequestStart(current),
                        "reconnect_attempts" to current.reconnectAttempts,
                        "server_url" to current.serverUrl
                    )
                )
                current.reconnectAttempts = 0
                broadcastStatus("connected", current)
                sendPendingPayloadIfNeeded(current)
            },
            onDisconnected = onDisconnected@{
                val current = activeTasks[task.traceId] ?: return@onDisconnected
                if (generation != current.clientGeneration) {
                    DebugTraceStore.record(
                        "task_ws_stale_disconnect_ignored",
                        mapOf(
                            "trace_id" to current.traceId,
                            "kind" to current.requestKind,
                            "server_url" to serverUrl
                        )
                    )
                    return@onDisconnected
                }
                DebugTraceStore.record(
                    "task_ws_disconnected",
                    mapOf(
                        "trace_id" to current.traceId,
                        "kind" to current.requestKind,
                        "elapsed_ms" to elapsedSinceRequestStart(current),
                        "reconnect_attempts" to current.reconnectAttempts
                    )
                )
                current.payloadSentForCurrentConnection = false
                broadcastStatus("disconnected", current)
                if (current.waitingForReply) scheduleReconnect(current)
            }
        )
        task.wsClient = created
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

    private fun connectAll() {
        if (activeTasks.isEmpty()) {
            broadcastState()
            return
        }
        activeTasks.values.toList().forEach { connect(it) }
    }

    private fun connect(task: RunningTask) {
        val client = ensureClient(task)
        client.connect()
        if (client.isConnected()) {
            sendPendingPayloadIfNeeded(task)
        }
        broadcastState()
    }

    private fun sendPendingPayloadIfNeeded(task: RunningTask) {
        val payload = task.payload
        if (!task.waitingForReply || task.payloadSentForCurrentConnection) return
        val sent = ensureClient(task).send(payload)
        task.payloadSentForCurrentConnection = sent
        if (sent) {
            Log.i(
                TAG,
                "request_sent trace=${task.traceId} kind=${task.requestKind} elapsed_ms=${elapsedSinceRequestStart(task)}"
            )
        }
        DebugTraceStore.record(
            if (sent) "task_payload_sent" else "task_payload_send_failed",
            mapOf(
                "trace_id" to task.traceId,
                "kind" to task.requestKind,
                "elapsed_ms" to elapsedSinceRequestStart(task),
                "payload_bytes" to payload.toByteArray().size
            )
        )
        if (!sent) scheduleReconnect(task)
    }

    private fun scheduleReconnect(task: RunningTask) {
        if (!task.waitingForReply || activeTasks[task.traceId] == null) return
        task.reconnectAttempts += 1
        val delay = (900L * task.reconnectAttempts).coerceAtMost(6_000L)
        DebugTraceStore.record(
            "task_reconnect_scheduled",
            mapOf(
                "trace_id" to task.traceId,
                "kind" to task.requestKind,
                "elapsed_ms" to elapsedSinceRequestStart(task),
                "reconnect_attempts" to task.reconnectAttempts,
                "delay_ms" to delay
            )
        )
        task.reconnectRunnable?.let { handler.removeCallbacks(it) }
        val runnable = Runnable {
            val current = activeTasks[task.traceId] ?: return@Runnable
            if (!current.waitingForReply) return@Runnable
            current.payloadSentForCurrentConnection = false
            ensureClient(current).connect()
        }
        task.reconnectRunnable = runnable
        handler.postDelayed(runnable, delay)
    }

    private fun handleServerMessage(traceId: String, raw: String) {
        val task = activeTasks[traceId] ?: return
        if (isAppInForeground()) {
            broadcastMessage(task, raw)
        } else {
            queueRawEvent(task, raw)
        }

        val parsed = runCatching { JSONObject(raw) }.getOrNull()
        val messageType = parsed?.optString("type")?.takeIf { it.isNotBlank() }
        val messagePreview = parsed?.let { jsonStringOrNull(it, "message") }?.let { preview(it) }
        DebugTraceStore.record(
            "task_server_message",
            mapOf(
                "trace_id" to task.traceId,
                "kind" to task.requestKind,
                "project_id" to task.projectId,
                "conversation_id" to task.conversationId,
                "type" to messageType,
                "elapsed_ms" to elapsedSinceRequestStart(task),
                "bytes" to raw.toByteArray().size,
                "message_preview" to messagePreview
            )
        )
        if (task.firstServerEventAtMs == 0L && messageType != "app_update_available") {
            task.firstServerEventAtMs = System.currentTimeMillis()
            Log.i(
                TAG,
                "first_server_event trace=${task.traceId} kind=${task.requestKind} type=${messageType.orEmpty()} elapsed_ms=${elapsedSinceRequestStart(task)}"
            )
            DebugTraceStore.record(
                "task_first_server_event",
                mapOf(
                    "trace_id" to task.traceId,
                    "kind" to task.requestKind,
                    "type" to messageType,
                    "elapsed_ms" to elapsedSinceRequestStart(task)
                )
            )
        }
        if (task.firstChatReplyAtMs == 0L && isChatReplyType(messageType)) {
            task.firstChatReplyAtMs = System.currentTimeMillis()
            Log.i(
                TAG,
                "first_chat_reply trace=${task.traceId} kind=${task.requestKind} type=${messageType.orEmpty()} elapsed_ms=${elapsedSinceRequestStart(task)}"
            )
            DebugTraceStore.record(
                "task_first_chat_reply",
                mapOf(
                    "trace_id" to task.traceId,
                    "kind" to task.requestKind,
                    "type" to messageType,
                    "elapsed_ms" to elapsedSinceRequestStart(task),
                    "message_preview" to messagePreview
                )
            )
        }
        when (messageType) {
            "app_update_available" -> {
                if (!isAppInForeground()) {
                    showAppUpdateNotification(parsed)
                }
            }
            "done" -> finishWork(task.traceId, parsed, success = true)
            "error" -> finishWork(task.traceId, parsed, success = false)
        }
    }

    private fun finishWork(traceId: String, json: JSONObject, success: Boolean) {
        val task = activeTasks[traceId] ?: return
        val apkUrl = jsonStringOrNull(json, "apk_url")
        val messagePreview = jsonStringOrNull(json, "message")?.let { preview(it) }
        Log.i(
            TAG,
            "request_finish trace=${task.traceId} kind=${task.requestKind} success=$success type=${json.optString("type")} elapsed_ms=${elapsedSinceRequestStart(task)}"
        )
        DebugTraceStore.record(
            if (success) "task_finish_done" else "task_finish_error",
            mapOf(
                "trace_id" to task.traceId,
                "project_id" to task.projectId,
                "conversation_id" to task.conversationId,
                "kind" to task.requestKind,
                "elapsed_ms" to elapsedSinceRequestStart(task),
                "first_chat_reply_elapsed_ms" to firstChatReplyElapsedMs(task),
                "has_apk_url" to !apkUrl.isNullOrBlank(),
                "message_preview" to messagePreview
            )
        )
        cleanupTask(traceId, disconnect = true)
        persistActiveWork()
        if (!isAppInForeground()) {
            notifyBackgroundTaskCompleted(
                wasDevelopment = task.isDevelopment,
                apkUrl = apkUrl,
                success = success
            )
        }
        broadcastState()
        if (!hasActiveTasks()) {
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        }
    }

    private fun elapsedSinceRequestStart(task: RunningTask): Long {
        if (task.startedAtMs <= 0L) return 0L
        return System.currentTimeMillis() - task.startedAtMs
    }

    private fun firstChatReplyElapsedMs(task: RunningTask): Long? {
        if (task.startedAtMs <= 0L || task.firstChatReplyAtMs <= 0L) return null
        return task.firstChatReplyAtMs - task.startedAtMs
    }

    private fun extractTraceId(payload: String?): String? {
        return extractPayloadString(payload, "trace_id")
    }

    private fun extractPayloadString(payload: String?, key: String): String? {
        return payload
            ?.let { runCatching { JSONObject(it).optString(key) }.getOrNull() }
            ?.takeIf { it.isNotBlank() }
    }

    private fun isChatReplyType(type: String?): Boolean {
        return type in setOf("progress", "done", "error", "task_event", "message", "assistant_message")
    }

    private fun preview(value: String, maxChars: Int = 160): String {
        val singleLine = value.replace('\n', ' ').trim()
        return if (singleLine.length <= maxChars) {
            singleLine
        } else {
            singleLine.take(maxChars) + "..."
        }
    }

    private fun hasActiveTasks(): Boolean {
        return activeTasks.values.any { it.waitingForReply }
    }

    private fun cleanupTask(traceId: String, disconnect: Boolean) {
        val task = activeTasks.remove(traceId) ?: return
        task.waitingForReply = false
        task.reconnectRunnable?.let { handler.removeCallbacks(it) }
        task.reconnectRunnable = null
        if (disconnect) task.wsClient?.disconnect()
        task.wsClient = null
    }

    private fun pauseWork(traceId: String?) {
        val tasksToPause = traceId?.let { activeTasks[it]?.let(::listOf) } ?: activeTasks.values.toList()
        tasksToPause.forEach { task ->
            DebugTraceStore.record(
                "task_pause",
                mapOf("trace_id" to task.traceId, "conversation_id" to task.conversationId)
            )
            broadcastStatus("paused", task)
            cleanupTask(task.traceId, disconnect = true)
        }
        persistActiveWork()
        broadcastState()
        if (!hasActiveTasks()) {
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        }
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

    private fun broadcastMessage(task: RunningTask, raw: String) {
        broadcastStatus("message", task) {
            putExtra(EXTRA_RAW_MESSAGE, raw)
        }
    }

    private fun broadcastStatus(
        kind: String,
        task: RunningTask? = null,
        extras: (Intent.() -> Unit)? = null
    ) {
        val intent = Intent(ACTION_EVENT).apply {
            setPackage(packageName)
            putExtra(EXTRA_KIND, kind)
            putExtra(EXTRA_CONNECTED, activeTasks.values.any { it.wsClient?.isConnected() == true })
            putExtra(EXTRA_WAITING, hasActiveTasks())
            task?.let { putTaskExtras(it) }
            extras?.invoke(this)
        }
        sendBroadcast(intent)
    }

    private fun broadcastState() {
        val intent = Intent(ACTION_STATE).apply {
            setPackage(packageName)
            putExtra(EXTRA_CONNECTED, activeTasks.values.any { it.wsClient?.isConnected() == true })
            putExtra(EXTRA_WAITING, hasActiveTasks())
            putExtra(EXTRA_ACTIVE_TASKS, activeTasksJson().toString())
        }
        sendBroadcast(intent)
    }

    private fun Intent.putTaskExtras(task: RunningTask) {
        putExtra(EXTRA_TRACE_ID, task.traceId)
        task.projectId?.let { putExtra(EXTRA_PROJECT_ID, it) }
        task.conversationId?.let { putExtra(EXTRA_CONVERSATION_ID, it) }
        putExtra(EXTRA_IS_DEVELOPMENT, task.isDevelopment)
    }

    private fun activeTasksJson(): JSONArray {
        val array = JSONArray()
        activeTasks.values.forEach { task ->
            array.put(
                JSONObject()
                    .put("trace_id", task.traceId)
                    .put("project_id", task.projectId)
                    .put("conversation_id", task.conversationId)
                    .put("is_development", task.isDevelopment)
                    .put("started_at", task.startedAtMs)
            )
        }
        return array
    }

    private fun queueRawEvent(task: RunningTask, raw: String) {
        val queue = runCatching {
            JSONArray(prefs.getString(PREF_QUEUED_TASK_EVENTS, "[]"))
        }.getOrElse { JSONArray() }
        queue.put(
            JSONObject()
                .put("raw", raw.take(MAX_QUEUED_EVENT_LENGTH))
                .put("trace_id", task.traceId)
                .put("project_id", task.projectId)
                .put("conversation_id", task.conversationId)
                .put("is_development", task.isDevelopment)
        )
        while (queue.length() > MAX_QUEUED_EVENTS) {
            queue.remove(0)
        }
        prefs.edit().putString(PREF_QUEUED_TASK_EVENTS, queue.toString()).apply()
    }

    private fun isAppInForeground(): Boolean {
        return prefs.getBoolean(PREF_APP_IN_FOREGROUND, false)
    }

    private fun persistActiveWork() {
        val array = JSONArray()
        activeTasks.values
            .filter { it.waitingForReply && it.payload.isNotBlank() }
            .forEach { task ->
                array.put(
                    JSONObject()
                        .put("payload", task.payload)
                        .put("is_development", task.isDevelopment)
                        .put("started_at", task.startedAtMs)
                )
            }
        if (array.length() == 0) {
            clearPersistedActiveWork()
            return
        }
        prefs.edit()
            .putString(PREF_PENDING_WORK_TASKS, array.toString())
            .remove(PREF_PENDING_WORK_PAYLOAD)
            .remove(PREF_PENDING_WORK_IS_DEVELOPMENT)
            .remove(PREF_PENDING_WORK_TIME)
            .apply()
    }

    private fun clearPersistedActiveWork() {
        prefs.edit()
            .remove(PREF_PENDING_WORK_TASKS)
            .remove(PREF_PENDING_WORK_PAYLOAD)
            .remove(PREF_PENDING_WORK_IS_DEVELOPMENT)
            .remove(PREF_PENDING_WORK_TIME)
            .apply()
    }

    private fun restorePendingWork() {
        val restored = mutableListOf<RunningTask>()
        val now = System.currentTimeMillis()
        val tasksJson = prefs.getString(PREF_PENDING_WORK_TASKS, null)?.takeIf { it.isNotBlank() }
        if (tasksJson != null) {
            val array = runCatching { JSONArray(tasksJson) }.getOrNull()
            if (array != null) {
                for (index in 0 until array.length()) {
                    val item = array.optJSONObject(index) ?: continue
                    val payload = item.optString("payload").takeIf { it.isNotBlank() } ?: continue
                    val savedAt = item.optLong("started_at", now)
                    if (savedAt <= 0L || now - savedAt > PENDING_WORK_TTL_MS) continue
                    val traceId = extractTraceId(payload) ?: continue
                    if (activeTasks.containsKey(traceId)) continue
                    restored += RunningTask(
                        traceId = traceId,
                        projectId = extractPayloadString(payload, "project_id"),
                        conversationId = extractPayloadString(payload, "conversation_id"),
                        payload = payload,
                        isDevelopment = item.optBoolean("is_development", true),
                        startedAtMs = savedAt
                    )
                }
            }
        }

        if (tasksJson == null) {
            val payload = prefs.getString(PREF_PENDING_WORK_PAYLOAD, null)?.takeIf { it.isNotBlank() }
            val savedAt = prefs.getLong(PREF_PENDING_WORK_TIME, 0L)
            val tooOld = savedAt <= 0L || now - savedAt > PENDING_WORK_TTL_MS
            if (payload != null && !tooOld) {
                val traceId = extractTraceId(payload)
                if (traceId != null && !activeTasks.containsKey(traceId)) {
                    restored += RunningTask(
                        traceId = traceId,
                        projectId = extractPayloadString(payload, "project_id"),
                        conversationId = extractPayloadString(payload, "conversation_id"),
                        payload = payload,
                        isDevelopment = prefs.getBoolean(PREF_PENDING_WORK_IS_DEVELOPMENT, true),
                        startedAtMs = savedAt
                    )
                }
            }
        }

        restored.forEach { activeTasks[it.traceId] = it }
        persistActiveWork()
    }

    private fun pendingWorkAgeMs(tasks: List<RunningTask>): Long? {
        val oldest = tasks.map { it.startedAtMs }.filter { it > 0L }.minOrNull() ?: return null
        return System.currentTimeMillis() - oldest
    }

    private fun jsonStringOrNull(json: JSONObject, key: String): String? {
        if (!json.has(key) || json.isNull(key)) return null
        return json.optString(key)
            .trim()
            .takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
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
        const val EXTRA_FORCE_START = "force_start"
        const val EXTRA_KIND = "kind"
        const val EXTRA_RAW_MESSAGE = "raw_message"
        const val EXTRA_CONNECTED = "connected"
        const val EXTRA_WAITING = "waiting"
        const val EXTRA_TRACE_ID = "trace_id"
        const val EXTRA_PROJECT_ID = "project_id"
        const val EXTRA_CONVERSATION_ID = "conversation_id"
        const val EXTRA_ACTIVE_TASKS = "active_tasks"

        const val PREF_PENDING_WORK_TASKS = "pending_work_tasks"
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
        const val PENDING_WORK_TTL_MS = 24 * 60 * 60 * 1000L
        private const val MAX_QUEUED_EVENTS = 120
        private const val MAX_QUEUED_EVENT_LENGTH = 20_000
    }
}
