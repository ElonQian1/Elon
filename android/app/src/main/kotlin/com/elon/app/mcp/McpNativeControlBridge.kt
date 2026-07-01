package com.elon.app.mcp

import com.elon.app.DebugTraceStore
import com.elon.app.MainActivity
import android.content.Context
import android.content.Intent
import android.os.SystemClock
import androidx.core.content.ContextCompat
import org.json.JSONObject

internal object McpNativeControlBridge {
    private const val TOKEN_PREF = "mcp_debug_token"
    private const val DEFAULT_OPEN_MAIN_BIND_WAIT_MS = 1_200L

    interface Controller {
        fun uiState(): JSONObject
        fun control(args: JSONObject): JSONObject
    }

    @Volatile private var controller: Controller? = null

    fun register(controller: Controller) {
        this.controller = controller
        DebugTraceStore.record("mcp_native_control_bound")
    }

    fun unregister(controller: Controller) {
        if (this.controller === controller) {
            this.controller = null
            DebugTraceStore.record("mcp_native_control_unbound")
        }
    }

    fun debugToken(context: Context): String = mcpDebugToken(context, TOKEN_PREF)

    fun uiState(context: Context, @Suppress("UNUSED_PARAMETER") args: JSONObject): JSONObject {
        val current = controller
        val state = current?.uiState() ?: JSONObject()
        return state
            .put("schema", "elon.apk.native_mcp_ui_state.v1")
            .put("activity_bound", current != null)
            .put("package_name", context.packageName)
            .put("mcp_endpoint", "http://127.0.0.1:8787/mcp")
            .put("adb_forward", "adb forward tcp:8787 tcp:8787")
            .put("generated_at_ms", System.currentTimeMillis())
    }

    fun control(context: Context, args: JSONObject): JSONObject {
        val action = args.optString("action", "state").trim().lowercase()
        if (action == "open_main") {
            val openError = runCatching { openMainActivity(context) }.exceptionOrNull()
            val waitMs = args.optLong("wait_for_bind_ms", DEFAULT_OPEN_MAIN_BIND_WAIT_MS)
                .coerceIn(0L, 5_000L)
            val boundAfterOpen = waitForController(waitMs) != null
            return uiState(context, args)
                .put("action", action)
                .put("opened_main_activity", openError == null)
                .put("activity_bound_after_open", boundAfterOpen)
                .apply {
                    if (openError != null) {
                        put("error", "open_main_failed")
                        put("message", openError.message ?: openError.javaClass.simpleName)
                    } else if (!boundAfterOpen) {
                        put("error", "main_activity_not_bound_after_open")
                        put(
                            "hint",
                            "Android may block background activity starts from the APK service. Start MainActivity through adb, then retry the MCP UI control."
                        )
                        put("adb_start_activity", "adb shell am start -n com.elon.app/.MainActivity")
                    }
                }
        }
        val current = controller ?: return JSONObject()
            .put("schema", "elon.apk.native_mcp_control_result.v1")
            .put("action", action)
            .put("activity_bound", false)
            .put("error", "main_activity_not_bound")
            .put("hint", "Call action=open_main, then retry after MainActivity registers the native MCP controller.")
        return current.control(args)
            .put("schema", "elon.apk.native_mcp_control_result.v1")
            .put("action", action)
            .put("activity_bound", true)
    }

    fun bootstrapJson(context: Context, action: String, serviceStarted: Boolean, error: String?): JSONObject {
        return JSONObject()
            .put("schema", "elon.apk.mcp_bootstrap.v1")
            .put("action", action)
            .put("package_name", context.packageName)
            .put("service_started", serviceStarted)
            .put("error", error ?: JSONObject.NULL)
            .put("mcp_endpoint", "http://127.0.0.1:8787/mcp")
            .put("mcp_health", "http://127.0.0.1:8787/health")
            .put("adb_forward", "adb forward tcp:8787 tcp:8787")
            .put("auth_token", debugToken(context))
            .put("generated_at_ms", System.currentTimeMillis())
    }

    private fun openMainActivity(context: Context) {
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or
                Intent.FLAG_ACTIVITY_CLEAR_TOP or
                Intent.FLAG_ACTIVITY_SINGLE_TOP
            putExtra("mcp_open_main", true)
        }
        ContextCompat.startActivity(context, intent, null)
        DebugTraceStore.record("mcp_native_control_open_main")
    }

    private fun waitForController(timeoutMs: Long): Controller? {
        val deadline = SystemClock.elapsedRealtime() + timeoutMs
        do {
            controller?.let { return it }
            if (timeoutMs <= 0L) return null
            SystemClock.sleep(100L)
        } while (SystemClock.elapsedRealtime() < deadline)
        return controller
    }
}

internal fun mcpUiState(context: Context, args: JSONObject): JSONObject {
    return toolResult("Native APK UI state returned.", McpNativeControlBridge.uiState(context, args))
}

internal fun mcpUiControl(context: Context, args: JSONObject): JSONObject {
    val structured = McpNativeControlBridge.control(context, args)
    val failed = structured.has("error")
    return toolResult(
        if (failed) "Native APK UI control failed." else "Native APK UI control executed.",
        structured,
        isError = failed
    )
}
