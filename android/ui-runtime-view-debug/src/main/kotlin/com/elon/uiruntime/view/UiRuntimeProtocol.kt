package com.elon.uiruntime.view

import com.google.gson.JsonElement
import com.google.gson.annotations.SerializedName

internal const val UI_RUNTIME_PROTOCOL_VERSION = 1
internal const val UI_RUNTIME_VERSION = "1.0.0"

internal data class LiveRect(
    val left: Int,
    val top: Int,
    val right: Int,
    val bottom: Int,
    val width: Int,
    val height: Int,
)

internal data class LiveGeometry(
    val boundsInDisplayPx: LiveRect,
    val density: Float,
    val fontScale: Float,
    val rotation: Int,
    val visible: Boolean,
)

internal data class LivePropertyValue(
    @SerializedName("type") val valueType: String,
    val value: JsonElement,
)

internal data class LivePropertySnapshot(
    val effective: LivePropertyValue?,
    val measured: LivePropertyValue?,
    val changeLevel: String = "LIVE",
    val commitMode: String = "CODEX",
    val binding: JsonElement? = null,
    val constraints: JsonElement? = null,
)

internal data class LiveUiNode(
    val runtimeNodeId: String,
    val definitionId: String,
    val instanceKey: String?,
    val parentRuntimeNodeId: String?,
    val screenId: String,
    val kind: String,
    val text: String?,
    val resourceId: String?,
    val className: String,
    val source: JsonElement? = null,
    val geometry: LiveGeometry,
    val properties: Map<String, LivePropertySnapshot>,
    val capabilities: Map<String, Boolean>,
)

internal data class LivePatchTarget(
    val scope: String,
    val runtimeNodeId: String?,
    val definitionId: String?,
    val instanceKey: String?,
)

internal data class LivePatchOperation(
    val property: String,
    val value: LivePropertyValue,
)

internal data class LiveStylePatch(
    val protocolVersion: Int,
    val messageType: String,
    val sessionId: String,
    val requestId: String,
    val gestureId: String?,
    val sequence: Long,
    val baseTreeRevision: Long?,
    val target: LivePatchTarget,
    val atomic: Boolean,
    val ephemeral: Boolean,
    val operations: List<LivePatchOperation>,
)

internal data class TreeSnapshotMessage(
    val protocolVersion: Int = UI_RUNTIME_PROTOCOL_VERSION,
    val messageType: String = "tree.snapshot",
    val treeRevision: Long,
    val nodes: List<LiveUiNode>,
)

internal data class RuntimeHelloMessage(
    val protocolVersion: Int = UI_RUNTIME_PROTOCOL_VERSION,
    val messageType: String = "runtime.hello",
    val sessionId: String,
    val packageName: String,
    val appBuildId: String,
    val runtimeVersion: String = UI_RUNTIME_VERSION,
    val androidSdk: Int,
    val renderer: String = "android-view",
)

internal data class PatchAckMessage(
    val protocolVersion: Int = UI_RUNTIME_PROTOCOL_VERSION,
    val messageType: String,
    val sessionId: String,
    val requestId: String,
    val gestureId: String?,
    val sequence: Long,
    val status: String,
    val newTreeRevision: Long,
    val beforeValues: Map<String, LivePropertyValue> = emptyMap(),
    val effectiveValues: Map<String, LivePropertyValue> = emptyMap(),
    val measuredGeometry: Map<String, Double> = emptyMap(),
    val error: String? = null,
)

internal data class UiRuntimeSessionConfig(
    val sessionId: String,
    val token: String,
    val devicePort: Int,
)
