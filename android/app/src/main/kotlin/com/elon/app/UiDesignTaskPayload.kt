package com.elon.app

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import java.util.Locale

internal enum class UiDesignRequestMode(val wireValue: String) {
    AUTO("AUTO"),
    MODIFY_EXISTING("MODIFY_EXISTING"),
    EXTEND_EXISTING("EXTEND_EXISTING"),
    CREATE_NEW("CREATE_NEW")
}

internal enum class UiDesignImageIntent(val wireValue: String) {
    AUTO("AUTO"),
    TARGET_DESIGN("TARGET_DESIGN"),
    ANNOTATED_CHANGE_REQUEST("ANNOTATED_CHANGE_REQUEST"),
    REFERENCE_STYLE("REFERENCE_STYLE"),
    CURRENT_SCREENSHOT("CURRENT_SCREENSHOT")
}

internal data class UiDesignRequestSelection(
    val enabled: Boolean = false,
    val mode: UiDesignRequestMode = UiDesignRequestMode.AUTO,
    val imageIntent: UiDesignImageIntent = UiDesignImageIntent.AUTO,
    val screenId: String? = null,
    val behaviorNotes: List<String> = emptyList()
)

internal fun buildUiDesignTaskPayload(
    traceId: String,
    outgoingText: String,
    attachmentRefs: JsonArray,
    selection: UiDesignRequestSelection = UiDesignRequestSelection()
): JsonObject? {
    val imageRefs = attachmentRefs
        .mapNotNull { element -> element.takeIf { it.isJsonObject }?.asJsonObject }
        .filter { item ->
            item.stringOrNull("kind") == "image" ||
                item.stringOrNull("mime_type").orEmpty().startsWith("image/")
        }
    if (imageRefs.isEmpty()) return null

    val hasAnnotations = imageRefs.any {
        ((it.get("annotations") as? JsonArray)?.size() ?: 0) > 0
    }
    if (!selection.enabled && !hasAnnotations && !looksLikeUiDesignRequest(outgoingText)) return null

    val mode = selection.mode.takeUnless { it == UiDesignRequestMode.AUTO }
        ?: inferUiDesignMode(outgoingText)
    val intent = selection.imageIntent.takeUnless { it == UiDesignImageIntent.AUTO }
        ?: inferImageIntent(outgoingText, hasAnnotations)
    val primaryAttachmentId = imageRefs.firstNotNullOfOrNull { it.stringOrNull("attachment_id") }

    return JsonObject().apply {
        addProperty("taskId", "design_$traceId")
        addProperty("mode", mode.wireValue)
        addProperty("attachmentIntent", intent.wireValue)
        selection.screenId?.trim()?.takeIf { it.isNotEmpty() }?.let { addProperty("screenId", it) }
        when (intent) {
            UiDesignImageIntent.TARGET_DESIGN -> primaryAttachmentId?.let {
                addProperty("targetDesignAttachmentId", it)
            }
            UiDesignImageIntent.ANNOTATED_CHANGE_REQUEST -> primaryAttachmentId?.let {
                addProperty("annotatedPreviewAttachmentId", it)
            }
            UiDesignImageIntent.REFERENCE_STYLE -> primaryAttachmentId?.let {
                add("referenceAttachmentIds", JsonArray().apply { add(it) })
            }
            UiDesignImageIntent.CURRENT_SCREENSHOT -> primaryAttachmentId?.let {
                addProperty("originalAttachmentId", it)
            }
            UiDesignImageIntent.AUTO -> Unit
        }
        if (selection.behaviorNotes.isNotEmpty()) {
            add("behaviorNotes", JsonArray().apply {
                selection.behaviorNotes
                    .map(String::trim)
                    .filter(String::isNotEmpty)
                    .take(32)
                    .forEach(::add)
            })
        }
        add("renderTarget", JsonObject().apply { addProperty("kind", "AUTO") })
        add("executionPolicy", JsonObject().apply {
            addProperty("allowLivePatch", true)
            addProperty("allowDeterministicCommit", true)
            addProperty("allowSourceEdit", true)
            addProperty("requireBuildVerification", true)
        })
    }
}

private fun inferUiDesignMode(text: String): UiDesignRequestMode {
    val normalized = text.lowercase(Locale.ROOT)
    return when {
        CREATE_MARKERS.any(normalized::contains) -> UiDesignRequestMode.CREATE_NEW
        EXTEND_MARKERS.any(normalized::contains) -> UiDesignRequestMode.EXTEND_EXISTING
        MODIFY_MARKERS.any(normalized::contains) -> UiDesignRequestMode.MODIFY_EXISTING
        else -> UiDesignRequestMode.AUTO
    }
}

private fun inferImageIntent(text: String, hasAnnotations: Boolean): UiDesignImageIntent {
    if (hasAnnotations) return UiDesignImageIntent.ANNOTATED_CHANGE_REQUEST
    val normalized = text.lowercase(Locale.ROOT)
    return when {
        CURRENT_MARKERS.any(normalized::contains) -> UiDesignImageIntent.CURRENT_SCREENSHOT
        REFERENCE_MARKERS.any(normalized::contains) -> UiDesignImageIntent.REFERENCE_STYLE
        TARGET_MARKERS.any(normalized::contains) -> UiDesignImageIntent.TARGET_DESIGN
        else -> UiDesignImageIntent.AUTO
    }
}

private fun looksLikeUiDesignRequest(text: String): Boolean {
    val normalized = text.lowercase(Locale.ROOT)
    return UI_MARKERS.any(normalized::contains) ||
        CREATE_MARKERS.any(normalized::contains) ||
        EXTEND_MARKERS.any(normalized::contains) ||
        MODIFY_MARKERS.any(normalized::contains)
}

private val UI_MARKERS = listOf(
    "设计稿", "设计图", "草稿图", "ui", "界面", "页面样式", "组件样式", "像素", "1:1", "拟合"
)
private val CREATE_MARKERS = listOf(
    "全新页面", "新建页面", "创建页面", "从零开始", "还没有源码", "没有相关源码", "create new screen"
)
private val EXTEND_MARKERS = listOf(
    "扩展页面", "增加区域", "新增区域", "添加组件", "新增组件", "extend existing"
)
private val MODIFY_MARKERS = listOf(
    "修改现有", "调整现有", "还原设计稿", "按图修改", "修改样式", "modify existing"
)
private val TARGET_MARKERS = listOf("设计稿", "设计图", "目标图", "1:1", "像素级", "按图还原")
private val REFERENCE_MARKERS = listOf("风格参考", "参考风格", "参考这张", "灵感图")
private val CURRENT_MARKERS = listOf("当前截图", "现状截图", "真机截图", "现在的页面")
