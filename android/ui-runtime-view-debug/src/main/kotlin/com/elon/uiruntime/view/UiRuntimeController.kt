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
    private val activeViewPatches = linkedMapOf<LivePatchTarget, LinkedHashMap<String, LivePatchOperation>>()

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

    fun stop(sessionId: String? = null) {
        if (sessionId != null && config?.sessionId != sessionId) {
            Log.i(TAG, "Ignore stale stop request for $sessionId")
            return
        }
        connected = false
        socket?.close()
        socket = null
        config = null
        activeViewPatches.clear()
        mainHandler.removeCallbacksAndMessages(null)
    }

    fun upsertExternalNode(node: UiRuntimeExternalNode) {
        mainHandler.post { registry?.upsertExternalNode(node) }
    }

    fun removeExternalNode(runtimeNodeId: String) {
        mainHandler.post { registry?.removeExternalNode(runtimeNodeId) }
    }

    fun clearExternalNodes() {
        mainHandler.post { registry?.clearExternalNodes() }
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
                val targets = currentRegistry.resolve(patch.target)
                require(targets.views.isNotEmpty() || targets.externalNodes.isNotEmpty()) {
                    "找不到目标节点，页面可能已经变化"
                }
                val viewResult = targets.views.takeIf { it.isNotEmpty() }?.let { views ->
                    UiRuntimeViewAdapter.apply(views, patch.operations)
                }
                val externalResults = targets.externalNodes.map { node ->
                    node.applyOperations(
                        patch.operations.map { operation ->
                            UiRuntimeExternalPatchOperation(
                                property = operation.property,
                                value = UiRuntimeValue(
                                    type = operation.value.valueType,
                                    value = operation.value.value.toPrimitiveValue(),
                                ),
                            )
                        },
                    )
                }
                mergeResults(viewResult, externalResults)
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
            if (result.isSuccess) {
                val operations = activeViewPatches.getOrPut(patch.target, ::linkedMapOf)
                patch.operations.forEach { operation -> operations[operation.property] = operation }
                schedulePatchReapply()
                scheduleTreeSnapshot()
            }
        }
    }

    private fun schedulePatchReapply() {
        mainHandler.removeCallbacks(reapplyPatchesRunnable)
        if (connected && activeViewPatches.isNotEmpty()) {
            mainHandler.postDelayed(reapplyPatchesRunnable, 120)
        }
    }

    private val reapplyPatchesRunnable = object : Runnable {
        override fun run() {
            val currentRegistry = registry ?: return
            var changed = false
            activeViewPatches.forEach { (target, operations) ->
                val views = currentRegistry.resolve(target).views
                if (views.isNotEmpty()) {
                    changed = UiRuntimeViewAdapter.reapply(views, operations.values.toList()) || changed
                }
            }
            if (changed) scheduleTreeSnapshot()
            if (connected && activeViewPatches.isNotEmpty()) {
                mainHandler.postDelayed(this, 120)
            }
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

    private val sendTreeRunnable = object : Runnable {
        override fun run() {
            val currentRegistry = registry ?: return
            send(gson.toJson(currentRegistry.snapshot()))
            if (connected) mainHandler.postDelayed(this, 1_200)
        }
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

    private fun mergeResults(
        viewResult: UiRuntimeViewAdapter.ApplyResult?,
        externalResults: List<UiRuntimeExternalApplyResult>,
    ): UiRuntimeViewAdapter.ApplyResult {
        val before = linkedMapOf<String, LivePropertyValue>()
        val effective = linkedMapOf<String, LivePropertyValue>()
        val measured = linkedMapOf<String, Double>()
        viewResult?.let {
            before.putAll(it.beforeValues)
            effective.putAll(it.effectiveValues)
            measured.putAll(it.measuredGeometry)
        }
        externalResults.forEach { result ->
            result.beforeValues.forEach { (key, value) -> before.putIfAbsent(key, value.toProtocolValue()) }
            result.effectiveValues.forEach { (key, value) -> effective[key] = value.toProtocolValue() }
            measured.putAll(result.measuredGeometry)
        }
        return UiRuntimeViewAdapter.ApplyResult(before, effective, measured)
    }

    private fun UiRuntimeValue.toProtocolValue(): LivePropertyValue = LivePropertyValue(
        valueType = type,
        value = gson.toJsonTree(value),
    )

    private fun com.google.gson.JsonElement.toPrimitiveValue(): Any? = when {
        isJsonNull -> null
        isJsonPrimitive && asJsonPrimitive.isBoolean -> asBoolean
        isJsonPrimitive && asJsonPrimitive.isNumber -> asDouble
        isJsonPrimitive -> asString
        else -> gson.fromJson(this, Any::class.java)
    }
}
