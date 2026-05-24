package com.elon.app

import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.SharedPreferences
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.BatteryManager
import android.os.Build
import android.os.SystemClock
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
import java.net.URL
import java.util.Locale
import java.util.TimeZone
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong

object McpDebugServer {
    private const val TAG = "ElonMcpServer"
    private const val HOST = "127.0.0.1"
    private const val PORT = 8787
    private const val PROTOCOL_VERSION = "2025-06-18"
    private const val PREF_MCP_TOKEN = "mcp_debug_token"
    private const val PREF_DEBUG_SESSION_ID = "mcp_debug_session_id"
    private const val PREF_DEBUG_SESSION_STARTED_AT = "mcp_debug_session_started_at"
    private const val PREF_DEBUG_SESSION_NOTE = "mcp_debug_session_note"
    private const val MAX_BODY_BYTES = 256 * 1024
    private const val MAX_HEADER_BYTES = 16 * 1024
    private const val SOCKET_TIMEOUT_MS = 5_000

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
            "initialize" -> rpcResult(id, initializeResult())
            "ping" -> rpcResult(id, JSONObject())
            "tools/list" -> rpcResult(id, toolsListResult())
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
            "trace_recent" -> traceRecent(args)
            "trace_clear" -> {
                DebugTraceStore.clear()
                toolResult("Trace buffer cleared.", JSONObject().put("cleared", true))
            }
            "debug_session" -> debugSession(args)
            "diagnostic_bundle" -> diagnosticBundle(args)
            "device_snapshot" -> deviceSnapshot(args)
            "network_check" -> networkCheck(args)
            "mcp_self_check" -> mcpSelfCheck(args)
            "mcp_metrics" -> mcpMetrics(args)
            "debug_keepalive" -> debugKeepalive(args)
            "update_status" -> updateStatus(args)
            "task_status" -> taskStatus(args)
            "task_control" -> taskControl(args)
            "task_events" -> taskEvents(args)
            "logcat_recent" -> logcatRecent(args)
            "chat_send" -> chatSend(args)
            else -> toolResult("Unknown tool: $name", JSONObject().put("tool", name), isError = true)
        }
        return rpcResult(id, result)
    }

    private fun traceRecent(args: JSONObject): JSONObject {
        val limit = args.optInt("limit", 80).coerceIn(1, 300)
        val traceId = args.optString("trace_id").takeIf { it.isNotBlank() }
        val phase = args.optString("phase").takeIf { it.isNotBlank() }
        val contains = args.optString("contains").takeIf { it.isNotBlank() }
        val sinceWallTimeMs = args.optLong("since_wall_time_ms", 0L).takeIf { it > 0L }
        val allEvents = DebugTraceStore.recentEvents(300)
        val filtered = allEvents.filter { event ->
            (traceId == null || event.details["trace_id"] == traceId) &&
                (phase == null || event.phase == phase) &&
                (contains == null || traceEventSearchText(event).contains(contains, ignoreCase = true)) &&
                (sinceWallTimeMs == null || event.wallTimeMs >= sinceWallTimeMs)
        }
        val events = filtered.takeLast(limit)
        val structured = JSONObject()
            .put("events", traceEventsJson(events))
            .put("limit", limit)
            .put("matched_count", filtered.size)
            .put(
                "filters",
                JSONObject()
                    .put("trace_id", traceId ?: JSONObject.NULL)
                    .put("phase", phase ?: JSONObject.NULL)
                    .put("contains", contains ?: JSONObject.NULL)
                    .put("since_wall_time_ms", sinceWallTimeMs ?: JSONObject.NULL)
            )
        return toolResult("Returned ${events.size} recent trace events.", structured)
    }

    private fun debugSession(args: JSONObject): JSONObject {
        val action = args.optString("action", "status").lowercase(Locale.ROOT)
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val structured = when (action) {
            "start" -> startDebugSession(
                prefs = prefs,
                requestedSessionId = args.optString("session_id").takeIf { it.isNotBlank() },
                note = args.optString("note").takeIf { it.isNotBlank() }
            ).put("action", action)
            "end" -> {
                val current = debugSessionJson(prefs)
                prefs.edit()
                    .remove(PREF_DEBUG_SESSION_ID)
                    .remove(PREF_DEBUG_SESSION_STARTED_AT)
                    .remove(PREF_DEBUG_SESSION_NOTE)
                    .apply()
                DebugTraceStore.record(
                    "mcp_debug_session_end",
                    mapOf("debug_session_id" to current.optString("session_id").takeIf { it.isNotBlank() })
                )
                debugSessionJson(prefs)
                    .put("action", action)
                    .put("ended_session", current)
            }
            "status" -> debugSessionJson(prefs).put("action", action)
            else -> return toolResult(
                "Unsupported debug session action: $action",
                JSONObject().put("action", action),
                isError = true
            )
        }
        return toolResult("Debug session status returned.", structured)
    }

    private fun diagnosticBundle(args: JSONObject): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val session = if (args.optBoolean("start_session", false)) {
            startDebugSession(
                prefs = prefs,
                requestedSessionId = args.optString("session_id").takeIf { it.isNotBlank() },
                note = args.optString("note").takeIf { it.isNotBlank() }
            )
        } else {
            debugSessionJson(prefs)
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
        val trace = traceRecent(traceArgs).optJSONObject("structuredContent") ?: JSONObject()
        val taskEvents = taskEvents(JSONObject().put("limit", args.optInt("task_event_limit", 20).coerceIn(1, 120)))
            .optJSONObject("structuredContent")
            ?: JSONObject()
        val logcat = if (includeLogcat) {
            logcatRecent(logcatArgs).optJSONObject("structuredContent") ?: JSONObject()
        } else {
            JSONObject.NULL
        }
        val network = if (includeNetworkCheck) {
            networkCheck(JSONObject()).optJSONObject("structuredContent") ?: JSONObject()
        } else {
            JSONObject.NULL
        }

        DebugTraceStore.record(
            "mcp_diagnostic_bundle",
            mapOf(
                "debug_session_id" to session.optString("session_id").takeIf { it.isNotBlank() },
                "include_logcat" to includeLogcat,
                "include_network_check" to includeNetworkCheck,
                "since_wall_time_ms" to sinceWallTimeMs
            )
        )
        val structured = JSONObject()
            .put("generated_at_ms", System.currentTimeMillis())
            .put("debug_session", session)
            .put("since_wall_time_ms", sinceWallTimeMs ?: JSONObject.NULL)
            .put("status", statusJson(includeToken = false))
            .put("self_check", selfCheck)
            .put("device_snapshot", deviceSnapshotJson())
            .put("network_check", network)
            .put("task_status", taskStatusJson(JSONObject()))
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
        return toolResult("Network check returned.", networkCheckJson(args))
    }

    private fun mcpSelfCheck(args: JSONObject): JSONObject {
        val includeUpdateCheck = args.optBoolean("include_update_check", false)
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val status = statusJson(includeToken = false)
        val taskStatus = taskStatusJson(JSONObject())
        val keepalive = McpDebugKeepAliveService.statusJson(appContext)
        val queuedEvents = queuedTaskEvents(prefs)
        val checks = JSONArray()
        fun addCheck(name: String, ok: Boolean, detail: String) {
            checks.put(JSONObject().put("name", name).put("ok", ok).put("detail", detail))
        }

        val appForeground = status.optBoolean("app_foreground", false)
        val keepaliveActive = keepalive.optBoolean("active", false)
        addCheck("mcp_server_running", running, "MCP server thread is accepting local HTTP requests.")
        addCheck(
            "background_keepalive",
            appForeground || keepaliveActive,
            if (appForeground) "App is foreground; keepalive is optional." else "Foreground keepalive must be active while app is background."
        )
        addCheck("trace_buffer", DebugTraceStore.count() > 0, "Trace buffer has ${DebugTraceStore.count()} persisted events.")
        addCheck(
            "request_health",
            lastError == null,
            lastError?.let { "Last MCP request error: $it" } ?: "No MCP request error recorded in this process."
        )
        addCheck(
            "task_queue",
            queuedEvents.size <= 120,
            "Queued background task events: ${queuedEvents.size}."
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

        val updateStatus = if (includeUpdateCheck) {
            updateStatus(JSONObject()).optJSONObject("structuredContent") ?: JSONObject.NULL
        } else {
            JSONObject.NULL
        }
        val ready = (0 until checks.length()).all { checks.optJSONObject(it)?.optBoolean("ok") == true }
        val structured = JSONObject()
            .put("ready", ready)
            .put("status", status)
            .put("task_status", taskStatus)
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

    private fun debugKeepalive(args: JSONObject): JSONObject {
        val action = args.optString("action", "status").lowercase(Locale.ROOT)
        when (action) {
            "start" -> {
                appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
                    .edit()
                    .putBoolean(McpDebugKeepAliveService.PREF_MANUAL_STOPPED, false)
                    .putBoolean(McpDebugKeepAliveService.PREF_ACTIVE, true)
                    .putLong(McpDebugKeepAliveService.PREF_STARTED_AT, System.currentTimeMillis())
                    .apply()
                val intent = Intent(appContext, McpDebugKeepAliveService::class.java).apply {
                    this.action = McpDebugKeepAliveService.ACTION_START
                }
                ContextCompat.startForegroundService(appContext, intent)
                DebugTraceStore.record("mcp_keepalive_requested", mapOf("action" to "start"))
            }
            "stop" -> {
                appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
                    .edit()
                    .putBoolean(McpDebugKeepAliveService.PREF_MANUAL_STOPPED, true)
                    .putBoolean(McpDebugKeepAliveService.PREF_ACTIVE, false)
                    .remove(McpDebugKeepAliveService.PREF_STARTED_AT)
                    .apply()
                val intent = Intent(appContext, McpDebugKeepAliveService::class.java).apply {
                    this.action = McpDebugKeepAliveService.ACTION_STOP
                }
                appContext.stopService(intent)
                DebugTraceStore.record("mcp_keepalive_requested", mapOf("action" to "stop"))
            }
            "status" -> Unit
            else -> return toolResult(
                "Unsupported keepalive action: $action",
                JSONObject().put("action", action),
                isError = true
            )
        }
        return toolResult(
            "MCP keepalive status returned.",
            McpDebugKeepAliveService.statusJson(appContext).put("action", action)
        )
    }

    private fun updateStatus(args: JSONObject): JSONObject {
        val url = args.optString("server_url")
            .takeIf { it.isNotBlank() }
            ?: "http://43.139.149.158:8080/app/version.json"
        val server = fetchJson(url)
        val latestCode = server?.optInt("versionCode", 0) ?: 0
        val structured = JSONObject()
            .put("installed_version_name", BuildConfig.VERSION_NAME)
            .put("installed_version_code", BuildConfig.VERSION_CODE)
            .put("server_url", url)
            .put("server_version", server ?: JSONObject.NULL)
            .put("update_available", latestCode > BuildConfig.VERSION_CODE)
        return toolResult("APK update status returned.", structured, isError = server == null)
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

    private fun logcatRecent(args: JSONObject): JSONObject {
        val lineCount = args.optInt("line_count", 300).coerceIn(20, 1_000)
        val pattern = args.optString(
            "pattern",
            "ElonTrace|ElonTaskWork|ElonWsClient|ElonMcpServer|AndroidRuntime|FATAL EXCEPTION|ANR"
        ).takeIf { it.isNotBlank() }
            ?: "ElonTrace|ElonTaskWork|ElonWsClient|ElonMcpServer|AndroidRuntime|FATAL EXCEPTION|ANR"
        val regex = runCatching { Regex(pattern) }.getOrElse {
            return toolResult(
                "Invalid logcat pattern.",
                JSONObject().put("pattern", pattern).put("error", it.message),
                isError = true
            )
        }
        val command = listOf("logcat", "-d", "-v", "time", "-t", lineCount.toString())
        val process = runCatching { ProcessBuilder(command).redirectErrorStream(true).start() }.getOrElse {
            return toolResult(
                "Could not start logcat.",
                JSONObject().put("error", it.message),
                isError = true
            )
        }
        val finished = process.waitFor(3, TimeUnit.SECONDS)
        if (!finished) {
            process.destroy()
            return toolResult(
                "logcat timed out.",
                JSONObject().put("timeout_ms", 3_000),
                isError = true
            )
        }
        val output = process.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
        val lines = output
            .lineSequence()
            .filter { regex.containsMatchIn(it) }
            .map { redactLogLine(it) }
            .toList()
            .takeLast(300)
        val structured = JSONObject()
            .put("line_count", lineCount)
            .put("pattern", pattern)
            .put("limited_by_android_log_permissions", true)
            .put("lines", JSONArray().apply { lines.forEach { put(it) } })
        return toolResult("Filtered logcat returned.", structured)
    }

    private fun chatSend(args: JSONObject): JSONObject {
        val message = args.optString("message").trim()
        if (message.isEmpty()) {
            return toolResult("message is required", JSONObject().put("field", "message"), isError = true)
        }
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val force = args.optBoolean("force", false)
        if (isTaskBusy(prefs) && !force) {
            val structured = taskStatusJson(JSONObject())
                .put("rejected", true)
                .put("reason", "busy")
            DebugTraceStore.record(
                "mcp_chat_rejected_busy",
                mapOf(
                    "active_trace_id" to structured.optString("trace_id").takeIf { it.isNotBlank() },
                    "message_chars" to message.length
                )
            )
            return toolResult(
                "Phone already has an active task. Pass force=true to override, or wait for completion.",
                structured,
                isError = true
            )
        }
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

        val structured = JSONObject()
            .put("trace_id", traceId)
            .put("project_id", projectId)
            .put("project_title", projectTitle)
            .put("conversation_id", conversationId ?: JSONObject.NULL)
            .put("is_development", isDevelopment)
            .put("force", force)
            .put("message_chars", message.length)
        return toolResult("Chat request queued on phone.", structured)
    }

    private fun initializeResult(): JSONObject {
        return JSONObject()
            .put("protocolVersion", PROTOCOL_VERSION)
            .put("capabilities", JSONObject().put("tools", JSONObject().put("listChanged", false)))
            .put(
                "serverInfo",
                JSONObject()
                    .put("name", "elon-phone-debug")
                    .put("title", "Elon Phone Debug MCP")
                    .put("version", BuildConfig.VERSION_NAME)
            )
            .put(
                "instructions",
                "Use adb forward tcp:$PORT tcp:$PORT, then call tools with the token from GET /health or logcat tag $TAG."
            )
    }

    private fun toolsListResult(): JSONObject {
        return JSONObject().put(
            "tools",
            JSONArray()
                .put(
                    tool(
                        name = "phone_status",
                        title = "Phone Status",
                        description = "Return app, MCP server, active project, and pending task state from the phone.",
                        properties = JSONObject(),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "trace_recent",
                        title = "Recent Trace Events",
                        description = "Return recent persisted phone trace events written to logcat tag ElonTrace, optionally filtered by trace id, phase, text, or wall time.",
                        properties = JSONObject()
                            .put("limit", intProperty("Maximum events to return, 1-300."))
                            .put("trace_id", stringProperty("Optional trace id filter."))
                            .put("phase", stringProperty("Optional exact phase filter."))
                            .put("contains", stringProperty("Optional case-insensitive text filter across phase and details."))
                            .put("since_wall_time_ms", intProperty("Optional lower bound for event wall_time_ms.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "trace_clear",
                        title = "Clear Trace Events",
                        description = "Clear the in-memory phone trace buffer.",
                        properties = JSONObject(),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "debug_session",
                        title = "Debug Session",
                        description = "Start, end, or inspect a named MCP debug session marker so later diagnostic bundles can return only the relevant trace window.",
                        properties = JSONObject()
                            .put("action", stringProperty("One of start, status, or end. Defaults to status."))
                            .put("session_id", stringProperty("Optional caller-provided session id for action=start."))
                            .put("note", stringProperty("Optional human note stored with the session.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "diagnostic_bundle",
                        title = "Diagnostic Bundle",
                        description = "Return one compact APK debug bundle: status, self-check, device snapshot, network check, task status/events, filtered trace, logcat, and metrics.",
                        properties = JSONObject()
                            .put("start_session", booleanProperty("Start a debug session before collecting the bundle. Defaults to false."))
                            .put("session_id", stringProperty("Optional debug session id when start_session=true."))
                            .put("note", stringProperty("Optional debug session note when start_session=true."))
                            .put("since_wall_time_ms", intProperty("Only include trace events after this wall time. Defaults to the active debug session start."))
                            .put("trace_limit", intProperty("Maximum trace events to return, 1-300. Defaults to 80."))
                            .put("trace_id", stringProperty("Optional trace id filter for trace_recent."))
                            .put("phase", stringProperty("Optional exact phase filter for trace_recent."))
                            .put("contains", stringProperty("Optional trace text filter."))
                            .put("include_logcat", booleanProperty("Include filtered logcat. Defaults to true."))
                            .put("logcat_line_count", intProperty("Raw logcat lines to scan, 20-1000. Defaults to 240."))
                            .put("logcat_pattern", stringProperty("Regex filter for logcat. Defaults to Elon/crash/foreground-service tags."))
                            .put("include_network_check", booleanProperty("Include backend network probes. Defaults to true."))
                            .put("include_update_check", booleanProperty("Include server version.json in self-check. Defaults to false."))
                            .put("task_event_limit", intProperty("Maximum queued task events to return, 1-120. Defaults to 20.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "device_snapshot",
                        title = "Device Snapshot",
                        description = "Return local device/app runtime facts useful for debugging: memory, battery, network capabilities, build info, and keepalive status.",
                        properties = JSONObject(),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "network_check",
                        title = "Network Check",
                        description = "Probe backend HTTP endpoints and TCP reachability from inside the APK process.",
                        properties = JSONObject()
                            .put("urls", arrayProperty("Optional HTTP URLs to probe. Defaults to server /health and /app/version.json."))
                            .put("tcp_host", stringProperty("Optional TCP host to probe. Defaults to 43.139.149.158."))
                            .put("tcp_port", intProperty("Optional TCP port to probe. Defaults to 8080.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "mcp_self_check",
                        title = "MCP Self Check",
                        description = "Run a one-shot local health check for MCP server reachability, background keepalive, trace storage, queued events, and request metrics.",
                        properties = JSONObject()
                            .put("include_update_check", booleanProperty("Also fetch server version.json. Defaults to false.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "mcp_metrics",
                        title = "MCP Metrics",
                        description = "Return MCP request counters, active connections, last RPC/tool names, and last request error.",
                        properties = JSONObject(),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "debug_keepalive",
                        title = "Debug Keepalive",
                        description = "Start, stop, or inspect the foreground debug keepalive service so MCP stays reachable while the user is in another app.",
                        properties = JSONObject()
                            .put("action", stringProperty("One of start, stop, or status. Defaults to status.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "update_status",
                        title = "APK Update Status",
                        description = "Compare the installed APK version with the server version.json.",
                        properties = JSONObject()
                            .put("server_url", stringProperty("Optional version.json URL.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "task_status",
                        title = "Task Status",
                        description = "Return the active or most recent task status with timing milestones and last message preview.",
                        properties = JSONObject()
                            .put("trace_id", stringProperty("Optional trace id. Defaults to current pending task or latest traced task.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "task_control",
                        title = "Task Control",
                        description = "Inspect or pause the active phone task from inside the APK process.",
                        properties = JSONObject()
                            .put("action", stringProperty("One of status or pause. Defaults to status.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "task_events",
                        title = "Queued Task Events",
                        description = "Return raw task events queued while the UI was in the background, with an option to clear them after reading.",
                        properties = JSONObject()
                            .put("limit", intProperty("Maximum queued events to return, 1-120. Defaults to 40."))
                            .put("clear", booleanProperty("Clear queued events after reading. Defaults to false.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "logcat_recent",
                        title = "Recent Logcat",
                        description = "Return filtered recent logcat lines visible to the APK process.",
                        properties = JSONObject()
                            .put("line_count", intProperty("Raw logcat lines to scan, 20-1000. Defaults to 300."))
                            .put("pattern", stringProperty("Regex filter. Defaults to Elon/AndroidRuntime crash tags.")),
                        required = JSONArray()
                    )
                )
                .put(
                    tool(
                        name = "chat_send",
                        title = "Send Chat",
                        description = "Queue a chat request on the phone through the same TaskWorkService path used by the UI.",
                        properties = JSONObject()
                            .put("message", stringProperty("Chat message to send from the phone."))
                            .put("project_id", stringProperty("Optional project id. Defaults to the active project."))
                            .put("project_title", stringProperty("Optional project title."))
                            .put("conversation_id", stringProperty("Optional conversation id for native CLI session continuity."))
                            .put("conversation_title", stringProperty("Optional conversation title."))
                            .put("agent", stringProperty("Optional backend agent id, such as codex_cli."))
                            .put("trace_id", stringProperty("Optional caller-provided trace id."))
                            .put("is_development", booleanProperty("Whether this should be treated as a development task."))
                            .put("force", booleanProperty("Override an active phone task. Defaults to false.")),
                        required = JSONArray().put("message")
                    )
                )
        )
    }

    private fun tool(
        name: String,
        title: String,
        description: String,
        properties: JSONObject,
        required: JSONArray
    ): JSONObject {
        val mergedProperties = JSONObject()
            .put("auth_token", stringProperty("MCP debug token from GET /health or logcat tag ElonMcpServer."))
        properties.keys().forEach { key -> mergedProperties.put(key, properties.get(key)) }
        val mergedRequired = JSONArray().put("auth_token")
        for (index in 0 until required.length()) mergedRequired.put(required.get(index))
        return JSONObject()
            .put("name", name)
            .put("title", title)
            .put("description", description)
            .put(
                "inputSchema",
                JSONObject()
                    .put("type", "object")
                    .put("properties", mergedProperties)
                    .put("required", mergedRequired)
                    .put("additionalProperties", false)
            )
    }

    private fun stringProperty(description: String) =
        JSONObject().put("type", "string").put("description", description)

    private fun intProperty(description: String) =
        JSONObject().put("type", "integer").put("description", description)

    private fun booleanProperty(description: String) =
        JSONObject().put("type", "boolean").put("description", description)

    private fun arrayProperty(description: String) =
        JSONObject().put("type", "array").put("description", description).put("items", JSONObject().put("type", "string"))

    private fun statusJson(includeToken: Boolean): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val pendingPayload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
        val pendingTask = pendingTaskJson(prefs)
        val pendingBusy = isTaskBusy(prefs)
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
            .put("app_foreground", prefs.getBoolean(TaskWorkService.PREF_APP_IN_FOREGROUND, false))
            .put("background_debug_supported", true)
            .put("trace_persistence", "shared_preferences")
            .put("debug_keepalive", McpDebugKeepAliveService.statusJson(appContext))
            .put("user_id", prefs.getString(TaskWorkService.PREF_USER_ID, null))
            .put("active_project_id", prefs.getString(TaskWorkService.PREF_ACTIVE_PROJECT_ID, null))
            .put("pending_work", pendingBusy)
            .put("busy", pendingBusy)
            .put("active_trace_id", if (pendingBusy) pendingTask?.optString("trace_id")?.takeIf { it.isNotBlank() } else null)
            .put("active_task_kind", if (pendingBusy && !pendingPayload.isNullOrBlank()) pendingTaskKind(prefs) else null)
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
            ?: latestTraceId(events)
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
        val status = when {
            finish?.phase == "task_finish_done" -> "done"
            finish?.phase == "task_finish_error" -> "error"
            rejected != null && traceEvents.none { it.phase == "task_start_work" } -> "rejected_busy"
            isTaskBusy(prefs) && traceId != null && pendingTask?.optString("trace_id") == traceId -> "running"
            traceEvents.any { it.phase == "task_payload_sent" } -> "sent"
            traceEvents.any { it.phase == "task_start_work" } -> "started"
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
            .put("sent_elapsed_ms", detailLong(traceEvents, "task_payload_sent", "elapsed_ms"))
            .put("first_server_event_elapsed_ms", detailLong(traceEvents, "task_first_server_event", "elapsed_ms"))
            .put("first_chat_reply_elapsed_ms", detailLong(traceEvents, "task_first_chat_reply", "elapsed_ms"))
            .put("finish_elapsed_ms", finish?.details?.get("elapsed_ms")?.toLongOrNull() ?: JSONObject.NULL)
            .put("last_message_type", lastMessage?.details?.get("type") ?: JSONObject.NULL)
            .put("last_message_preview", lastMessage?.details?.get("message_preview") ?: JSONObject.NULL)
            .put("has_apk_url", finish?.details?.get("has_apk_url") ?: JSONObject.NULL)
            .put("last_phase", traceEvents.lastOrNull()?.phase ?: JSONObject.NULL)
            .put("last_event_wall_time_ms", traceEvents.lastOrNull()?.wallTimeMs ?: JSONObject.NULL)
    }

    private fun pendingWorkAgeMs(prefs: SharedPreferences): Long? {
        val savedAt = prefs.getLong(TaskWorkService.PREF_PENDING_WORK_TIME, 0L)
        return if (savedAt > 0L) System.currentTimeMillis() - savedAt else null
    }

    private fun isTaskBusy(prefs: SharedPreferences): Boolean {
        val payload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
            ?.takeIf { it.isNotBlank() }
            ?: return false
        val savedAt = prefs.getLong(TaskWorkService.PREF_PENDING_WORK_TIME, 0L)
        val expired = savedAt > 0L && System.currentTimeMillis() - savedAt > TaskWorkService.PENDING_WORK_TTL_MS
        return !expired && payload.isNotBlank()
    }

    private fun pendingTaskJson(prefs: SharedPreferences): JSONObject? {
        val payload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
            ?.takeIf { it.isNotBlank() }
            ?: return null
        return runCatching { JSONObject(payload) }.getOrNull()
    }

    private fun pendingTaskKind(prefs: SharedPreferences): String {
        if (!isTaskBusy(prefs)) return "idle"
        return if (prefs.getBoolean(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT, true)) {
            "development"
        } else {
            "chat"
        }
    }

    private fun reservePendingTask(prefs: SharedPreferences, payload: String, isDevelopment: Boolean) {
        prefs.edit()
            .putString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, payload)
            .putBoolean(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT, isDevelopment)
            .putLong(TaskWorkService.PREF_PENDING_WORK_TIME, System.currentTimeMillis())
            .apply()
    }

    private fun clearPersistedTask(prefs: SharedPreferences) {
        prefs.edit()
            .remove(TaskWorkService.PREF_PENDING_WORK_PAYLOAD)
            .remove(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT)
            .remove(TaskWorkService.PREF_PENDING_WORK_TIME)
            .apply()
    }

    private fun clearReservedPendingTask(prefs: SharedPreferences, traceId: String) {
        val currentTraceId = traceIdFromPayload(
            prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
        )
        if (currentTraceId == traceId) clearPersistedTask(prefs)
    }

    private fun traceIdFromPayload(payload: String?): String? {
        return payload
            ?.let { runCatching { JSONObject(it).optString("trace_id") }.getOrNull() }
            ?.takeIf { it.isNotBlank() }
    }

    private fun latestTraceId(events: List<DebugTraceStore.TraceEvent>): String? {
        return events.asReversed()
            .firstNotNullOfOrNull { it.details["trace_id"]?.takeIf(String::isNotBlank) }
    }

    private fun detailLong(
        events: List<DebugTraceStore.TraceEvent>,
        phase: String,
        key: String
    ): Any {
        return events.firstOrNull { it.phase == phase }
            ?.details
            ?.get(key)
            ?.toLongOrNull()
            ?: JSONObject.NULL
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

    private fun queuedTaskEvents(prefs: SharedPreferences): List<String> {
        val raw = prefs.getString(TaskWorkService.PREF_QUEUED_TASK_EVENTS, null)
            ?.takeIf { it.isNotBlank() }
            ?: return emptyList()
        val array = runCatching { JSONArray(raw) }.getOrElse { return emptyList() }
        return buildList {
            for (index in 0 until array.length()) {
                array.optString(index).takeIf { it.isNotBlank() }?.let { add(it) }
            }
        }
    }

    private fun rawTaskEventJson(raw: String): JSONObject {
        val parsed = runCatching { JSONObject(raw) }.getOrNull()
        return JSONObject()
            .put("raw", raw.take(20_000))
            .put("json", parsed ?: JSONObject.NULL)
    }

    private fun traceEventsJson(events: List<DebugTraceStore.TraceEvent>): JSONArray {
        return JSONArray().apply {
            events.forEach { event ->
                put(
                    JSONObject()
                        .put("wall_time_ms", event.wallTimeMs)
                        .put("elapsed_ms", event.elapsedMs)
                        .put("phase", event.phase)
                        .put("details", JSONObject().apply {
                            event.details.forEach { (key, value) -> put(key, value) }
                        })
                )
            }
        }
    }

    private fun traceEventSearchText(event: DebugTraceStore.TraceEvent): String {
        return buildString {
            append(event.phase)
            event.details.forEach { (key, value) ->
                append(' ').append(key).append('=').append(value)
            }
        }
    }

    private fun startDebugSession(
        prefs: SharedPreferences,
        requestedSessionId: String?,
        note: String?
    ): JSONObject {
        val sessionId = requestedSessionId ?: "mcp_session_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}"
        val startedAt = System.currentTimeMillis()
        prefs.edit()
            .putString(PREF_DEBUG_SESSION_ID, sessionId)
            .putLong(PREF_DEBUG_SESSION_STARTED_AT, startedAt)
            .apply {
                if (note == null) remove(PREF_DEBUG_SESSION_NOTE) else putString(PREF_DEBUG_SESSION_NOTE, note)
            }
            .apply()
        DebugTraceStore.record(
            "mcp_debug_session_start",
            mapOf("debug_session_id" to sessionId, "note" to note)
        )
        return debugSessionJson(prefs)
    }

    private fun debugSessionJson(prefs: SharedPreferences): JSONObject {
        val sessionId = prefs.getString(PREF_DEBUG_SESSION_ID, null)?.takeIf { it.isNotBlank() }
        val startedAt = prefs.getLong(PREF_DEBUG_SESSION_STARTED_AT, 0L).takeIf { it > 0L }
        return JSONObject()
            .put("active", sessionId != null && startedAt != null)
            .put("session_id", sessionId ?: JSONObject.NULL)
            .put("started_at_ms", startedAt ?: JSONObject.NULL)
            .put("age_ms", startedAt?.let { System.currentTimeMillis() - it } ?: JSONObject.NULL)
            .put("note", prefs.getString(PREF_DEBUG_SESSION_NOTE, null) ?: JSONObject.NULL)
    }

    private fun deviceSnapshotJson(): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        return JSONObject()
            .put("wall_time_ms", System.currentTimeMillis())
            .put("elapsed_realtime_ms", SystemClock.elapsedRealtime())
            .put("timezone", TimeZone.getDefault().id)
            .put("app", statusJson(includeToken = false))
            .put("debug_session", debugSessionJson(prefs))
            .put("memory", memoryJson())
            .put("battery", batteryJson())
            .put("network", networkCapabilitiesJson())
            .put("build", buildJson())
    }

    private fun memoryJson(): JSONObject {
        val runtime = Runtime.getRuntime()
        val activityManager = appContext.getSystemService(ActivityManager::class.java)
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

    private fun batteryJson(): JSONObject {
        val intent = appContext.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
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

    private fun networkCapabilitiesJson(): JSONObject {
        val connectivity = appContext.getSystemService(ConnectivityManager::class.java)
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

    private fun buildJson(): JSONObject {
        return JSONObject()
            .put("manufacturer", Build.MANUFACTURER)
            .put("brand", Build.BRAND)
            .put("model", Build.MODEL)
            .put("device", Build.DEVICE)
            .put("sdk_int", Build.VERSION.SDK_INT)
            .put("release", Build.VERSION.RELEASE)
            .put("supported_abis", JSONArray().apply { Build.SUPPORTED_ABIS.forEach { put(it) } })
    }

    private fun networkCheckJson(args: JSONObject): JSONObject {
        val urls = urlsFromArgs(args)
        val tcpHost = args.optString("tcp_host").takeIf { it.isNotBlank() } ?: "43.139.149.158"
        val tcpPort = args.optInt("tcp_port", 8080).takeIf { it in 1..65535 } ?: 8080
        return JSONObject()
            .put("network", networkCapabilitiesJson())
            .put("tcp_probe", tcpProbe(tcpHost, tcpPort))
            .put("http_probes", JSONArray().apply { urls.forEach { put(httpProbe(it)) } })
    }

    private fun urlsFromArgs(args: JSONObject): List<String> {
        val array = args.optJSONArray("urls")
        if (array != null && array.length() > 0) {
            return buildList {
                for (index in 0 until array.length()) {
                    array.optString(index).takeIf { it.startsWith("http://") || it.startsWith("https://") }?.let { add(it) }
                }
            }.ifEmpty { defaultProbeUrls() }
        }
        return defaultProbeUrls()
    }

    private fun defaultProbeUrls() = listOf(
        "http://43.139.149.158:8080/health",
        "http://43.139.149.158:8080/app/version.json"
    )

    private fun tcpProbe(host: String, port: Int): JSONObject {
        val started = SystemClock.elapsedRealtime()
        return runCatching {
            Socket().use { socket ->
                socket.connect(InetSocketAddress(host, port), 5_000)
            }
            JSONObject()
                .put("host", host)
                .put("port", port)
                .put("ok", true)
                .put("duration_ms", SystemClock.elapsedRealtime() - started)
        }.getOrElse { error ->
            JSONObject()
                .put("host", host)
                .put("port", port)
                .put("ok", false)
                .put("duration_ms", SystemClock.elapsedRealtime() - started)
                .put("error", error.message ?: error.javaClass.simpleName)
        }
    }

    private fun httpProbe(url: String): JSONObject {
        val started = SystemClock.elapsedRealtime()
        return runCatching {
            val connection = (URL(url).openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = 5_000
                readTimeout = 5_000
            }
            try {
                val code = connection.responseCode
                val stream = if (code in 200..299) connection.inputStream else connection.errorStream
                val preview = stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText().take(512) }
                JSONObject()
                    .put("url", url)
                    .put("ok", code in 200..299)
                    .put("status_code", code)
                    .put("duration_ms", SystemClock.elapsedRealtime() - started)
                    .put("content_type", connection.contentType ?: JSONObject.NULL)
                    .put("body_preview", preview ?: JSONObject.NULL)
            } finally {
                connection.disconnect()
            }
        }.getOrElse { error ->
            JSONObject()
                .put("url", url)
                .put("ok", false)
                .put("duration_ms", SystemClock.elapsedRealtime() - started)
                .put("error", error.message ?: error.javaClass.simpleName)
        }
    }

    private fun authorized(headers: Map<String, String>, args: JSONObject): Boolean {
        val expected = debugToken()
        val bearer = headers["authorization"]
            ?.trim()
            ?.takeIf { it.lowercase(Locale.ROOT).startsWith("bearer ") }
            ?.substringAfter(' ')
        val customHeader = headers["x-elon-mcp-token"]?.trim()
        val argToken = args.optString("auth_token").takeIf { it.isNotBlank() }
        return listOfNotNull(bearer, customHeader, argToken).any { constantTimeEquals(it, expected) }
    }

    private fun debugToken(): String {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        return prefs.getString(PREF_MCP_TOKEN, null)
            ?: UUID.randomUUID().toString().replace("-", "").also {
                prefs.edit().putString(PREF_MCP_TOKEN, it).apply()
            }
    }

    private fun constantTimeEquals(left: String, right: String): Boolean {
        val a = left.toByteArray()
        val b = right.toByteArray()
        if (a.size != b.size) return false
        var diff = 0
        for (index in a.indices) diff = diff or (a[index].toInt() xor b[index].toInt())
        return diff == 0
    }

    private fun fetchJson(url: String): JSONObject? {
        return runCatching {
            val connection = (URL(url).openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = 5_000
                readTimeout = 5_000
            }
            try {
                if (connection.responseCode !in 200..299) return@runCatching null
                val body = connection.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
                JSONObject(body)
            } finally {
                connection.disconnect()
            }
        }.getOrNull()
    }

    private fun redactLogLine(line: String): String {
        return line
            .replace(Regex("MCP debug token: [A-Za-z0-9._-]+"), "MCP debug token: <redacted>")
            .replace(Regex("(?i)(auth[_-]?token[\\\"'=:\\s]+)[A-Za-z0-9._-]+")) {
                it.groupValues[1] + "<redacted>"
            }
    }

    private fun rpcResult(id: Any, result: JSONObject): JSONObject {
        return JSONObject()
            .put("jsonrpc", "2.0")
            .put("id", id)
            .put("result", result)
    }

    private fun rpcError(id: Any?, code: Int, message: String): JSONObject {
        return JSONObject()
            .put("jsonrpc", "2.0")
            .put("id", id ?: JSONObject.NULL)
            .put("error", JSONObject().put("code", code).put("message", message))
    }

    private fun toolResult(message: String, structured: JSONObject, isError: Boolean = false): JSONObject {
        return JSONObject()
            .put(
                "content",
                JSONArray().put(JSONObject().put("type", "text").put("text", message))
            )
            .put("structuredContent", structured)
            .put("isError", isError)
    }

    private fun jsonError(code: String, message: String): String {
        return JSONObject().put("error", code).put("message", message).toString()
    }

    private fun readRequest(socket: Socket): HttpRequest? {
        val input = socket.getInputStream()
        val headerBytes = ByteArrayOutputStream()
        val window = java.util.ArrayDeque<Int>(4)
        while (headerBytes.size() <= MAX_HEADER_BYTES) {
            val byte = input.read()
            if (byte < 0) return null
            headerBytes.write(byte)
            window.addLast(byte)
            if (window.size > 4) window.removeFirst()
            if (window.size == 4 && window.toList() == listOf(13, 10, 13, 10)) break
        }
        val headerText = String(headerBytes.toByteArray(), Charsets.UTF_8)
        val lines = headerText.split("\r\n").filter { it.isNotBlank() }
        val requestLine = lines.firstOrNull() ?: return null
        val parts = requestLine.split(' ')
        if (parts.size < 2) return null
        val headers = mutableMapOf<String, String>()
        for (line in lines.drop(1)) {
            val separator = line.indexOf(':')
            if (separator > 0) {
                headers[line.substring(0, separator).trim().lowercase(Locale.ROOT)] =
                    line.substring(separator + 1).trim()
            }
        }
        val length = headers["content-length"]?.toIntOrNull()?.coerceAtMost(MAX_BODY_BYTES) ?: 0
        val bodyBytes = ByteArray(length)
        var offset = 0
        while (offset < length) {
            val read = input.read(bodyBytes, offset, length - offset)
            if (read < 0) break
            offset += read
        }
        return HttpRequest(
            method = parts[0].uppercase(Locale.ROOT),
            path = parts[1].substringBefore('?'),
            headers = headers,
            body = String(bodyBytes.copyOf(offset), Charsets.UTF_8)
        )
    }

    private fun writeResponse(
        socket: Socket,
        status: Int,
        reason: String,
        body: String,
        contentType: String = "application/json; charset=utf-8"
    ) {
        val bodyBytes = body.toByteArray(Charsets.UTF_8)
        val headers = buildString {
            append("HTTP/1.1 ").append(status).append(' ').append(reason).append("\r\n")
            append("Content-Type: ").append(contentType).append("\r\n")
            append("Content-Length: ").append(bodyBytes.size).append("\r\n")
            append("Connection: close\r\n")
            append("MCP-Protocol-Version: ").append(PROTOCOL_VERSION).append("\r\n")
            append("\r\n")
        }
        socket.getOutputStream().use { output ->
            output.write(headers.toByteArray(Charsets.UTF_8))
            if (bodyBytes.isNotEmpty()) output.write(bodyBytes)
            output.flush()
        }
    }

    private data class HttpRequest(
        val method: String,
        val path: String,
        val headers: Map<String, String>,
        val body: String
    )
}
