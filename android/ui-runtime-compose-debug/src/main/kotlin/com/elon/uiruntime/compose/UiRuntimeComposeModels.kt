package com.elon.uiruntime.compose

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit

sealed interface UiSourceBinding {
    fun protocolValue(): Map<String, Any?>

    data class Token(val path: String) : UiSourceBinding {
        override fun protocolValue() = mapOf("kind" to "TOKEN", "path" to path)
    }

    data class StyleJson(val relativeFile: String, val jsonPointer: String) : UiSourceBinding {
        override fun protocolValue() = mapOf(
            "kind" to "STYLE_JSON",
            "relativeFile" to relativeFile,
            "jsonPointer" to jsonPointer,
        )
    }

    data class KotlinSymbol(val symbol: String, val anchorHash: String? = null) : UiSourceBinding {
        override fun protocolValue() = mapOf(
            "kind" to "KOTLIN_SYMBOL",
            "symbol" to symbol,
            "anchorHash" to anchorHash,
        )
    }

    data object Computed : UiSourceBinding {
        override fun protocolValue() = mapOf("kind" to "COMPUTED")
    }

    data object SessionOnly : UiSourceBinding {
        override fun protocolValue() = mapOf("kind" to "SESSION_ONLY")
    }
}

@Immutable
data class UiEditableProperty(
    val key: String,
    val changeLevel: String = "LIVE",
    val commitMode: String = "CODEX",
    val binding: UiSourceBinding = UiSourceBinding.Computed,
    val minimum: Double? = null,
    val maximum: Double? = null,
    val step: Double? = null,
)

@Immutable
data class UiSourceSymbol(
    val module: String? = null,
    val fullyQualifiedName: String? = null,
    val relativeFile: String? = null,
    val symbolHash: String? = null,
)

@Immutable
data class UiNodeSpec(
    val id: String,
    val instanceKey: String? = null,
    val parentRuntimeNodeId: String? = null,
    val screenId: String,
    val kind: String,
    val source: UiSourceSymbol? = null,
    val editableProperties: List<UiEditableProperty>,
)

@Immutable
data class UiStyle(
    val width: Dp? = null,
    val height: Dp? = null,
    val paddingStart: Dp? = null,
    val paddingTop: Dp? = null,
    val paddingEnd: Dp? = null,
    val paddingBottom: Dp? = null,
    val backgroundColor: Color? = null,
    val contentColor: Color? = null,
    val borderColor: Color? = null,
    val borderWidth: Dp? = null,
    val cornerRadius: Dp? = null,
    val text: String? = null,
    val textSize: TextUnit? = null,
    val opacity: Float? = null,
    val horizontalAlignment: String? = null,
    val verticalAlignment: String? = null,
)
