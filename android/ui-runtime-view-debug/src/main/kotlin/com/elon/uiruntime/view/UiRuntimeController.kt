package com.elon.uiruntime.view

import android.app.Application
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import com.google.gson.Gson
import com.google.gson.JsonObject

internal object UiRuntimeController {
    private const val TAG = "YilongUiRuntime"
    private val gson = Gson()
    private val mainHandler = Handler(Looper.getMainLooper())
    private var application: Application? = null
    private var registry: UiRuntimeRegistry? = null
    private var socket: UiRuntimeWebSocket? = null
    private var config: UiRuntimeSessionConfig? = null
    private var connected = false

    fun initialize(context: Context) {
        if (application != null) return
        val app = context.applicationContext as? Application ?: return
        application = app
        registry = UiRuntimeRegistry(app, ::scheduleTreeSnapshot).also { it.start() }
        Log.i(TAG, "View Debug Runtime initialized")
    }

    fun start(context: Context, sessionId: String, token: String, devicePort: Int) {
        initialize(context)
        stop()
        val next = UiRuntimeSessionConfig(sessionId, token, devicePort)
        config = next
        socket = UiRuntimeWebSocket(
            config = next,
            onMessage = ::handleMessage,
            onConnectionChanged = { isConnected, error ->
                connected = isConnected
                if (error != null) Log.w(TAG, error)
            },
        ).also { it.connect() }
    }

    fun stop() {
        connected = false
        socket?.close()
        socket = null
        config = null
        mainHandler.removeCallbacksAndMessages(null)
    }

    private fun handleMessage(text: String) {
        val root = runCatching { gson.fromJson(text, JsonObject::class.java) }
            .getOrElse {
                sendRuntimeError("无法解析 Broker 消息: ${it.message}")
                return
            }
        when (root.get("messageType")?.asString) {
            "broker.welcome" -> {
                sendHello()
                scheduleTreeSnapshot()
            }
            "patch.apply" -> {
                val patch = runCatching { gson.fromJson(root, LiveStylePatch::class.java) }
                    .getOrElse {
                        sendRuntimeError("Patch 格式错误: ${it.message}")
                        return
                    }
                applyPatch(patch)
            }
        }
    }

    private fun applyPatch(patch: LiveStylePatch) {
        mainHandler.post {
            val currentRegistry = registry ?: return@post
            val session = config ?: return@post
            val result = runCatching {
                val views = currentRegistry.resolve(patch.target)
                UiRuntimeViewAdapter.apply(views, patch.operations)
            }
            val treeRevision = currentRegistry.nextTreeRevision()
            val ack = result.fold(
                onSuccess = {
                    PatchAckMessage(
                        messageType = "patch.ack",
                        sessionId = session.sessionId,
                        requestId = patch.requestId,
                        gestureId = patch.gestureId,
                        sequence = patch.sequence,
                        status = "APPLIED",
                        newTreeRevision = treeRevision,
                        beforeValues = it.beforeValues,
                        effectiveValues = it.effectiveValues,
                        measuredGeometry = it.measuredGeometry,
                    )
                },
                onFailure = {
                    PatchAckMessage(
                        messageType = "patch.reject",
                        sessionId = session.sessionId,
                        requestId = patch.requestId,
                        gestureId = patch.gestureId,
                        sequence = patch.sequence,
                        status = "REJECTED",
                        newTreeRevision = treeRevision,
                        error = it.message ?: "Android View 无法应用 Patch",
                    )
                },
            )
            send(gson.toJson(ack))
            if (result.isSuccess) scheduleTreeSnapshot()
        }
    }

    private fun sendHello() {
        val app = application ?: return
        val session = config ?: return
        val packageInfo = runCatching {
            app.packageManager.getPackageInfo(app.packageName, 0)
        }.getOrNull()
        val versionCode = if (Build.VERSION.SDK_INT >= 28) {
            packageInfo?.longVersionCode
        } else {
            @Suppress("DEPRECATION") packageInfo?.versionCode?.toLong()
        }
        val buildId = listOfNotNull(packageInfo?.versionName, versionCode?.toString())
            .joinToString("+")
            .ifBlank { "unknown" }
        send(
            gson.toJson(
                RuntimeHelloMessage(
                    sessionId = session.sessionId,
                    packageName = app.packageName,
                    appBuildId = buildId,
                    androidSdk = Build.VERSION.SDK_INT,
                ),
            ),
        )
    }

    private fun scheduleTreeSnapshot() {
        if (!connected) return
        mainHandler.removeCallbacks(sendTreeRunnable)
        mainHandler.postDelayed(sendTreeRunnable, 80)
    }

    private val sendTreeRunnable = Runnable {
        val currentRegistry = registry ?: return@Runnable
        send(gson.toJson(currentRegistry.snapshot()))
    }

    private fun sendRuntimeError(error: String) {
        val payload = JsonObject().apply {
            addProperty("protocolVersion", UI_RUNTIME_PROTOCOL_VERSION)
            addProperty("messageType", "runtime.error")
            addProperty("error", error)
        }
        send(gson.toJson(payload))
    }

    private fun send(text: String) {
        if (socket?.send(text) != true) Log.w(TAG, "Live UI message not sent")
    }
}
