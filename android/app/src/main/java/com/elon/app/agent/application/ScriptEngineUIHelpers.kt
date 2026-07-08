package com.elon.app.agent.application

import android.accessibilityservice.AccessibilityService
import android.content.Context
import android.util.Log
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import com.elon.app.agent.AgentService
import com.elon.app.agent.application.executor.*
import com.elon.app.agent.domain.execution.ExecutionConfig
import com.elon.app.agent.domain.execution.ExecutionMode
import com.elon.app.agent.domain.execution.ExecutionState
import com.elon.app.agent.domain.execution.ExecutionStateManager
import com.elon.app.agent.domain.screen.ScreenCaptureMode
import com.elon.app.agent.domain.script.*
import com.elon.app.agent.infrastructure.ai.AIClientFactory
import com.elon.app.agent.infrastructure.debug.DebugInterface
import com.elon.app.agent.infrastructure.popup.PopupDismisser
import com.google.gson.Gson
import com.google.gson.GsonBuilder
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.*
import java.io.File
import java.util.UUID

// 本地常量(与companion object保持一致)
private const val TAG = "ScriptEngine"
private const val SCRIPTS_DIR = "scripts"
private const val MAX_IMPROVE_ATTEMPTS = 3

// ===== [ScriptEngineUIHelpers.kt] =====
// ===== [ScriptEngineUIHelpers.kt] =====
// ========== 辅助函数 ==========

internal fun ScriptEngine.performTap(x: Int, y: Int): StepResult {
    val path = android.graphics.Path().apply {
        moveTo(x.toFloat(), y.toFloat())
    }
    val gesture = android.accessibilityservice.GestureDescription.Builder()
        .addStroke(android.accessibilityservice.GestureDescription.StrokeDescription(path, 0, 150))
        .build()
    
    val success = service.dispatchGesture(gesture, null, null)
    if (!success) {
        debugInterface.recordError("TAP_GESTURE_FAILED", "点击手势执行失败", context = mapOf(
            "x" to x.toString(),
            "y" to y.toString()
        ), suggestion = "检查无障碍服务是否正常运行，或坐标是否在屏幕范围内")
    }
    return StepResult(success, if (!success) "点击手势执行失败 ($x, $y)" else null)
}

internal fun ScriptEngine.performSwipe(direction: String): StepResult {
    val displayMetrics = service.resources.displayMetrics
    val width = displayMetrics.widthPixels
    val height = displayMetrics.heightPixels
    
    val (startX, startY, endX, endY) = when (direction.lowercase()) {
        "up" -> listOf(width / 2, height * 3 / 4, width / 2, height / 4)
        "down" -> listOf(width / 2, height / 4, width / 2, height * 3 / 4)
        "left" -> listOf(width * 3 / 4, height / 2, width / 4, height / 2)
        "right" -> listOf(width / 4, height / 2, width * 3 / 4, height / 2)
        else -> {
            debugInterface.recordError("INVALID_SWIPE_DIRECTION", "无效的滑动方向: $direction", context = mapOf(
                "direction" to direction,
                "valid_directions" to "up, down, left, right"
            ))
            return StepResult(false, "无效的滑动方向: $direction")
        }
    }
    
    val path = android.graphics.Path().apply {
        moveTo(startX.toFloat(), startY.toFloat())
        lineTo(endX.toFloat(), endY.toFloat())
    }
    val gesture = android.accessibilityservice.GestureDescription.Builder()
        .addStroke(android.accessibilityservice.GestureDescription.StrokeDescription(path, 0, 300))
        .build()
    
    val success = service.dispatchGesture(gesture, null, null)
    if (!success) {
        debugInterface.recordError("SWIPE_GESTURE_FAILED", "滑动手势执行失败", context = mapOf(
            "direction" to direction,
            "start" to "($startX, $startY)",
            "end" to "($endX, $endY)"
        ))
    }
    return StepResult(success)
}

internal fun ScriptEngine.findAndTapByText(text: String): StepResult {
    val root = getRootNode() ?: return StepResult(false, "No window")
    val node = findMatchingNode(root, text, null, null)
    
    if (node != null) {
        val rect = android.graphics.Rect()
        node.getBoundsInScreen(rect)
        return performTap(rect.centerX(), rect.centerY())
    }
    
    return StepResult(false, "Text not found: $text")
}

internal fun ScriptEngine.findMatchingNode(
    node: android.view.accessibility.AccessibilityNodeInfo,
    exactText: String?,
    containsText: String?,
    pattern: String?
): android.view.accessibility.AccessibilityNodeInfo? {
    val nodeText = node.text?.toString() ?: ""
    val nodeDesc = node.contentDescription?.toString() ?: ""
    val combined = "$nodeText $nodeDesc"
    
    val matches = when {
        exactText != null -> nodeText == exactText || nodeDesc == exactText
        containsText != null -> combined.contains(containsText, ignoreCase = true)
        pattern != null -> Regex(pattern).containsMatchIn(combined)
        else -> false
    }
    
    if (matches && node.isClickable) return node
    
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        val result = findMatchingNode(child, exactText, containsText, pattern)
        if (result != null) return result
    }
    
    return null
}

/**
 * 增强版节点查找 - 即使元素不可点击也返回（用于获取坐标点击）
 * 优先返回可点击元素，否则返回匹配元素本身
 */
internal fun ScriptEngine.findMatchingNodeEnhanced(
    node: android.view.accessibility.AccessibilityNodeInfo,
    exactText: String?,
    containsText: String?,
    pattern: String?,
    clickableParent: android.view.accessibility.AccessibilityNodeInfo? = null
): android.view.accessibility.AccessibilityNodeInfo? {
    val nodeText = node.text?.toString() ?: ""
    val nodeDesc = node.contentDescription?.toString() ?: ""
    val combined = "$nodeText $nodeDesc"
    
    // 更新可点击父级
    val currentClickable = if (node.isClickable) node else clickableParent
    
    val matches = when {
        exactText != null -> nodeText == exactText || nodeDesc == exactText
        containsText != null -> smartContainsMatch(combined, containsText)
        pattern != null -> {
            try {
                // 处理可能过度转义的正则表达式
                val cleanPattern = pattern
                    .replace("\\\\\\\\", "\\")  // 4个反斜杠 -> 1个
                    .replace("\\\\", "\\")       // 2个反斜杠 -> 1个
                Regex(cleanPattern).containsMatchIn(combined)
            } catch (e: Exception) {
                log("⚠️ 正则匹配错误: ${e.message}, pattern=$pattern")
                // 尝试简单的数字匹配作为后备
                val hasLargeNumber = Regex("\\d+(\\.\\d)?[万w]|[1-9]\\d{4,}").containsMatchIn(combined)
                if (hasLargeNumber) log("🎯 后备正则匹配成功")
                hasLargeNumber
            }
        }
        else -> false
    }
    
    if (matches) {
        // 如果找到匹配，优先返回可点击父级，否则返回当前节点
        log("🎯 匹配: '$combined'")
        return currentClickable ?: node
    }
    
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        val result = findMatchingNodeEnhanced(child, exactText, containsText, pattern, currentClickable)
        if (result != null) return result
    }
    
    return null
}

/**
 * 智能包含匹配 - 处理各种等价表达
 * 例如：搜索"万"时也匹配"w"、"1.2w"、"10000+"等
 */
internal fun ScriptEngine.smartContainsMatch(text: String, searchTerm: String): Boolean {
    // 首先尝试直接匹配
    if (text.contains(searchTerm, ignoreCase = true)) {
        return true
    }
    
    // 特殊语义匹配
    when (searchTerm.lowercase()) {
        // 匹配大数字的各种表达: 万、w、10000+
        "万", "w" -> {
            // 匹配: 1万、1.2万、1w、1.2w、10000+
            val largeNumberPattern = Regex("\\d+(\\.\\d+)?[万wW]|[1-9]\\d{4,}")
            return largeNumberPattern.containsMatchIn(text)
        }
        // 匹配赞/点赞
        "赞", "点赞", "喜欢" -> {
            return text.contains("赞", ignoreCase = true) || 
                   text.contains("喜欢", ignoreCase = true) ||
                   text.contains("like", ignoreCase = true)
        }
        // 匹配评论
        "评论", "留言" -> {
            return text.contains("评论", ignoreCase = true) ||
                   text.contains("留言", ignoreCase = true) ||
                   text.contains("comment", ignoreCase = true)
        }
    }
    
    return false
}

internal fun ScriptEngine.extractTexts(
    node: android.view.accessibility.AccessibilityNodeInfo,
    results: MutableList<String>,
    maxCount: Int
) {
    if (results.size >= maxCount) return
    
    val text = node.text?.toString()?.trim()
    if (!text.isNullOrEmpty() && text.length > 5) {
        results.add(text)
    }
    
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        extractTexts(child, results, maxCount)
    }
}

internal fun ScriptEngine.collectElements(node: android.view.accessibility.AccessibilityNodeInfo): String {
    val elements = mutableListOf<String>()
    collectElementsRecursive(node, elements, 20)
    return elements.joinToString("\n")
}

internal fun ScriptEngine.collectElementsRecursive(
    node: android.view.accessibility.AccessibilityNodeInfo,
    elements: MutableList<String>,
    maxCount: Int
) {
    if (elements.size >= maxCount) return
    
    val text = node.text?.toString() ?: node.contentDescription?.toString()
    if (!text.isNullOrEmpty() && node.isClickable) {
        val rect = android.graphics.Rect()
        node.getBoundsInScreen(rect)
        elements.add("\"$text\" @ (${rect.centerX()}, ${rect.centerY()})")
    }
    
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        collectElementsRecursive(child, elements, maxCount)
    }
}
