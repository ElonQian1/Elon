package com.elon.app

import android.app.Service
import android.app.NotificationManager
import android.content.Intent
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import org.json.JSONObject

class TaskWorkService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private val prefs by lazy { AuthManager.userDataPrefs(this) }
    private val activeTasks = linkedMapOf<String, RunningTask>()
    private var nextClientGeneration = 0

    override fun onCreate() {
        super.onCreate()
        createTaskWorkNotificationChannels(this)
        restorePersistedTaskWork(prefs, activeTasks)
        if (hasActiveTasks()) {
            enterForeground("restore")
            connectAll()
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        val payload = intent?.getStringExtra(EXTRA_PAYLOAD)?.takeIf { it.isNotBlank() }
        val traceId = intent?.getStringExtra(EXTRA_TRACE_ID)?.takeIf { it.isNotBlank() }
            ?: taskPayloadTraceId(payload)
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
                restorePersistedTaskWork(prefs, activeTasks)
                val task = traceId?.let { activeTasks[it] }
                val tasksToResume = task?.let { listOf(it) } ?: activeTasks.values.toList()
                if (tasksToResume.isNotEmpty()) {
                    DebugTraceStore.record(
                        "task_resume_pending",
                        mapOf(
                            "trace_id" to traceId,
                            "task_count" to tasksToResume.size,
                            "pending_age_ms" to taskPendingWorkAgeMs(tasksToResume)
                        )
                    )
                    enterForeground("resume_pending")
                    tasksToResume.forEach {
                        it.payloadSentForCurrentConnection = false
                        connect(it)
                    }
                } else {
                    broadcastState()
                }
            }
            ACTION_CONNECT -> {
                if (hasActiveTasks()) enterForeground("connect")
                traceId?.let { activeTasks[it] }?.let { connect(it) } ?: connectAll()
            }
            ACTION_PAUSE -> pauseWork(traceId)
            ACTION_SYNC_STATE -> broadcastState()
            else -> {
                restorePersistedTaskWork(prefs, activeTasks)
                if (hasActiveTasks()) {
                    enterForeground("implicit_resume")
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
        val traceId = taskPayloadTraceId(payload) ?: "task_${System.currentTimeMillis()}"
        if (force) cleanupTask(traceId, disconnect = true)
        val task = activeTasks[traceId] ?: RunningTask(
            traceId = traceId,
            projectId = taskPayloadString(payload, "project_id"),
            conversationId = taskPayloadString(payload, "conversation_id"),
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
        persistTaskWork(prefs, activeTasks.values)
        enterForeground("start_work")
        connect(task)
    }

    private fun ensureClient(task: RunningTask): ElonWsClient {
        val serverUrl = taskProjectWsUrl(this, prefs, task.payload)
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
            },
            onAuthRequired = onAuthRequired@{
                val current = activeTasks[task.traceId] ?: return@onAuthRequired
                if (generation != current.clientGeneration) return@onAuthRequired
                DebugTraceStore.record(
                    "task_ws_auth_required",
                    mapOf("trace_id" to current.traceId, "server_url" to serverUrl)
                )
                // 停止重连，通知 UI 跳转登录
                cleanupTask(current.traceId, disconnect = false)
                broadcastStatus("auth_required", current)
                persistTaskWork(prefs, activeTasks.values)
                broadcastState()
                if (!hasActiveTasks()) {
                    stopForeground(STOP_FOREGROUND_REMOVE)
                    stopSelf()
                }
            }
        )
        task.wsClient = created
        return created
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
        if (isTaskAppInForeground(prefs)) {
            broadcastMessage(task, raw)
        } else {
            queueTaskRawEvent(prefs, task, raw)
        }

        val parsed = runCatching { JSONObject(raw) }.getOrNull()
        val messageType = parsed?.optString("type")?.takeIf { it.isNotBlank() }
        val messagePreview = parsed?.let { taskJsonStringOrNull(it, "message") }?.let { taskTextPreview(it) }
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
        if (task.firstChatReplyAtMs == 0L && isTaskChatReplyType(messageType)) {
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
                if (!isTaskAppInForeground(prefs)) {
                    showAppUpdateNotification(this, parsed)
                }
            }
            "progress" -> {
                val step = parsed?.optInt("step_current", 0) ?: 0
                val total = parsed?.optInt("step_total", 0) ?: 0
                val phase = parsed?.optString("phase")?.takeIf { it.isNotBlank() }
                if (step > 0 && total > 0 && phase != null && !isTaskAppInForeground(prefs)) {
                    task.lastStep = step
                    task.lastStepTotal = total
                    if (task.lastPhaseStartMs == 0L) task.lastPhaseStartMs = System.currentTimeMillis()
                    val etaText = estimateTaskEta(task)
                    val notif = updateProgressNotification(this, step, total, phase, etaText)
                    getSystemService(NotificationManager::class.java)
                        .notify(ACTIVE_WORK_NOTIFICATION_ID, notif)
                }
            }
            "done" -> finishWork(task.traceId, parsed, success = true)
            "error" -> finishWork(task.traceId, parsed, success = false)
        }
    }

    private fun finishWork(traceId: String, json: JSONObject, success: Boolean) {
        val task = activeTasks[traceId] ?: return
        val apkUrl = taskJsonStringOrNull(json, "apk_url")
        val messagePreview = taskJsonStringOrNull(json, "message")?.let { taskTextPreview(it) }
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
        persistTaskWork(prefs, activeTasks.values)
        if (!isTaskAppInForeground(prefs)) {
            notifyBackgroundTaskCompleted(
                context = this,
                prefs = prefs,
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
        persistTaskWork(prefs, activeTasks.values)
        broadcastState()
        if (!hasActiveTasks()) {
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        }
    }

    private fun enterForeground(reason: String) {
        DebugTraceStore.record(
            "task_foreground_entered",
            mapOf("reason" to reason, "active_task_count" to activeTasks.size)
        )
        startForeground(ACTIVE_WORK_NOTIFICATION_ID, activeTaskNotification(this))
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
            putExtra(EXTRA_ACTIVE_TASKS, taskActiveTasksJson(activeTasks.values).toString())
        }
        sendBroadcast(intent)
    }

    private fun Intent.putTaskExtras(task: RunningTask) {
        putExtra(EXTRA_TRACE_ID, task.traceId)
        task.projectId?.let { putExtra(EXTRA_PROJECT_ID, it) }
        task.conversationId?.let { putExtra(EXTRA_CONVERSATION_ID, it) }
        putExtra(EXTRA_IS_DEVELOPMENT, task.isDevelopment)
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
    }
}
