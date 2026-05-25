package com.elon.app

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
import android.os.SystemClock
import android.provider.Settings
import android.util.Log
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.URLEncoder
import java.net.URL
import java.util.Locale
import java.util.TimeZone
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong

object McpDebugServer {
    private const val TAG = "ElonMcpServer"
    private const val HOST = "127.0.0.1"
    private const val PORT = 8787
    private const val PROTOCOL_VERSION = "2025-06-18"
    private const val PREF_MCP_TOKEN = "mcp_debug_token"
    private const val SOCKET_TIMEOUT_MS = 5_000
    private const val DEFAULT_SERVER_BASE_URL = "http://43.139.149.158:8080"

    @Volatile private var running = false
    @Volatile private var serverSocket: ServerSocket? = null
    private val workers = Executors.newCachedThreadPool()
    private val processStartedElapsedMs = SystemClock.elapsedRealtime()
    private val activeConnections = AtomicInteger(0)
    private val totalRequests = AtomicLong(0)
    private val totalToolCalls = AtomicLong(0)
    private val failedRequests = AtomicLong(0)
    @Volatile private var lastRequestWallTimeMs = 0L
    @Volatile private var lastRequestDurationMs = 0L
    @Volatile private var lastHttpMethod: String? = null
    @Volatile private var lastPath: String? = null
    @Volatile private var lastRpcMethod: String? = null
    @Volatile private var lastToolName: String? = null
    @Volatile private var lastError: String? = null
    private lateinit var appContext: Context

    fun start(context: Context) {
        synchronized(this) {
            if (running) return
            appContext = context.applicationContext
            DebugTraceStore.init(appContext)
            running = true
            Thread(::serveLoop, "elon-mcp-debug-server").apply {
                isDaemon = true
                start()
            }
        }
    }

    fun stop() {
        running = false
        runCatching { serverSocket?.close() }
        serverSocket = null
    }

    private fun serveLoop() {
        debugToken()
        try {
            ServerSocket(PORT, 50, InetAddress.getByName(HOST)).use { server ->
                serverSocket = server
                DebugTraceStore.record(
                    "mcp_server_started",
                    mapOf("host" to HOST, "port" to PORT, "token_ready" to true)
                )
                Log.i(TAG, "MCP endpoint: adb forward tcp:$PORT tcp:$PORT then http://$HOST:$PORT/mcp")
                Log.i(TAG, "MCP debug token is ready")
                while (running) {
                    val socket = try {
                        server.accept()
                    } catch (_: Exception) {
                        if (running) Log.w(TAG, "accept failed")
                        break
                    }
                    socket.soTimeout = SOCKET_TIMEOUT_MS
                    socket.tcpNoDelay = true
                    workers.execute { handleSocket(socket) }
                }
            }
        } catch (error: Exception) {
            DebugTraceStore.record("mcp_server_failed", mapOf("error" to error.message))
            Log.e(TAG, "MCP server failed: ${error.message}", error)
        } finally {
            running = false
            serverSocket = null
        }
    }

    private fun handleSocket(socket: Socket) {
        val startedElapsedMs = SystemClock.elapsedRealtime()
        activeConnections.incrementAndGet()
        socket.use {
            try {
                val request = readRequest(it) ?: run {
                    recordHttpRequest("INVALID", "invalid", startedElapsedMs)
                    writeResponse(it, 400, "Bad Request", jsonError("bad_request", "Invalid HTTP request"))
                    return
                }
                when {
                    request.method == "GET" && request.path == "/health" -> {
                        writeResponse(it, 200, "OK", statusJson(includeToken = true).toString())
                    }
                    request.method == "GET" && request.path == "/mcp" -> {
                        writeResponse(it, 405, "Method Not Allowed", jsonError("method_not_allowed", "SSE GET is not implemented in this APK build"))
                    }
                    request.method == "POST" && request.path == "/mcp" -> {
                        handleMcpPost(it, request)
                    }
                    else -> writeResponse(it, 404, "Not Found", jsonError("not_found", "Unknown endpoint"))
                }
                recordHttpRequest(request.method, request.path, startedElapsedMs)
            } catch (error: Exception) {
                failedRequests.incrementAndGet()
                lastError = error.message ?: error.javaClass.simpleName
                DebugTraceStore.record(
                    "mcp_request_failed",
                    mapOf("error" to lastError, "duration_ms" to (SystemClock.elapsedRealtime() - startedElapsedMs))
                )
                runCatching {
                    writeResponse(
                        it,
                        500,
                        "Internal Server Error",
                        jsonError("internal_error", lastError ?: "MCP request failed")
                    )
                }
            } finally {
                activeConnections.decrementAndGet()
            }
        }
    }

    private fun handleMcpPost(socket: Socket, request: HttpRequest) {
        val rpc = runCatching { JSONObject(request.body) }.getOrNull()
        if (rpc == null || rpc.optString("jsonrpc") != "2.0") {
            writeResponse(socket, 400, "Bad Request", rpcError(null, -32600, "Invalid JSON-RPC request").toString())
            return
        }

        val method = rpc.optString("method")
        lastRpcMethod = method
        val id = if (rpc.has("id")) rpc.opt("id") else null
        if (id == null) {
            writeResponse(socket, 202, "Accepted", "")
            return
        }

        val response = when (method) {
            "initialize" -> rpcResult(id, mcpInitializeResult(PROTOCOL_VERSION, PORT, TAG))
            "ping" -> rpcResult(id, JSONObject())
            "tools/list" -> rpcResult(id, mcpToolsListResult())
            "tools/call" -> handleToolCall(id, rpc.optJSONObject("params") ?: JSONObject(), request.headers)
            else -> rpcError(id, -32601, "Method not found: $method")
        }
        writeResponse(socket, 200, "OK", response.toString())
    }

    private fun handleToolCall(id: Any, params: JSONObject, headers: Map<String, String>): JSONObject {
        val name = params.optString("name")
        val args = params.optJSONObject("arguments") ?: JSONObject()
        if (!authorized(headers, args)) {
            return rpcError(id, -32001, "Unauthorized MCP debug tool call")
        }
        totalToolCalls.incrementAndGet()
        lastToolName = name

        val result = when (name) {
            "phone_status" -> toolResult("Phone MCP debug server is running.", statusJson(includeToken = false))
            "trace_recent" -> mcpTraceRecent(args)
            "trace_clear" -> {
                DebugTraceStore.clear()
                toolResult("Trace buffer cleared.", JSONObject().put("cleared", true))
            }
            "debug_session" -> mcpDebugSession(appContext, args)
            "diagnostic_bundle" -> diagnosticBundle(args)
            "device_snapshot" -> deviceSnapshot(args)
            "network_check" -> networkCheck(args)
            "background_debug_status" -> backgroundDebugStatus(args)
            "latency_report" -> latencyReport(args)
            "server_trace" -> serverTrace(args)
            "mcp_self_check" -> mcpSelfCheck(args)
            "mcp_metrics" -> mcpMetrics(args)
            "debug_keepalive" -> mcpDebugKeepalive(appContext, args)
            "update_status" -> mcpUpdateStatus(args)
            "task_status" -> taskStatus(args)
            "task_control" -> taskControl(args)
            "task_events" -> taskEvents(args)
            "logcat_recent" -> mcpLogcatRecent(args)
            "chat_send" -> chatSend(args)
            "chat_probe" -> chatProbe(args)
            else -> toolResult("Unknown tool: $name", JSONObject().put("tool", name), isError = true)
        }
        return rpcResult(id, result)
    }

    private fun diagnosticBundle(args: JSONObject): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val session = if (args.optBoolean("start_session", false)) {
            startMcpDebugSession(
                prefs = prefs,
                requestedSessionId = args.optString("session_id").takeIf { it.isNotBlank() },
                note = args.optString("note").takeIf { it.isNotBlank() }
            )
        } else {
            mcpDebugSessionJson(prefs)
        }
        val sessionStartedAt = session.optLong("started_at_ms", 0L).takeIf { it > 0L }
        val sinceWallTimeMs = args.optLong("since_wall_time_ms", 0L)
            .takeIf { it > 0L }
            ?: sessionStartedAt
        val traceArgs = JSONObject()
            .put("limit", args.optInt("trace_limit", 80).coerceIn(1, 300))
            .put("since_wall_time_ms", sinceWallTimeMs ?: 0L)
        args.optString("trace_id").takeIf { it.isNotBlank() }?.let { traceArgs.put("trace_id", it) }
        args.optString("phase").takeIf { it.isNotBlank() }?.let { traceArgs.put("phase", it) }
        args.optString("contains").takeIf { it.isNotBlank() }?.let { traceArgs.put("contains", it) }

        val includeLogcat = if (args.has("include_logcat")) args.optBoolean("include_logcat") else true
        val includeNetworkCheck = if (args.has("include_network_check")) args.optBoolean("include_network_check") else true
        val includeUpdateCheck = args.optBoolean("include_update_check", false)
        val includeServerTrace = if (args.has("include_server_trace")) args.optBoolean("include_server_trace") else true
        val serverTraceLimit = args.optInt("server_trace_limit", 120).coerceIn(1, 300)
        val logcatArgs = JSONObject()
            .put("line_count", args.optInt("logcat_line_count", 240).coerceIn(20, 1_000))
            .put(
                "pattern",
                args.optString(
                    "logcat_pattern",
                    "ElonTrace|ElonTaskWork|ElonWsClient|ElonMcpServer|AndroidRuntime|FATAL EXCEPTION|ANR|ForegroundService"
                )
            )
        val selfCheck = mcpSelfCheck(JSONObject().put("include_update_check", includeUpdateCheck))
            .optJSONObject("structuredContent")
            ?: JSONObject()
        val trace = mcpTraceRecent(traceArgs).optJSONObject("structuredContent") ?: JSONObject()
        val taskEvents = taskEvents(JSONObject().put("limit", args.optInt("task_event_limit", 20).coerceIn(1, 120)))
            .optJSONObject("structuredContent")
            ?: JSONObject()
        val logcat = if (includeLogcat) {
            mcpLogcatRecent(logcatArgs).optJSONObject("structuredContent") ?: JSONObject()
        } else {
            JSONObject.NULL
        }
        val network = if (includeNetworkCheck) {
            networkCheck(JSONObject()).optJSONObject("structuredContent") ?: JSONObject()
        } else {
            JSONObject.NULL
        }
        val backgroundRuntime = backgroundDebugStatusJson(appContext)
        val taskStatusArgs = JSONObject()
        args.optString("trace_id").takeIf { it.isNotBlank() }?.let { taskStatusArgs.put("trace_id", it) }
        val taskStatus = taskStatusJson(taskStatusArgs)
        val bundleTraceId = args.optString("trace_id").takeIf { it.isNotBlank() }
            ?: taskStatus.optString("trace_id").takeIf { it.isNotBlank() && it != "null" }
            ?: mcpLatestTraceId(DebugTraceStore.recentEvents(300))
        val latencyArgs = JSONObject()
            .put("timeline_limit", args.optInt("timeline_limit", 80).coerceIn(1, 300))
        bundleTraceId?.let {
            latencyArgs.put("trace_id", it)
        }
        val latency = latencyReportJson(latencyArgs)
        val serverTrace = if (includeServerTrace) {
            JSONObject()
                .put("trace_id", bundleTraceId ?: "")
                .put("limit", serverTraceLimit)
                .apply {
                    args.optString("server_url").takeIf { it.isNotBlank() }?.let { put("server_url", it) }
                }
                .let { serverTraceJson(it) }
        } else {
            JSONObject.NULL
        }
        val assessment = mcpDiagnosticAssessmentJson(
            selfCheck = selfCheck,
            backgroundRuntime = backgroundRuntime,
            network = network,
            taskStatus = taskStatus,
            trace = trace,
            logcat = logcat,
            latency = latency,
            serverTrace = serverTrace
        )

        DebugTraceStore.record(
            "mcp_diagnostic_bundle",
            mapOf(
                "debug_session_id" to session.optString("session_id").takeIf { it.isNotBlank() },
                "include_logcat" to includeLogcat,
                "include_network_check" to includeNetworkCheck,
                "include_server_trace" to includeServerTrace,
                "assessment" to assessment.optString("severity"),
                "since_wall_time_ms" to sinceWallTimeMs
            )
        )
        val structured = JSONObject()
            .put("generated_at_ms", System.currentTimeMillis())
            .put("debug_session", session)
            .put("since_wall_time_ms", sinceWallTimeMs ?: JSONObject.NULL)
            .put("assessment", assessment)
            .put("status", statusJson(includeToken = false))
            .put("self_check", selfCheck)
            .put("background_debug", backgroundRuntime)
            .put("device_snapshot", deviceSnapshotJson())
            .put("network_check", network)
            .put("task_status", taskStatus)
            .put("latency_report", latency)
            .put("server_trace", serverTrace)
            .put("task_events", taskEvents)
            .put("trace_recent", trace)
            .put("logcat_recent", logcat)
            .put("metrics", metricsJson())
        return toolResult("Diagnostic bundle returned.", structured, isError = selfCheck.optBoolean("ready", true).not())
    }

    private fun deviceSnapshot(@Suppress("UNUSED_PARAMETER") args: JSONObject): JSONObject {
        return toolResult("Device snapshot returned.", deviceSnapshotJson())
    }

    private fun networkCheck(args: JSONObject): JSONObject {
        return toolResult("Network check returned.", mcpNetworkCheckJson(appContext, args, DEFAULT_SERVER_BASE_URL))
    }

    private fun backgroundDebugStatus(@Suppress("UNUSED_PARAMETER") args: JSONObject): JSONObject {
        return toolResult("Background debug status returned.", backgroundDebugStatusJson(appContext))
    }

    private fun latencyReport(args: JSONObject): JSONObject {
        return toolResult("Latency report returned.", latencyReportJson(args))
    }

    private fun serverTrace(args: JSONObject): JSONObject {
        val structured = serverTraceJson(args)
        val available = structured.optBoolean("available", false)
        return toolResult(
            if (available) "Server trace returned." else "Server trace is not available.",
            structured,
            isError = !available
        )
    }

    private fun mcpSelfCheck(args: JSONObject): JSONObject {
        val includeUpdateCheck = args.optBoolean("include_update_check", false)
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val status = statusJson(includeToken = false)
        val taskStatus = taskStatusJson(JSONObject())
        val keepalive = McpDebugKeepAliveService.statusJson(appContext)
        val backgroundRuntime = backgroundDebugStatusJson(appContext)
        val queuedEvents = queuedTaskEvents(prefs)
        val checks = JSONArray()
        fun addCheck(name: String, ok: Boolean, detail: String, critical: Boolean = true) {
            checks.put(
                JSONObject()
                    .put("name", name)
                    .put("ok", ok)
                    .put("critical", critical)
                    .put("detail", detail)
            )
        }

        val appForeground = status.optBoolean("app_foreground", false)
        val keepaliveActive = keepalive.optBoolean("active", false)
        addCheck("mcp_server_running", running, "MCP server thread is accepting local HTTP requests.")
        addCheck(
            "background_keepalive",
            appForeground || keepaliveActive,
            if (appForeground) "App is foreground; keepalive is optional." else "Foreground keepalive must be active while app is background."
        )
        addCheck("trace_buffer", DebugTraceStore.count() > 0, "Trace buffer has ${DebugTraceStore.count()} persisted events.", critical = false)
        addCheck(
            "request_health",
            lastError == null,
            lastError?.let { "Last MCP request error: $it" } ?: "No MCP request error recorded in this process."
        )
        addCheck(
            "task_queue",
            queuedEvents.size <= 120,
            "Queued background task events: ${queuedEvents.size}.",
            critical = false
        )
        addCheck(
            "notification_permission",
            backgroundRuntime.optJSONObject("notification_permission")?.optBoolean("granted", true) ?: true,
            "Notification permission affects whether the user can see background debug/task service notifications.",
            critical = false
        )
        addCheck(
            "battery_optimization",
            backgroundRuntime.optJSONObject("battery_optimization")?.optBoolean("ignoring", true) ?: true,
            "Battery optimization may let the system defer background work on some Android builds.",
            critical = false
        )
        addCheck(
            "network_validated",
            backgroundRuntime.optJSONObject("network")?.optBoolean("validated", true) ?: true,
            "Validated network helps distinguish phone network issues from server or APK issues.",
            critical = false
        )

        val recommendations = JSONArray()
        if (!appForeground && !keepaliveActive) {
            recommendations.put("Call debug_keepalive start, then keep the notification alive before switching apps.")
        }
        if (lastError != null) {
            recommendations.put("Call mcp_metrics and logcat_recent with pattern ElonMcpServer|ElonTrace to inspect the last request failure.")
        }
        if (taskStatus.optBoolean("busy", false)) {
            recommendations.put("Call task_status for timing, or task_control pause if the active phone task should be cancelled.")
        }
        if (queuedEvents.isNotEmpty()) {
            recommendations.put("Call task_events to inspect messages queued while the app UI was backgrounded.")
        }
        backgroundRuntime.optJSONArray("recommendations")?.let { runtimeRecommendations ->
            for (index in 0 until runtimeRecommendations.length()) {
                runtimeRecommendations.optString(index).takeIf { it.isNotBlank() }?.let { recommendations.put(it) }
            }
        }

        val updateStatus = if (includeUpdateCheck) {
            mcpUpdateStatus(JSONObject()).optJSONObject("structuredContent") ?: JSONObject.NULL
        } else {
            JSONObject.NULL
        }
        val ready = (0 until checks.length()).all { check ->
            val item = checks.optJSONObject(check) ?: return@all false
            !item.optBoolean("critical", true) || item.optBoolean("ok", false)
        }
        val structured = JSONObject()
            .put("ready", ready)
            .put("status", status)
            .put("task_status", taskStatus)
            .put("background_debug", backgroundRuntime)
            .put("metrics", metricsJson())
            .put("queued_task_event_count", queuedEvents.size)
            .put("checks", checks)
            .put("recommendations", recommendations)
            .put("update_status", updateStatus)
        return toolResult("MCP self-check returned.", structured, isError = !ready)
    }

    private fun mcpMetrics(@Suppress("UNUSED_PARAMETER") args: JSONObject): JSONObject {
        return toolResult("MCP metrics returned.", metricsJson())
    }

    private fun taskStatus(args: JSONObject): JSONObject {
        return toolResult("Task status returned.", taskStatusJson(args))
    }

    private fun taskControl(args: JSONObject): JSONObject {
        val action = args.optString("action", "status").lowercase(Locale.ROOT)
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        when (action) {
            "pause" -> {
                val activeTraceId = pendingTaskJson(prefs)?.optString("trace_id")?.takeIf { it.isNotBlank() }
                clearPersistedTask(prefs)
                val serviceIntent = Intent(appContext, TaskWorkService::class.java).apply {
                    this.action = TaskWorkService.ACTION_PAUSE
                }
                val serviceSignal = runCatching { appContext.startService(serviceIntent) }
                    .fold(onSuccess = { "pause_sent" }, onFailure = { "pause_start_failed:${it.javaClass.simpleName}" })
                val stopped = if (serviceSignal.startsWith("pause_start_failed")) {
                    runCatching { appContext.stopService(Intent(appContext, TaskWorkService::class.java)) }
                        .getOrDefault(false)
                } else {
                    false
                }
                DebugTraceStore.record(
                    "mcp_task_control",
                    mapOf(
                        "action" to action,
                        "active_trace_id" to activeTraceId,
                        "service_signal" to serviceSignal,
                        "stop_service" to stopped
                    )
                )
                return toolResult(
                    "Task pause requested.",
                    taskStatusJson(JSONObject())
                        .put("action", action)
                        .put("service_signal", serviceSignal)
                        .put("stop_service", stopped)
                )
            }
            "status" -> Unit
            else -> return toolResult(
                "Unsupported task control action: $action",
                JSONObject().put("action", action),
                isError = true
            )
        }
        return toolResult("Task status returned.", taskStatusJson(JSONObject()).put("action", action))
    }

    private fun taskEvents(args: JSONObject): JSONObject {
        val limit = args.optInt("limit", 40).coerceIn(1, 120)
        val clear = args.optBoolean("clear", false)
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val allEvents = queuedTaskEvents(prefs)
        val returned = allEvents.takeLast(limit)
        if (clear) {
            prefs.edit().remove(TaskWorkService.PREF_QUEUED_TASK_EVENTS).apply()
            DebugTraceStore.record("mcp_task_events_cleared", mapOf("cleared_count" to allEvents.size))
        }
        val structured = JSONObject()
            .put("events", JSONArray().apply { returned.forEach { put(rawTaskEventJson(it)) } })
            .put("returned_count", returned.size)
            .put("queued_count", allEvents.size)
            .put("limit", limit)
            .put("cleared", clear)
        return toolResult("Queued task events returned.", structured)
    }

    private fun chatSend(args: JSONObject): JSONObject {
        val message = args.optString("message").trim()
        if (message.isEmpty()) {
            return toolResult("message is required", JSONObject().put("field", "message"), isError = true)
        }
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val force = args.optBoolean("force", false)
        val userId = prefs.getString(TaskWorkService.PREF_USER_ID, null)
            ?: UUID.randomUUID().toString().replace("-", "").also {
                prefs.edit().putString(TaskWorkService.PREF_USER_ID, it).apply()
            }
        val projectId = args.optString("project_id").takeIf { it.isNotBlank() }
            ?: prefs.getString(TaskWorkService.PREF_ACTIVE_PROJECT_ID, null)
            ?: "elon-self"
        val projectTitle = args.optString("project_title").takeIf { it.isNotBlank() }
            ?: prefs.getString("project_title", null)
            ?: "Elon debug project"
        val traceId = args.optString("trace_id").takeIf { it.isNotBlank() }
            ?: "mcp_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}"
        val agent = args.optString("agent").takeIf { it.isNotBlank() }
        val conversationId = args.optString("conversation_id").takeIf { it.isNotBlank() }
        val conversationTitle = args.optString("conversation_title").takeIf { it.isNotBlank() }
        val isDevelopment = if (args.has("is_development")) args.optBoolean("is_development") else true
        val startAckTimeoutMs = args.optInt("start_ack_timeout_ms", 1_800).coerceIn(0, 10_000)

        val payload = JSONObject()
            .put("trace_id", traceId)
            .put("user_id", userId)
            .put("project_id", projectId)
            .put("project_title", projectTitle)
            .put("message", message)
        if (agent != null) payload.put("agent", agent)
        if (conversationId != null) payload.put("conversation_id", conversationId)
        if (conversationTitle != null) payload.put("conversation_title", conversationTitle)
        val payloadText = payload.toString()

        if (!force) {
            reservePendingTask(prefs, payloadText, isDevelopment)
        }

        DebugTraceStore.record(
            "mcp_chat_send",
            mapOf(
                "trace_id" to traceId,
                "project_id" to projectId,
                "chars" to message.length,
                "reserved_pending" to !force
            )
        )
        val intent = Intent(appContext, TaskWorkService::class.java).apply {
            action = TaskWorkService.ACTION_START_WORK
            putExtra(TaskWorkService.EXTRA_PAYLOAD, payloadText)
            putExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, isDevelopment)
            putExtra(TaskWorkService.EXTRA_FORCE_START, force)
        }
        val startResult = runCatching {
            ContextCompat.startForegroundService(appContext, intent)
        }.exceptionOrNull()
        if (startResult != null) {
            if (!force) clearReservedPendingTask(prefs, traceId)
            DebugTraceStore.record(
                "mcp_chat_start_failed",
                mapOf("trace_id" to traceId, "error" to startResult.message)
            )
            return toolResult(
                "Could not start phone task service.",
                JSONObject()
                    .put("trace_id", traceId)
                    .put("project_id", projectId)
                    .put("error", startResult.message ?: startResult.javaClass.simpleName),
                isError = true
            )
        }
        DebugTraceStore.record(
            "mcp_chat_queued",
            mapOf("trace_id" to traceId, "project_id" to projectId, "force" to force)
        )
        var serviceStart = waitForTaskStartSignal(traceId, startAckTimeoutMs)
        if (!serviceStart.optBoolean("confirmed", false) && startAckTimeoutMs > 0) {
            DebugTraceStore.record(
                "mcp_chat_start_unconfirmed",
                mapOf(
                    "trace_id" to traceId,
                    "timeout_ms" to startAckTimeoutMs,
                    "last_phase" to serviceStart.optString("last_phase").takeIf { it.isNotBlank() }
                )
            )
            val resumeIntent = Intent(appContext, TaskWorkService::class.java).apply {
                action = TaskWorkService.ACTION_RESUME_PENDING
            }
            val fallbackError = runCatching {
                ContextCompat.startForegroundService(appContext, resumeIntent)
            }.exceptionOrNull()
            if (fallbackError != null) {
                DebugTraceStore.record(
                    "mcp_chat_start_fallback_failed",
                    mapOf("trace_id" to traceId, "error" to fallbackError.message)
                )
                serviceStart = serviceStart
                    .put("fallback_attempted", true)
                    .put("fallback_error", fallbackError.message ?: fallbackError.javaClass.simpleName)
            } else {
                DebugTraceStore.record("mcp_chat_start_fallback_resume", mapOf("trace_id" to traceId))
                val fallback = waitForTaskStartSignal(traceId, startAckTimeoutMs)
                    .put("fallback_attempted", true)
                serviceStart = fallback.put("initial", serviceStart)
            }
        }

        val structured = JSONObject()
            .put("trace_id", traceId)
            .put("project_id", projectId)
            .put("project_title", projectTitle)
            .put("conversation_id", conversationId ?: JSONObject.NULL)
            .put("is_development", isDevelopment)
            .put("force", force)
            .put("message_chars", message.length)
            .put("service_start", serviceStart)
        return toolResult("Chat request queued on phone.", structured)
    }

    private fun waitForTaskStartSignal(traceId: String, timeoutMs: Int): JSONObject {
        val started = SystemClock.elapsedRealtime()
        val deadline = started + timeoutMs
        var events = emptyList<DebugTraceStore.TraceEvent>()
        while (true) {
            events = DebugTraceStore.recentEvents(300)
                .filter { it.details["trace_id"] == traceId }
            val signal = taskStartSignalJson(
                traceId = traceId,
                events = events,
                elapsedWaitMs = SystemClock.elapsedRealtime() - started,
                timedOut = false
            )
            if (signal.optBoolean("confirmed", false) || timeoutMs <= 0) return signal
            if (SystemClock.elapsedRealtime() >= deadline) break
            Thread.sleep(minOf(100L, (deadline - SystemClock.elapsedRealtime()).coerceAtLeast(0L)))
        }
        return taskStartSignalJson(
            traceId = traceId,
            events = events,
            elapsedWaitMs = SystemClock.elapsedRealtime() - started,
            timedOut = timeoutMs > 0
        )
    }

    private fun taskStartSignalJson(
        traceId: String,
        events: List<DebugTraceStore.TraceEvent>,
        elapsedWaitMs: Long,
        timedOut: Boolean
    ): JSONObject {
        fun latest(vararg phases: String) = events.lastOrNull { it.phase in phases }
        val started = latest("task_start_work")
        val resumed = latest("task_resume_pending")
        val rejected = latest("task_start_rejected_busy")
        val missingPayload = latest("task_start_missing_payload")
        val command = latest("task_service_command")
        val matched = started ?: resumed ?: rejected ?: missingPayload ?: command
        val status = when {
            started != null -> "started"
            resumed != null -> "resumed"
            rejected != null -> "rejected_busy"
            missingPayload != null -> "missing_payload"
            command != null -> "service_command_received"
            else -> "unconfirmed"
        }
        val confirmed = status != "unconfirmed" && status != "service_command_received"
        return JSONObject()
            .put("trace_id", traceId)
            .put("status", status)
            .put("confirmed", confirmed)
            .put("timed_out", timedOut && !confirmed)
            .put("elapsed_wait_ms", elapsedWaitMs)
            .put("event_count", events.size)
            .put("matched_phase", matched?.phase ?: JSONObject.NULL)
            .put("matched_wall_time_ms", matched?.wallTimeMs ?: JSONObject.NULL)
            .put("last_phase", events.lastOrNull()?.phase ?: JSONObject.NULL)
    }

    private fun chatProbe(args: JSONObject): JSONObject {
        val message = args.optString("message").trim()
        if (message.isEmpty()) {
            return toolResult("message is required", JSONObject().put("field", "message"), isError = true)
        }
        val waitFor = args.optString("wait_for", "first_reply").lowercase(Locale.ROOT)
        val timeoutMs = args.optInt("wait_timeout_ms", 25_000).coerceIn(0, 120_000)
        val pollIntervalMs = args.optInt("poll_interval_ms", 350).coerceIn(100, 2_000)
        val timelineLimit = args.optInt("timeline_limit", 80).coerceIn(1, 300)
        val includeDiagnosticBundle = if (args.has("include_diagnostic_bundle")) {
            args.optBoolean("include_diagnostic_bundle")
        } else {
            true
        }
        val includeLogcat = args.optBoolean("include_logcat", false)
        val includeNetworkCheck = if (args.has("include_network_check")) {
            args.optBoolean("include_network_check")
        } else {
            true
        }
        val includeServerTrace = if (args.has("include_server_trace")) {
            args.optBoolean("include_server_trace")
        } else {
            true
        }

        if (waitTargetPhases(waitFor) == null) {
            return toolResult(
                "Unsupported wait_for value: $waitFor",
                JSONObject()
                    .put("wait_for", waitFor)
                    .put("supported", JSONArray().apply {
                        listOf("queued", "task_start", "payload_sent", "first_server_event", "first_reply", "finish")
                            .forEach { put(it) }
                    }),
                isError = true
            )
        }

        val probeStartedAt = System.currentTimeMillis()
        val chatArgs = JSONObject(args.toString())
        if (!chatArgs.has("is_development")) chatArgs.put("is_development", false)
        if (!chatArgs.has("trace_id")) {
            chatArgs.put("trace_id", "mcp_probe_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}")
        }
        DebugTraceStore.record(
            "mcp_chat_probe_start",
            mapOf(
                "trace_id" to chatArgs.optString("trace_id"),
                "wait_for" to waitFor,
                "timeout_ms" to timeoutMs,
                "is_development" to chatArgs.optBoolean("is_development")
            )
        )

        val sendResult = chatSend(chatArgs)
        val sendStructured = sendResult.optJSONObject("structuredContent") ?: JSONObject()
        val traceId = sendStructured.optString("trace_id").takeIf { it.isNotBlank() }
            ?: chatArgs.optString("trace_id").takeIf { it.isNotBlank() }
        if (sendResult.optBoolean("isError", false) || traceId == null) {
            return toolResult(
                "Chat probe could not queue a phone chat request.",
                JSONObject()
                    .put("send_result", sendStructured)
                    .put("wait_for", waitFor)
                    .put("queued", false),
                isError = true
            )
        }

        val waitResult = waitForTraceTarget(
            traceId = traceId,
            waitFor = waitFor,
            timeoutMs = timeoutMs,
            pollIntervalMs = pollIntervalMs,
            startedAtWallMs = probeStartedAt
        )
        val latency = latencyReportJson(JSONObject().put("trace_id", traceId).put("timeline_limit", timelineLimit))
        val taskStatus = taskStatusJson(JSONObject().put("trace_id", traceId))
        val serverTrace = if (includeServerTrace) {
            JSONObject()
                .put("trace_id", traceId)
                .put("limit", args.optInt("server_trace_limit", 120).coerceIn(1, 300))
                .apply {
                    args.optString("server_url").takeIf { it.isNotBlank() }?.let { put("server_url", it) }
                }
                .let { serverTraceJson(it) }
        } else {
            JSONObject.NULL
        }
        val diagnostic = if (includeDiagnosticBundle) {
            diagnosticBundle(
                JSONObject()
                    .put("trace_id", traceId)
                    .put("include_logcat", includeLogcat)
                    .put("include_network_check", includeNetworkCheck)
                    .put("include_server_trace", includeServerTrace)
                    .put("server_trace_limit", args.optInt("server_trace_limit", 120).coerceIn(1, 300))
                    .put("trace_limit", timelineLimit)
                    .put("timeline_limit", timelineLimit)
                    .put("since_wall_time_ms", probeStartedAt)
                    .apply {
                        args.optString("server_url").takeIf { it.isNotBlank() }?.let { put("server_url", it) }
                    }
            ).optJSONObject("structuredContent") ?: JSONObject()
        } else {
            JSONObject.NULL
        }

        DebugTraceStore.record(
            "mcp_chat_probe_finish",
            mapOf(
                "trace_id" to traceId,
                "wait_for" to waitFor,
                "reached" to waitResult.optBoolean("reached"),
                "matched_phase" to waitResult.optString("matched_phase").takeIf { it.isNotBlank() },
                "elapsed_wait_ms" to waitResult.optLong("elapsed_wait_ms")
            )
        )
        val structured = JSONObject()
            .put("trace_id", traceId)
            .put("wait_for", waitFor)
            .put("timeout_ms", timeoutMs)
            .put("queued", true)
            .put("send_result", sendStructured)
            .put("wait_result", waitResult)
            .put("task_status", taskStatus)
            .put("latency_report", latency)
            .put("server_trace", serverTrace)
            .put("diagnostic_bundle", diagnostic)
        return toolResult(
            if (waitResult.optBoolean("reached", false)) {
                "Chat probe reached $waitFor."
            } else {
                "Chat probe timed out before $waitFor."
            },
            structured,
            isError = waitResult.optBoolean("timed_out", false)
        )
    }

    private fun waitForTraceTarget(
        traceId: String,
        waitFor: String,
        timeoutMs: Int,
        pollIntervalMs: Int,
        startedAtWallMs: Long
    ): JSONObject {
        val targetPhases = waitTargetPhases(waitFor) ?: emptySet()
        val terminalPhases = setOf("task_finish_done", "task_finish_error")
        val deadline = SystemClock.elapsedRealtime() + timeoutMs
        var lastEvents = emptyList<DebugTraceStore.TraceEvent>()
        while (true) {
            lastEvents = DebugTraceStore.recentEvents(300)
                .filter { it.details["trace_id"] == traceId || it.phase == "mcp_chat_send" && it.details["trace_id"] == traceId }
            val matched = lastEvents.firstOrNull { it.phase in targetPhases }
            if (matched != null) {
                return waitResultJson(
                    traceId = traceId,
                    waitFor = waitFor,
                    reached = true,
                    timedOut = false,
                    matched = matched,
                    events = lastEvents,
                    startedAtWallMs = startedAtWallMs,
                    timeoutMs = timeoutMs,
                    pollIntervalMs = pollIntervalMs
                )
            }
            val terminal = lastEvents.firstOrNull { it.phase in terminalPhases }
            if (terminal != null && waitFor != "finish") {
                return waitResultJson(
                    traceId = traceId,
                    waitFor = waitFor,
                    reached = false,
                    timedOut = false,
                    matched = terminal,
                    events = lastEvents,
                    startedAtWallMs = startedAtWallMs,
                    timeoutMs = timeoutMs,
                    pollIntervalMs = pollIntervalMs,
                    terminalBeforeTarget = true
                )
            }
            if (timeoutMs <= 0 || SystemClock.elapsedRealtime() >= deadline) break
            Thread.sleep(minOf(pollIntervalMs.toLong(), (deadline - SystemClock.elapsedRealtime()).coerceAtLeast(0L)))
        }
        return waitResultJson(
            traceId = traceId,
            waitFor = waitFor,
            reached = false,
            timedOut = true,
            matched = lastEvents.lastOrNull(),
            events = lastEvents,
            startedAtWallMs = startedAtWallMs,
            timeoutMs = timeoutMs,
            pollIntervalMs = pollIntervalMs
        )
    }

    private fun waitTargetPhases(waitFor: String): Set<String>? {
        return when (waitFor) {
            "queued" -> setOf("mcp_chat_queued")
            "task_start" -> setOf("task_start_work", "task_resume_pending")
            "payload_sent" -> setOf("task_payload_sent")
            "first_server_event" -> setOf("task_first_server_event")
            "first_reply" -> setOf("task_first_chat_reply")
            "finish" -> setOf("task_finish_done", "task_finish_error")
            else -> null
        }
    }

    private fun waitResultJson(
        traceId: String,
        waitFor: String,
        reached: Boolean,
        timedOut: Boolean,
        matched: DebugTraceStore.TraceEvent?,
        events: List<DebugTraceStore.TraceEvent>,
        startedAtWallMs: Long,
        timeoutMs: Int,
        pollIntervalMs: Int,
        terminalBeforeTarget: Boolean = false
    ): JSONObject {
        return JSONObject()
            .put("trace_id", traceId)
            .put("wait_for", waitFor)
            .put("reached", reached)
            .put("timed_out", timedOut)
            .put("terminal_before_target", terminalBeforeTarget)
            .put("matched_phase", matched?.phase ?: JSONObject.NULL)
            .put("matched_wall_time_ms", matched?.wallTimeMs ?: JSONObject.NULL)
            .put("last_phase", events.lastOrNull()?.phase ?: JSONObject.NULL)
            .put("event_count", events.size)
            .put("elapsed_wait_ms", System.currentTimeMillis() - startedAtWallMs)
            .put("timeout_ms", timeoutMs)
            .put("poll_interval_ms", pollIntervalMs)
            .put("diagnosis", waitFailureDiagnosis(waitFor, reached, terminalBeforeTarget, events))
    }

    private fun waitFailureDiagnosis(
        waitFor: String,
        reached: Boolean,
        terminalBeforeTarget: Boolean,
        events: List<DebugTraceStore.TraceEvent>
    ): JSONObject {
        if (reached) {
            return JSONObject()
                .put("available", false)
                .put("reason", "target_reached")
        }
        val phases = events.map { it.phase }.toSet()
        val code = when {
            terminalBeforeTarget -> "terminal_before_target"
            "task_payload_send_failed" in phases -> "payload_send_failed"
            "task_payload_sent" in phases && "task_first_server_event" !in phases -> "waiting_for_backend_first_event"
            ("task_start_work" in phases || "task_resume_pending" in phases) && "task_payload_sent" !in phases -> "task_started_without_payload_sent"
            "mcp_chat_start_unconfirmed" in phases -> "service_start_unconfirmed"
            "mcp_chat_queued" in phases && "task_start_work" !in phases && "task_resume_pending" !in phases -> "queued_without_task_start"
            "task_service_command" in phases && "task_start_work" !in phases && "task_resume_pending" !in phases -> "service_command_without_task_start"
            else -> "insufficient_trace"
        }
        val area = when (code) {
            "service_start_unconfirmed",
            "queued_without_task_start",
            "service_command_without_task_start" -> "phone_task_start"
            "task_started_without_payload_sent",
            "payload_send_failed" -> "phone_payload_send"
            "waiting_for_backend_first_event" -> "backend_or_network_first_byte"
            "terminal_before_target" -> "task_terminal"
            else -> "unknown"
        }
        val action = when (area) {
            "phone_task_start" -> "Inspect task_service_command, task_start_work, and mcp_chat_start_unconfirmed events; the MCP request may not be reaching TaskWorkService reliably."
            "phone_payload_send" -> "Inspect WebSocket connection state and payload size; a connected client must still call sendPendingPayloadIfNeeded for each new task."
            "backend_or_network_first_byte" -> "Compare phone latency_report with backend server_trace for the same trace_id."
            "task_terminal" -> "Read task_status and task_server_message for the terminal error before the requested milestone."
            else -> "Collect diagnostic_bundle with include_logcat=true for the same trace_id."
        }
        return JSONObject()
            .put("available", true)
            .put("wait_for", waitFor)
            .put("code", code)
            .put("likely_area", area)
            .put("action", action)
    }

    private fun statusJson(includeToken: Boolean): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val pendingTask = pendingTaskJson(prefs)
        val pendingTasks = pendingTasksJson(prefs)
        val pendingBusy = isTaskBusy(prefs)
        val processState = processStateJson()
        val appForegroundRecorded = prefs.getBoolean(TaskWorkService.PREF_APP_IN_FOREGROUND, false)
        return JSONObject()
            .put("package_name", appContext.packageName)
            .put("version_name", BuildConfig.VERSION_NAME)
            .put("version_code", BuildConfig.VERSION_CODE)
            .put("mcp_endpoint", "http://$HOST:$PORT/mcp")
            .put("mcp_health", "http://$HOST:$PORT/health")
            .put("adb_forward", "adb forward tcp:$PORT tcp:$PORT")
            .put("host", HOST)
            .put("port", PORT)
            .put("protocol_version", PROTOCOL_VERSION)
            .put("running", running)
            .put("process_uptime_ms", SystemClock.elapsedRealtime() - processStartedElapsedMs)
            .put("app_foreground", processState.optBoolean("foreground", appForegroundRecorded))
            .put("app_foreground_recorded", appForegroundRecorded)
            .put("process_state", processState)
            .put("background_debug_supported", true)
            .put("trace_persistence", "shared_preferences")
            .put("debug_keepalive", McpDebugKeepAliveService.statusJson(appContext))
            .put("user_id", prefs.getString(TaskWorkService.PREF_USER_ID, null))
            .put("active_project_id", prefs.getString(TaskWorkService.PREF_ACTIVE_PROJECT_ID, null))
            .put("pending_work", pendingBusy)
            .put("busy", pendingBusy)
            .put("active_task_count", pendingTasks.length())
            .put("active_tasks", pendingTasks)
            .put("active_trace_id", if (pendingBusy) pendingTask?.optString("trace_id")?.takeIf { it.isNotBlank() } else null)
            .put("active_task_kind", if (pendingBusy) pendingTaskKind(prefs) else null)
            .put("pending_work_age_ms", if (pendingBusy) pendingWorkAgeMs(prefs) else null)
            .put("queued_task_events", queuedTaskEvents(prefs).size)
            .put("trace_events", DebugTraceStore.count())
            .put("mcp_metrics", metricsJson())
            .apply {
                if (includeToken) put("auth_token", debugToken())
            }
    }

    private fun taskStatusJson(args: JSONObject): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val pendingTask = pendingTaskJson(prefs)
        val requestedTraceId = args.optString("trace_id").takeIf { it.isNotBlank() }
        val events = DebugTraceStore.recentEvents(300)
        val traceId = requestedTraceId
            ?: pendingTask?.optString("trace_id")?.takeIf { it.isNotBlank() }
            ?: mcpLatestTraceId(events)
        val traceEvents = if (traceId == null) {
            emptyList()
        } else {
            events.filter { it.details["trace_id"] == traceId }
        }
        val finish = traceEvents.lastOrNull {
            it.phase == "task_finish_done" || it.phase == "task_finish_error"
        }
        val rejected = traceEvents.lastOrNull {
            it.phase == "task_start_rejected_busy" || it.phase == "mcp_chat_rejected_busy"
        }
        val serviceCommand = traceEvents.lastOrNull { it.phase == "task_service_command" }
        val startUnconfirmed = traceEvents.lastOrNull { it.phase == "mcp_chat_start_unconfirmed" }
        val hasTaskStart = traceEvents.any { it.phase == "task_start_work" || it.phase == "task_resume_pending" }
        val status = when {
            finish?.phase == "task_finish_done" -> "done"
            finish?.phase == "task_finish_error" -> "error"
            rejected != null && !hasTaskStart -> "rejected_busy"
            isTaskBusy(prefs) && traceId != null && isTracePending(prefs, traceId) -> "running"
            traceEvents.any { it.phase == "task_payload_sent" } -> "sent"
            hasTaskStart -> "started"
            startUnconfirmed != null -> "start_unconfirmed"
            serviceCommand != null -> "service_received"
            traceEvents.isNotEmpty() -> "observed"
            else -> "idle"
        }
        val lastMessage = traceEvents.lastOrNull { it.phase == "task_server_message" }
        return JSONObject()
            .put("status", status)
            .put("busy", isTaskBusy(prefs))
            .put("trace_id", traceId ?: JSONObject.NULL)
            .put("kind", pendingTaskKind(prefs))
            .put("pending_work_age_ms", pendingWorkAgeMs(prefs) ?: JSONObject.NULL)
            .put("event_count", traceEvents.size)
            .put("sent_elapsed_ms", mcpTraceDetailLong(traceEvents, "task_payload_sent", "elapsed_ms"))
            .put("first_server_event_elapsed_ms", mcpTraceDetailLong(traceEvents, "task_first_server_event", "elapsed_ms"))
            .put("first_chat_reply_elapsed_ms", mcpTraceDetailLong(traceEvents, "task_first_chat_reply", "elapsed_ms"))
            .put("finish_elapsed_ms", finish?.details?.get("elapsed_ms")?.toLongOrNull() ?: JSONObject.NULL)
            .put("last_message_type", lastMessage?.details?.get("type") ?: JSONObject.NULL)
            .put("last_message_preview", lastMessage?.details?.get("message_preview") ?: JSONObject.NULL)
            .put("has_apk_url", finish?.details?.get("has_apk_url") ?: JSONObject.NULL)
            .put("last_phase", traceEvents.lastOrNull()?.phase ?: JSONObject.NULL)
            .put("last_event_wall_time_ms", traceEvents.lastOrNull()?.wallTimeMs ?: JSONObject.NULL)
    }

    private fun latencyReportJson(args: JSONObject): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val allEvents = DebugTraceStore.recentEvents(300)
        val requestedTraceId = args.optString("trace_id").takeIf { it.isNotBlank() }
        val pendingTraceId = pendingTaskJson(prefs)?.optString("trace_id")?.takeIf { it.isNotBlank() }
        val traceId = requestedTraceId ?: pendingTraceId ?: mcpLatestTraceId(allEvents)
        val traceEvents = if (traceId == null) {
            emptyList()
        } else {
            allEvents.filter { it.details["trace_id"] == traceId }
        }
        val timelineLimit = args.optInt("timeline_limit", 80).coerceIn(1, 300)
        val taskStatusArgs = JSONObject()
        traceId?.let { taskStatusArgs.put("trace_id", it) }
        val taskStatus = taskStatusJson(taskStatusArgs)

        fun first(phase: String) = traceEvents.firstOrNull { it.phase == phase }
        fun last(phase: String) = traceEvents.lastOrNull { it.phase == phase }

        val mcpSend = first("mcp_chat_send")
        val mcpQueued = first("mcp_chat_queued")
        val serviceCommand = first("task_service_command")
        val startUnconfirmed = last("mcp_chat_start_unconfirmed")
        val taskStart = first("task_start_work") ?: first("task_resume_pending")
        val wsConnected = first("task_ws_connected")
        val payloadSent = first("task_payload_sent")
        val firstServer = first("task_first_server_event")
        val firstReply = first("task_first_chat_reply")
        val finish = first("task_finish_done") ?: first("task_finish_error")
        val baseline = taskStart ?: traceEvents.firstOrNull()
        val milestones = JSONObject()

        fun elapsedFromBaseline(event: DebugTraceStore.TraceEvent): Any {
            val fromTrace = event.details["elapsed_ms"]?.toLongOrNull()
            return when {
                fromTrace != null -> fromTrace
                baseline != null -> (event.wallTimeMs - baseline.wallTimeMs).coerceAtLeast(0L)
                else -> JSONObject.NULL
            }
        }

        fun putMilestone(name: String, event: DebugTraceStore.TraceEvent?) {
            if (event == null) return
            milestones.put(
                name,
                JSONObject()
                    .put("phase", event.phase)
                    .put("wall_time_ms", event.wallTimeMs)
                    .put("elapsed_from_task_start_ms", elapsedFromBaseline(event))
                    .put("details", JSONObject().apply {
                        event.details.forEach { (key, value) -> put(key, value) }
                    })
            )
        }

        putMilestone("mcp_send", mcpSend)
        putMilestone("mcp_queued", mcpQueued)
        putMilestone("service_command", serviceCommand)
        putMilestone("mcp_start_unconfirmed", startUnconfirmed)
        putMilestone("task_start", taskStart)
        putMilestone("ws_connected", wsConnected)
        putMilestone("payload_sent", payloadSent)
        putMilestone("first_server_event", firstServer)
        putMilestone("first_chat_reply", firstReply)
        putMilestone("finish", finish)

        val segments = JSONArray()
        val bottleneckCandidates = mutableListOf<Pair<String, Long>>()
        fun addSegment(name: String, from: DebugTraceStore.TraceEvent?, to: DebugTraceStore.TraceEvent?, candidate: Boolean = true) {
            if (from == null || to == null) return
            val duration = (to.wallTimeMs - from.wallTimeMs).coerceAtLeast(0L)
            segments.put(
                JSONObject()
                    .put("name", name)
                    .put("from_phase", from.phase)
                    .put("to_phase", to.phase)
                    .put("duration_ms", duration)
            )
            if (candidate) bottleneckCandidates += name to duration
        }

        addSegment("mcp_send_to_queue", mcpSend, mcpQueued)
        addSegment("mcp_queue_to_service_command", mcpQueued, serviceCommand)
        addSegment("service_command_to_task_start", serviceCommand, taskStart)
        addSegment("mcp_queue_to_task_start", mcpQueued, taskStart)
        if (mcpQueued != null && taskStart == null && finish == null) {
            val now = DebugTraceStore.TraceEvent(
                wallTimeMs = System.currentTimeMillis(),
                elapsedMs = SystemClock.elapsedRealtime(),
                phase = "now",
                details = mapOf("trace_id" to traceId.orEmpty())
            )
            addSegment("mcp_queue_to_now", mcpQueued, now)
        }
        addSegment("task_start_to_ws_connected", taskStart, wsConnected)
        addSegment("ws_connected_to_payload_sent", wsConnected, payloadSent)
        addSegment("task_start_to_payload_sent", taskStart, payloadSent)
        addSegment("payload_sent_to_first_server_event", payloadSent, firstServer)
        addSegment("first_server_event_to_first_chat_reply", firstServer, firstReply)
        addSegment("first_chat_reply_to_finish", firstReply, finish)
        addSegment("task_start_to_finish", taskStart, finish, candidate = false)

        val bottleneck = bottleneckCandidates.maxByOrNull { it.second }
        val missingMilestones = JSONArray().apply {
            if (taskStart == null) put("task_start")
            if (payloadSent == null) put("payload_sent")
            if (firstServer == null) put("first_server_event")
            if (firstReply == null) put("first_chat_reply")
            if (finish == null) put("finish")
        }
        val timeline = JSONArray().apply {
            traceEvents.takeLast(timelineLimit).forEach { event ->
                put(
                    JSONObject()
                        .put("phase", event.phase)
                        .put("wall_time_ms", event.wallTimeMs)
                        .put("elapsed_from_task_start_ms", elapsedFromBaseline(event))
                        .put("details", JSONObject().apply {
                            event.details.forEach { (key, value) -> put(key, value) }
                        })
                )
            }
        }

        return JSONObject()
            .put("trace_id", traceId ?: JSONObject.NULL)
            .put("status", taskStatus.optString("status", "idle"))
            .put("kind", taskStatus.optString("kind", "idle"))
            .put("event_count", traceEvents.size)
            .put("timeline_limit", timelineLimit)
            .put("milestones", milestones)
            .put("segments", segments)
            .put("bottleneck", mcpBottleneckJson(bottleneck))
            .put("missing_milestones", missingMilestones)
            .put("timeline", timeline)
            .put("task_status", taskStatus)
    }

    private fun serverTraceJson(args: JSONObject): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val events = DebugTraceStore.recentEvents(300)
        val traceId = args.optString("trace_id").takeIf { it.isNotBlank() }
            ?: pendingTaskJson(prefs)?.optString("trace_id")?.takeIf { it.isNotBlank() }
            ?: mcpLatestTraceId(events)
        val limit = args.optInt("limit", 120).coerceIn(1, 300)
        val baseUrl = args.optString("server_url").takeIf { it.isNotBlank() }
            ?: DEFAULT_SERVER_BASE_URL
        if (traceId == null) {
            return JSONObject()
                .put("available", false)
                .put("reason", "missing_trace_id")
                .put("server_url", baseUrl)
                .put("limit", limit)
        }

        val encodedTraceId = URLEncoder.encode(traceId, "UTF-8")
        val url = "${baseUrl.trimEnd('/')}/api/debug/traces/$encodedTraceId?limit=$limit"
        val started = SystemClock.elapsedRealtime()
        val response = fetchJson(url)
        val durationMs = SystemClock.elapsedRealtime() - started
        if (response == null) {
            return JSONObject()
                .put("available", false)
                .put("reason", "server_unreachable_or_non_json")
                .put("trace_id", traceId)
                .put("url", url)
                .put("duration_ms", durationMs)
                .put("limit", limit)
        }
        return response
            .put("available", true)
            .put("url", url)
            .put("duration_ms", durationMs)
            .put("server_url", baseUrl)
    }

    private fun metricsJson(): JSONObject {
        return JSONObject()
            .put("active_connections", activeConnections.get())
            .put("total_requests", totalRequests.get())
            .put("total_tool_calls", totalToolCalls.get())
            .put("failed_requests", failedRequests.get())
            .put("last_request_wall_time_ms", if (lastRequestWallTimeMs > 0L) lastRequestWallTimeMs else JSONObject.NULL)
            .put("last_request_duration_ms", lastRequestDurationMs)
            .put("last_http_method", lastHttpMethod ?: JSONObject.NULL)
            .put("last_path", lastPath ?: JSONObject.NULL)
            .put("last_rpc_method", lastRpcMethod ?: JSONObject.NULL)
            .put("last_tool_name", lastToolName ?: JSONObject.NULL)
            .put("last_error", lastError ?: JSONObject.NULL)
            .put("socket_timeout_ms", SOCKET_TIMEOUT_MS)
    }

    private fun recordHttpRequest(method: String, path: String, startedElapsedMs: Long) {
        totalRequests.incrementAndGet()
        lastRequestWallTimeMs = System.currentTimeMillis()
        lastRequestDurationMs = SystemClock.elapsedRealtime() - startedElapsedMs
        lastHttpMethod = method
        lastPath = path
        lastError = null
    }

    private fun deviceSnapshotJson(): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        return JSONObject()
            .put("wall_time_ms", System.currentTimeMillis())
            .put("elapsed_realtime_ms", SystemClock.elapsedRealtime())
            .put("timezone", TimeZone.getDefault().id)
            .put("app", statusJson(includeToken = false))
            .put("debug_session", mcpDebugSessionJson(prefs))
            .put("background_debug", backgroundDebugStatusJson(appContext))
            .put("memory", memoryJson(appContext))
            .put("battery", batteryJson(appContext))
            .put("network", networkCapabilitiesJson(appContext))
            .put("build", buildJson())
    }

    private fun authorized(headers: Map<String, String>, args: JSONObject): Boolean {
        return mcpAuthorized(headers, args, debugToken())
    }

    private fun debugToken(): String {
        return mcpDebugToken(appContext, PREF_MCP_TOKEN)
    }

    private fun writeResponse(
        socket: Socket,
        status: Int,
        reason: String,
        body: String,
        contentType: String = "application/json; charset=utf-8"
    ) {
        writeHttpResponse(socket, status, reason, body, PROTOCOL_VERSION, contentType)
    }
}

