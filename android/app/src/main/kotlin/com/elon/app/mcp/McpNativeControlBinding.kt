package com.elon.app.mcp

import android.app.Activity
import android.os.Looper
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.json.JSONObject

internal class McpNativeControlBinding(
    private val activity: Activity,
    private val uiState: () -> JSONObject,
    private val control: (JSONObject) -> JSONObject,
) {
    private val controller = object : McpNativeControlBridge.Controller {
        override fun uiState(): JSONObject = runOnMain(DEFAULT_TIMEOUT_MS, uiState)

        override fun control(args: JSONObject): JSONObject = runOnMain(
            args.optLong("main_thread_timeout_ms", DEFAULT_TIMEOUT_MS)
                .coerceIn(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS),
        ) {
            control(args)
        }
    }

    fun register() {
        McpNativeControlBridge.register(controller)
    }

    fun unregister() {
        McpNativeControlBridge.unregister(controller)
    }

    private fun runOnMain(timeoutMs: Long, action: () -> JSONObject): JSONObject {
        if (Looper.myLooper() == Looper.getMainLooper()) return action()
        var result: JSONObject? = null
        var error: Throwable? = null
        val latch = CountDownLatch(1)
        activity.runOnUiThread {
            try {
                result = action()
            } catch (failure: Throwable) {
                error = failure
            } finally {
                latch.countDown()
            }
        }
        if (!latch.await(timeoutMs, TimeUnit.MILLISECONDS)) {
            return failure("main_thread_timeout").put("timeout_ms", timeoutMs)
        }
        error?.let {
            return failure(it.javaClass.simpleName).put("message", it.message ?: "")
        }
        return result ?: failure("empty_result")
    }

    private fun failure(code: String): JSONObject = JSONObject()
        .put("control_ok", false)
        .put("error", code)

    private companion object {
        const val DEFAULT_TIMEOUT_MS = 15_000L
        const val MIN_TIMEOUT_MS = 1_000L
        const val MAX_TIMEOUT_MS = 60_000L
    }
}
