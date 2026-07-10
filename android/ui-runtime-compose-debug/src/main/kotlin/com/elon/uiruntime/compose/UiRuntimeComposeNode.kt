package com.elon.uiruntime.compose

import android.graphics.Color.parseColor
import android.view.View
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.elon.uiruntime.view.UiRuntimeBridge
import com.elon.uiruntime.view.UiRuntimeExternalApplyResult
import com.elon.uiruntime.view.UiRuntimeExternalGeometry
import com.elon.uiruntime.view.UiRuntimeExternalNode
import com.elon.uiruntime.view.UiRuntimeExternalPatchOperation
import com.elon.uiruntime.view.UiRuntimeExternalProperty
import com.elon.uiruntime.view.UiRuntimeExternalSource
import com.elon.uiruntime.view.UiRuntimeValue
import java.util.Locale
import java.util.UUID
import kotlin.math.roundToInt

@Stable
class UiNodeHandle internal constructor(
    spec: UiNodeSpec,
    declaredStyle: UiStyle,
    hostView: View,
    density: Density,
) {
    private val runtimeNodeId = "rn_compose_${UUID.randomUUID().toString().replace("-", "")}"
    private var spec = spec
    private var declaredStyle = declaredStyle
    private var density = density
    private var hostOffsetX = 0
    private var hostOffsetY = 0
    private var rotation = hostView.display?.rotation ?: 0
    private var bounds = Rect.Zero
    private val overrides = linkedMapOf<String, UiRuntimeValue>()

    var style by mutableStateOf(declaredStyle)
        private set

    val definitionId: String get() = spec.id

    internal fun refresh(nextSpec: UiNodeSpec, nextDeclaredStyle: UiStyle, hostView: View, nextDensity: Density) {
        spec = nextSpec
        declaredStyle = nextDeclaredStyle
        density = nextDensity
        val location = IntArray(2)
        hostView.getLocationOnScreen(location)
        hostOffsetX = location[0]
        hostOffsetY = location[1]
        rotation = hostView.display?.rotation ?: 0
        style = overrides.entries.fold(declaredStyle) { current, (property, value) ->
            current.withValue(property, value)
        }
        publish()
    }

    internal fun updateBounds(nextBounds: Rect) {
        bounds = nextBounds
        publish()
    }

    internal fun publish() {
        UiRuntimeBridge.upsert(externalNode())
    }

    internal fun dispose() {
        UiRuntimeBridge.remove(runtimeNodeId)
    }

    private fun apply(operations: List<UiRuntimeExternalPatchOperation>): UiRuntimeExternalApplyResult {
        val editable = spec.editableProperties.associateBy(UiEditableProperty::key)
        val before = linkedMapOf<String, UiRuntimeValue>()
        operations.forEach { operation ->
            require(editable.containsKey(operation.property)) {
                "${spec.id} 不允许 LIVE 修改 ${operation.property}"
            }
            valueFor(style, operation.property)?.let { before[operation.property] = it }
            overrides[operation.property] = operation.value
            style = style.withValue(operation.property, operation.value)
        }
        val effective = operations.mapNotNull { operation ->
            valueFor(style, operation.property)?.let { operation.property to it }
        }.toMap()
        publish()
        return UiRuntimeExternalApplyResult(
            beforeValues = before,
            effectiveValues = effective,
            measuredGeometry = mapOf(
                "widthDp" to (bounds.width / density.density).toDouble(),
                "heightDp" to (bounds.height / density.density).toDouble(),
            ),
        )
    }

    private fun externalNode(): UiRuntimeExternalNode {
        val geometry = UiRuntimeExternalGeometry(
            leftPx = hostOffsetX + bounds.left.roundToInt(),
            topPx = hostOffsetY + bounds.top.roundToInt(),
            rightPx = hostOffsetX + bounds.right.roundToInt(),
            bottomPx = hostOffsetY + bounds.bottom.roundToInt(),
            density = density.density,
            fontScale = density.fontScale,
            rotation = rotation,
            visible = bounds.width > 0f && bounds.height > 0f,
        )
        val properties = spec.editableProperties.associate { editable ->
            val effective = valueFor(style, editable.key)
            val measured = when (editable.key) {
                "width" -> UiRuntimeValue("dp", bounds.width / density.density)
                "height" -> UiRuntimeValue("dp", bounds.height / density.density)
                else -> null
            }
            editable.key to UiRuntimeExternalProperty(
                effective = effective,
                measured = measured,
                changeLevel = editable.changeLevel,
                commitMode = editable.commitMode,
                binding = editable.binding.protocolValue(),
                constraints = mapOf(
                    "minimum" to editable.minimum,
                    "maximum" to editable.maximum,
                    "step" to editable.step,
                ).filterValues { it != null },
            )
        }
        val propertyKeys = properties.keys
        return UiRuntimeExternalNode(
            runtimeNodeId = runtimeNodeId,
            definitionId = spec.id,
            instanceKey = spec.instanceKey,
            parentRuntimeNodeId = spec.parentRuntimeNodeId,
            screenId = spec.screenId,
            kind = spec.kind,
            text = style.text,
            className = "androidx.compose.${spec.kind}",
            source = spec.source?.let {
                UiRuntimeExternalSource(
                    module = it.module,
                    fullyQualifiedName = it.fullyQualifiedName,
                    relativeFile = it.relativeFile,
                    symbolHash = it.symbolHash,
                )
            },
            geometry = geometry,
            properties = properties,
            capabilities = mapOf(
                "resizeWidth" to propertyKeys.contains("width"),
                "resizeHeight" to propertyKeys.contains("height"),
                "padding" to propertyKeys.any { it.startsWith("padding.") },
                "opacity" to propertyKeys.contains("opacity"),
                "text" to propertyKeys.contains("text"),
                "textSize" to propertyKeys.contains("textSize"),
                "backgroundColor" to propertyKeys.contains("backgroundColor"),
                "contentColor" to propertyKeys.contains("contentColor"),
                "cornerRadius" to propertyKeys.contains("cornerRadius.all"),
                "freeTranslate" to false,
            ),
            applyOperations = ::apply,
        )
    }
}

@Composable
fun rememberUiNode(spec: UiNodeSpec, declaredStyle: UiStyle): UiNodeHandle {
    require(spec.id.isNotBlank()) { "Compose uiNode id 不能为空" }
    require(spec.screenId.isNotBlank()) { "Compose uiNode screenId 不能为空" }
    val hostView = LocalView.current
    val density = LocalDensity.current
    val handle = remember(spec.id, spec.instanceKey) {
        UiNodeHandle(spec, declaredStyle, hostView, density)
    }
    SideEffect { handle.refresh(spec, declaredStyle, hostView, density) }
    DisposableEffect(handle) {
        handle.publish()
        onDispose(handle::dispose)
    }
    return handle
}

fun Modifier.uiNode(handle: UiNodeHandle): Modifier = this
    .testTag(handle.definitionId)
    .onGloballyPositioned { coordinates -> handle.updateBounds(coordinates.boundsInRoot()) }

private fun UiStyle.withValue(property: String, value: UiRuntimeValue): UiStyle = when (property) {
    "width" -> copy(width = value.number().dp)
    "height" -> copy(height = value.number().dp)
    "padding.start" -> copy(paddingStart = value.number().dp)
    "padding.top" -> copy(paddingTop = value.number().dp)
    "padding.end" -> copy(paddingEnd = value.number().dp)
    "padding.bottom" -> copy(paddingBottom = value.number().dp)
    "backgroundColor" -> copy(backgroundColor = value.color())
    "contentColor" -> copy(contentColor = value.color())
    "borderColor" -> copy(borderColor = value.color())
    "borderWidth" -> copy(borderWidth = value.number().dp)
    "cornerRadius.all" -> copy(cornerRadius = value.number().dp)
    "text" -> copy(text = value.value?.toString().orEmpty())
    "textSize" -> copy(textSize = value.number().sp)
    "opacity" -> copy(opacity = value.number().toFloat().coerceIn(0f, 1f))
    "horizontalAlignment" -> copy(horizontalAlignment = value.value?.toString())
    "verticalAlignment" -> copy(verticalAlignment = value.value?.toString())
    else -> error("Compose Runtime 不支持属性 $property")
}

private fun valueFor(style: UiStyle, property: String): UiRuntimeValue? = when (property) {
    "width" -> style.width?.let { UiRuntimeValue("dp", it.value) }
    "height" -> style.height?.let { UiRuntimeValue("dp", it.value) }
    "padding.start" -> style.paddingStart?.let { UiRuntimeValue("dp", it.value) }
    "padding.top" -> style.paddingTop?.let { UiRuntimeValue("dp", it.value) }
    "padding.end" -> style.paddingEnd?.let { UiRuntimeValue("dp", it.value) }
    "padding.bottom" -> style.paddingBottom?.let { UiRuntimeValue("dp", it.value) }
    "backgroundColor" -> style.backgroundColor?.let { UiRuntimeValue("argb", it.argb()) }
    "contentColor" -> style.contentColor?.let { UiRuntimeValue("argb", it.argb()) }
    "borderColor" -> style.borderColor?.let { UiRuntimeValue("argb", it.argb()) }
    "borderWidth" -> style.borderWidth?.let { UiRuntimeValue("dp", it.value) }
    "cornerRadius.all" -> style.cornerRadius?.let { UiRuntimeValue("dp", it.value) }
    "text" -> style.text?.let { UiRuntimeValue("text", it) }
    "textSize" -> style.textSize?.let { UiRuntimeValue("sp", it.value) }
    "opacity" -> style.opacity?.let { UiRuntimeValue("float", it) }
    "horizontalAlignment" -> style.horizontalAlignment?.let { UiRuntimeValue("enum", it) }
    "verticalAlignment" -> style.verticalAlignment?.let { UiRuntimeValue("enum", it) }
    else -> null
}

private fun UiRuntimeValue.number(): Double = (value as? Number)?.toDouble()
    ?: value?.toString()?.toDoubleOrNull()
    ?: error("$type 不是数值")

private fun UiRuntimeValue.color(): Color = Color(parseColor(value?.toString()))

private fun Color.argb(): String = String.format(Locale.ROOT, "#%08X", toArgb())
