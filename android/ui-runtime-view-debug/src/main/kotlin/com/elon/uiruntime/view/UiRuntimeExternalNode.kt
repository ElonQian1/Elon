package com.elon.uiruntime.view

/**
 * 非 View 渲染器（例如 Jetpack Compose）接入统一 Live UI Runtime 的公开桥接模型。
 *
 * 该模块只通过 debugImplementation 进入应用；Release APK 不应包含此 API。
 */
data class UiRuntimeValue(
    val type: String,
    val value: Any?,
)

data class UiRuntimeExternalGeometry(
    val leftPx: Int,
    val topPx: Int,
    val rightPx: Int,
    val bottomPx: Int,
    val density: Float,
    val fontScale: Float,
    val rotation: Int,
    val visible: Boolean = true,
)

data class UiRuntimeExternalProperty(
    val effective: UiRuntimeValue?,
    val measured: UiRuntimeValue? = null,
    val changeLevel: String = "LIVE",
    val commitMode: String = "CODEX",
    val binding: Any? = null,
    val constraints: Any? = null,
)

data class UiRuntimeExternalSource(
    val module: String? = null,
    val fullyQualifiedName: String? = null,
    val relativeFile: String? = null,
    val symbolHash: String? = null,
)

data class UiRuntimeExternalPatchOperation(
    val property: String,
    val value: UiRuntimeValue,
)

data class UiRuntimeExternalApplyResult(
    val beforeValues: Map<String, UiRuntimeValue>,
    val effectiveValues: Map<String, UiRuntimeValue>,
    val measuredGeometry: Map<String, Double> = emptyMap(),
)

class UiRuntimeExternalNode(
    val runtimeNodeId: String,
    val definitionId: String,
    val instanceKey: String? = null,
    val parentRuntimeNodeId: String? = null,
    val screenId: String,
    val kind: String,
    val text: String? = null,
    val className: String,
    val source: UiRuntimeExternalSource? = null,
    val geometry: UiRuntimeExternalGeometry,
    val properties: Map<String, UiRuntimeExternalProperty>,
    val capabilities: Map<String, Boolean>,
    internal val applyOperations: (List<UiRuntimeExternalPatchOperation>) -> UiRuntimeExternalApplyResult,
)

object UiRuntimeBridge {
    fun upsert(node: UiRuntimeExternalNode) {
        require(node.runtimeNodeId.isNotBlank()) { "runtimeNodeId 不能为空" }
        require(node.definitionId.isNotBlank()) { "definitionId 不能为空" }
        UiRuntimeController.upsertExternalNode(node)
    }

    fun remove(runtimeNodeId: String) {
        if (runtimeNodeId.isNotBlank()) UiRuntimeController.removeExternalNode(runtimeNodeId)
    }

    /** Clear renderer-owned nodes before a Preview Host switches to another screen. */
    fun clear() {
        UiRuntimeController.clearExternalNodes()
    }
}
