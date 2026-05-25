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
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.URL
import java.util.TimeZone
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
            "task_status" -> mcpTaskStatus(appContext, args)
            "task_control" -> mcpTaskControl(appContext, args)
            "task_events" -> mcpTaskEvents(appContext, args)
            "logcat_recent" -> mcpLogcatRecent(args)
            "chat_send" -> chatSend(appContext, args)
            "chat_probe" -> mcpChatProbe(appContext, args, DEFAULT_SERVER_BASE_URL) { bundleArgs ->
                diagnosticBundle(bundleArgs).optJSONObject("structuredContent") ?: JSONObject()
            }
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
        val taskEvents = mcpTaskEvents(appContext, JSONObject().put("limit", args.optInt("task_event_limit", 20).coerceIn(1, 120)))
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
        val taskStatus = taskStatusJson(appContext, taskStatusArgs)
        val bundleTraceId = args.optString("trace_id").takeIf { it.isNotBlank() }
            ?: taskStatus.optString("trace_id").takeIf { it.isNotBlank() && it != "null" }
            ?: mcpLatestTraceId(DebugTraceStore.recentEvents(300))
        val latencyArgs = JSONObject()
            .put("timeline_limit", args.optInt("timeline_limit", 80).coerceIn(1, 300))
        bundleTraceId?.let {
            latencyArgs.put("trace_id", it)
        }
        val latency = latencyReportJson(appContext, latencyArgs)
        val serverTrace = if (includeServerTrace) {
            JSONObject()
                .put("trace_id", bundleTraceId ?: "")
                .put("limit", serverTraceLimit)
                .apply {
                    args.optString("server_url").takeIf { it.isNotBlank() }?.let { put("server_url", it) }
                }
                .let { serverTraceJson(appContext, it, DEFAULT_SERVER_BASE_URL) }
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
        return toolResult("Latency report returned.", latencyReportJson(appContext, args))
    }

    private fun serverTrace(args: JSONObject): JSONObject {
        val structured = serverTraceJson(appContext, args, DEFAULT_SERVER_BASE_URL)
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
        val taskStatus = taskStatusJson(appContext, JSONObject())
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

