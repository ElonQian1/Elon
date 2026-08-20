package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatModelRangePolicyTest {
    @Test
    fun convertsOfficialFourPointAriaRangeIntoNativeEffortLevels() {
        val binding = WebChatModelRangePolicy.resolve(
            listOf(control(slider = slider(min = 0.0, max = 3.0, step = 1.0, value = 2.0))),
        )

        assertNotNull(binding)
        assertEquals(listOf("低", "中", "高", "极高"), binding?.options?.map { it.label })
        assertEquals(listOf(false, false, true, false), binding?.options?.map { it.selected })
        assertTrue(binding?.options?.all { it.parentLabel == "高级" } == true)
        val highest = binding?.options?.last() ?: error("missing highest level")
        assertEquals(3.0, binding.selections.getValue(highest.id).value, 0.0)
    }

    @Test
    fun ignoresRangesOutsideTheOfficialCompactModelScale() {
        assertNull(WebChatModelRangePolicy.resolve(
            listOf(control(slider = slider(min = 0.0, max = 10.0, step = 1.0, value = 2.0))),
        ))
    }

    private fun slider(min: Double, max: Double, step: Double, value: Double) =
        object : WebChatConsumerSlider {
            override val min = min
            override val max = max
            override val step = step
            override val value = value
            override val stepCount = kotlin.math.round((max - min) / step).toInt()
        }

    private fun control(slider: WebChatConsumerSlider) = object : WebChatConsumerControl {
        override val id = "official_effort"
        override val semantic = "slider"
        override val label = "推理强度"
        override val region = "overlay"
        override val role = "slider"
        override val enabled = true
        override val selected = false
        override val inputKind = "range"
        override val writable = false
        override val stateSettable = false
        override val choiceLabels = emptyList<String>()
        override val selectedChoiceIndex: Int? = null
        override val slider = slider
        override val expanded: Boolean? = null
        override val expandable = false
        override val contextId: String? = null
        override val inViewport = true
        override val webXRatio: Double? = null
        override val webYRatio: Double? = null
    }
}
