package com.elon.app

internal data class WebChatModelRangeSelection(
    val controlId: String,
    val value: Double,
)

internal data class WebChatModelRangeBinding(
    val options: List<WebChatConsumerOption>,
    val selections: Map<String, WebChatModelRangeSelection>,
)

internal object WebChatModelRangePolicy {
    fun resolve(controls: List<WebChatConsumerControl>): WebChatModelRangeBinding? {
        val control = controls.firstOrNull { candidate ->
            candidate.region == "overlay" && candidate.supportsSliderValue &&
                candidate.slider?.stepCount in MIN_STEP_COUNT..MAX_STEP_COUNT
        } ?: return null
        val slider = control.slider ?: return null
        val labels = labels(slider.stepCount + 1)
        val parentId = "model-range-parent:${control.id}"
        val options = labels.mapIndexed { index, label ->
            val id = "model-range:${control.id}:$index"
            val value = slider.min + index * slider.step
            WebChatConsumerOption(
                id = id,
                label = label,
                selected = kotlin.math.abs(slider.value - value) <= slider.step * TOLERANCE,
                semantic = "model",
                opensSubmenu = false,
                nativeSelector = "web-chat-model-level:$index",
                parentId = parentId,
                parentLabel = "高级",
            )
        }
        return WebChatModelRangeBinding(
            options = options,
            selections = options.mapIndexed { index, option ->
                option.id to WebChatModelRangeSelection(
                    controlId = control.id,
                    value = slider.min + index * slider.step,
                )
            }.toMap(),
        )
    }

    private fun labels(count: Int): List<String> = when (count) {
        2 -> listOf("低", "高")
        3 -> listOf("低", "中", "高")
        4 -> listOf("低", "中", "高", "极高")
        else -> List(count) { index -> "档位 ${index + 1}" }
    }

    private const val MIN_STEP_COUNT = 1
    private const val MAX_STEP_COUNT = 5
    private const val TOLERANCE = 1e-7
}
