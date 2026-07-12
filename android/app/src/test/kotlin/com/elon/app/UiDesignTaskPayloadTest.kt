package com.elon.app

import com.google.gson.JsonArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class UiDesignTaskPayloadTest {
    @Test
    fun pureTextStyleRequestEntersModifyWorkflow() {
        val payload = buildUiDesignTaskPayload(
            traceId = "style-1",
            outgoingText = "这个按钮太大了，圆角改小一点",
            attachmentRefs = JsonArray()
        )

        assertNotNull(payload)
        assertEquals("MODIFY_EXISTING", payload?.get("mode")?.asString)
        assertEquals("AUTO", payload?.get("attachmentIntent")?.asString)
        assertEquals(true, payload?.getAsJsonObject("executionPolicy")?.get("allowLivePatch")?.asBoolean)
    }

    @Test
    fun newScreenWithoutSourceEntersCreateWorkflow() {
        val payload = buildUiDesignTaskPayload(
            traceId = "create-1",
            outgoingText = "这是一个全新页面，还没有相关源码，请从零开始创建",
            attachmentRefs = JsonArray()
        )

        assertNotNull(payload)
        assertEquals("CREATE_NEW", payload?.get("mode")?.asString)
    }

    @Test
    fun addingVisualComponentEntersExtensionWorkflow() {
        val payload = buildUiDesignTaskPayload(
            traceId = "extend-1",
            outgoingText = "在当前页面新增一张卡片",
            attachmentRefs = JsonArray()
        )

        assertNotNull(payload)
        assertEquals("EXTEND_EXISTING", payload?.get("mode")?.asString)
    }

    @Test
    fun buttonBehaviorBugDoesNotMasqueradeAsStyleRequest() {
        val payload = buildUiDesignTaskPayload(
            traceId = "behavior-1",
            outgoingText = "按钮点击后没有反应，请修复接口逻辑",
            attachmentRefs = JsonArray()
        )

        assertNull(payload)
    }

    @Test
    fun englishBuildRequestDoesNotMatchUiSubstring() {
        val payload = buildUiDesignTaskPayload(
            traceId = "build-1",
            outgoingText = "build apk and publish release",
            attachmentRefs = JsonArray()
        )

        assertNull(payload)
    }

    @Test
    fun explicitUiDesignSelectionWorksWithoutImage() {
        val payload = buildUiDesignTaskPayload(
            traceId = "explicit-1",
            outgoingText = "按现有设计系统处理",
            attachmentRefs = JsonArray(),
            selection = UiDesignRequestSelection(enabled = true)
        )

        assertNotNull(payload)
        assertEquals("AUTO", payload?.get("attachmentIntent")?.asString)
    }
}
