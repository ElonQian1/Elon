package com.elon.app

import android.content.Context
import android.content.Intent
import android.os.SystemClock
import android.util.Log
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.URL
import java.util.Locale
import java.util.UUID
import java.util.concurrent.Executors

object McpDebugServer {
    private const val TAG = "ElonMcpServer"
    private const val HOST = "127.0.0.1"
    private const val PORT = 8787
    private const val PROTOCOL_VERSION = "2025-06-18"
    private const val PREF_MCP_TOKEN = "mcp_debug_token"
    private const val MAX_BODY_BYTES = 256 * 1024

    @Volatile private var running = false
    @Volatile private var serverSocket: ServerSocket? = null
    private val workers = Executors.newCachedThreadPool()
    private val processStartedElapsedMs = SystemClock.elapsedRealtime()
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
        val token = debugToken()
        try {
            ServerSocket(PORT, 50, InetAddress.getByName(HOST)).use { server ->
                serverSocket = server
                DebugTraceStore.record(
                    "mcp_server_started",
                    mapOf("host" to HOST, "port" to PORT, "token_ready" to true)
                )
                Log.i(TAG, "MCP endpoint: adb forward tcp:$PORT tcp:$PORT then http://$HOST:$PORT/mcp")
                Log.i(TAG, "MCP debug token: $token")
                while (running) {
                    val socket = try {
                        server.accept()
                    } catch (_: Exception) {
                        if (running) Log.w(TAG, "accept failed")
                        break
                    }
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
        socket.use {
            val request = readRequest(it) ?: run {
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
        }
    }

    private fun handleMcpPost(socket: Socket, request: HttpRequest) {
        val rpc = runCatching { JSONObject(request.body) }.getOrNull()
        if (rpc == null || rpc.optString("jsonrpc") != "2.0") {
            writeResponse(socket, 400, "Bad Request", rpcError(null, -32600, "Invalid JSON-RPC request").toString())
            return
        }

        val method = rpc.optString("method")
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

        val result = when (name) {
            "phone_status" -> toolResult("Phone MCP debug server is running.", statusJson(includeToken = false))
            "trace_recent" -> traceRecent(args)
            "trace_clear" -> {
                DebugTraceStore.clear()
                toolResult("Trace buffer cleared.", JSONObject().put("cleared", true))
            }
            "debug_keepalive" -> debugKeepalive(args)
            "update_status" -> updateStatus(args)
            "chat_send" -> chatSend(args)
            else -> toolResult("Unknown tool: $name", JSONObject().put("tool", name), isError = true)
        }
        return rpcResult(id, result)
    }

    private fun traceRecent(args: JSONObject): JSONObject {
        val limit = args.optInt("limit", 80).coerceIn(1, 300)
        val structured = JSONObject()
            .put("events", DebugTraceStore.recent(limit))
            .put("limit", limit)
        return toolResult("Returned $limit recent trace events.", structured)
    }

    private fun debugKeepalive(args: JSONObject): JSONObject {
        val action = args.optString("action", "status").lowercase(Locale.ROOT)
        when (action) {
            "start" -> {
                appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
                    .edit()
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

    private fun chatSend(args: JSONObject): JSONObject {
        val message = args.optString("message").trim()
        if (message.isEmpty()) {
            return toolResult("message is required", JSONObject().put("field", "message"), isError = true)
        }
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
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
        val isDevelopment = if (args.has("is_development")) args.optBoolean("is_development") else true

        val payload = JSONObject()
            .put("trace_id", traceId)
            .put("user_id", userId)
            .put("project_id", projectId)
            .put("project_title", projectTitle)
            .put("message", message)
        if (agent != null) payload.put("agent", agent)

        DebugTraceStore.record(
            "mcp_chat_send",
            mapOf("trace_id" to traceId, "project_id" to projectId, "chars" to message.length)
        )
        val intent = Intent(appContext, TaskWorkService::class.java).apply {
            action = TaskWorkService.ACTION_START_WORK
            putExtra(TaskWorkService.EXTRA_PAYLOAD, payload.toString())
            putExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, isDevelopment)
        }
        ContextCompat.startForegroundService(appContext, intent)

        val structured = JSONObject()
            .put("trace_id", traceId)
            .put("project_id", projectId)
            .put("project_title", projectTitle)
            .put("is_development", isDevelopment)
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
                        description = "Return recent persisted phone trace events written to logcat tag ElonTrace.",
                        properties = JSONObject().put("limit", intProperty("Maximum events to return, 1-300.")),
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
                        name = "chat_send",
                        title = "Send Chat",
                        description = "Queue a chat request on the phone through the same TaskWorkService path used by the UI.",
                        properties = JSONObject()
                            .put("message", stringProperty("Chat message to send from the phone."))
                            .put("project_id", stringProperty("Optional project id. Defaults to the active project."))
                            .put("project_title", stringProperty("Optional project title."))
                            .put("agent", stringProperty("Optional backend agent id, such as codex_cli."))
                            .put("trace_id", stringProperty("Optional caller-provided trace id."))
                            .put("is_development", booleanProperty("Whether this should be treated as a development task.")),
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

    private fun statusJson(includeToken: Boolean): JSONObject {
        val prefs = appContext.getSharedPreferences("elon", Context.MODE_PRIVATE)
        val pendingPayload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
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
            .put("pending_work", !pendingPayload.isNullOrBlank())
            .put("pending_work_age_ms", pendingWorkAgeMs(prefs))
            .put("trace_events", DebugTraceStore.count())
            .apply {
                if (includeToken) put("auth_token", debugToken())
            }
    }

    private fun pendingWorkAgeMs(prefs: android.content.SharedPreferences): Long? {
        val savedAt = prefs.getLong(TaskWorkService.PREF_PENDING_WORK_TIME, 0L)
        return if (savedAt > 0L) System.currentTimeMillis() - savedAt else null
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
        while (headerBytes.size() <= 16 * 1024) {
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
