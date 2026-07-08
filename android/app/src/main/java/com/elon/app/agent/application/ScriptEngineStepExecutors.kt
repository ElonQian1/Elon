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

// ===== [ScriptEngineStepExecutors.kt] =====
internal suspend fun ScriptEngine.executeStep(
    step: ScriptStep,
    context: Map<String, Any>
): StepResult {
    return when (step.type) {
        StepType.LAUNCH_APP -> executeLaunchApp(step)
        StepType.TAP -> executeTap(step)
        StepType.SWIPE -> executeSwipe(step)
        StepType.WAIT -> executeWait(step)
        StepType.FIND_AND_TAP -> executeFindAndTap(step)
        StepType.SCROLL_UNTIL_FIND -> executeScrollUntilFind(step)
        StepType.EXTRACT_DATA -> executeExtractData(step)
        StepType.INPUT_TEXT -> executeInputText(step)
        StepType.BACK -> executeBack(step)
        StepType.ASSERT -> executeAssert(step)
        StepType.AI_DECIDE -> executeAIDecide(step)
        StepType.SEARCH -> executeSearch(step) // SEARCH 等同于 FIND_AND_TAP
        else -> StepResult(false, "Unsupported step type: ${step.type}")
    }
}

// ========== 步骤执行实现 ==========

/**
 * 执行搜索步骤（等同于FIND_AND_TAP）
 */
internal suspend fun ScriptEngine.executeSearch(step: ScriptStep): StepResult {
    val text = step.params["text"] as? String
    val contains = step.params["contains"] as? String
    
    log("🔍 SEARCH: text=$text, contains=$contains")
    
    // 如果有text参数，先尝试点击搜索框然后输入
    if (text != null) {
        // 尝试找到并点击包含"搜索"的元素
        val root = getRootNode() ?: return StepResult(false, "No window")
        val searchBox = findMatchingNodeEnhanced(root, null, "搜索", null)
        if (searchBox != null) {
            val rect = android.graphics.Rect()
            searchBox.getBoundsInScreen(rect)
            performTap(rect.centerX(), rect.centerY())
            delay(500)
            val inputResult = executeInputText(
                ScriptStep(
                    index = step.index,
                    type = StepType.INPUT_TEXT,
                    description = "输入搜索关键词",
                    params = mapOf("text" to text),
                    onFail = step.onFail,
                    maxRetries = step.maxRetries
                )
            )
            if (!inputResult.success) {
                return inputResult
            }
            return StepResult(true, "Search text entered")
        }
        return StepResult(false, "Search box not found")
    }
    
    // 如果有contains，当作FIND_AND_TAP处理
    if (contains != null) {
        return executeFindAndTap(step)
    }
    
    return StepResult(false, "Missing search parameters")
}

internal suspend fun ScriptEngine.executeLaunchApp(step: ScriptStep): StepResult {
    val packageName = step.params["package"] as? String ?: return StepResult(false, "Missing package name")
    val goToHome = step.params["go_home"] as? Boolean ?: true // 默认回到首页
    
    try {
        log("🚀 尝试启动应用: $packageName")
        val intent = service.packageManager.getLaunchIntentForPackage(packageName)
        if (intent != null) {
            intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
            service.startActivity(intent)
            delay(2000) // 等待应用启动
            
            // 如果是小红书，自动点击"首页"按钮确保回到首页
            if (goToHome && packageName == "com.xingin.xhs") {
                log("🏠 尝试回到首页...")
                delay(500)
                ensureXhsHomePage()
            }
            
            return StepResult(true)
        }
        
        val error = "应用未安装或无法启动: $packageName"
        debugInterface.recordError("LAUNCH_APP_FAILED", error, context = mapOf(
            "package" to packageName,
            "reason" to "getLaunchIntentForPackage 返回 null"
        ), suggestion = "检查应用是否已安装，或在 AndroidManifest.xml 中添加 <queries> 声明")
        return StepResult(false, error)
    } catch (e: Exception) {
        val error = "启动应用失败: ${e.message}"
        debugInterface.recordError("LAUNCH_APP_EXCEPTION", error, e, mapOf(
            "package" to packageName
        ), suggestion = if (e.message?.contains("BLOCKED") == true) 
            "Android 11+ 包可见性限制，需要在 AndroidManifest.xml 添加 <queries> 声明" 
            else "检查应用是否存在权限问题")
        return StepResult(false, error)
    }
}

/**
 * 确保小红书在首页
 * 通过查找并点击底部导航栏的"首页"按钮
 */
internal suspend fun ScriptEngine.ensureXhsHomePage() {
    val root = getRootNode() ?: return
    
    // 方法1: 查找底部导航栏的"首页"按钮
    val homeTab = findMatchingNodeEnhanced(root, "首页", null, null)
    if (homeTab != null) {
        log("🏠 找到首页按钮，点击回到首页")
        val rect = android.graphics.Rect()
        homeTab.getBoundsInScreen(rect)
        performTap(rect.centerX(), rect.centerY())
        delay(1000)
        return
    }
    
    // 方法2: 如果找不到首页按钮，尝试按返回键直到到达首页
    for (i in 0 until 3) {
        service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
        delay(800)
        
        val root2 = getRootNode() ?: continue
        val home2 = findMatchingNodeEnhanced(root2, "首页", null, null)
        if (home2 != null) {
            val rect = android.graphics.Rect()
            home2.getBoundsInScreen(rect)
            performTap(rect.centerX(), rect.centerY())
            delay(1000)
            log("🏠 已回到首页")
            return
        }
    }
    
    log("⚠️ 未能确保回到首页，可能已经在首页")
}

internal suspend fun ScriptEngine.executeTap(step: ScriptStep): StepResult {
    val x = (step.params["x"] as? Number)?.toInt()
    val y = (step.params["y"] as? Number)?.toInt()
    val text = step.params["text"] as? String
    
    return if (x != null && y != null) {
        performTap(x, y)
    } else if (text != null) {
        findAndTapByText(text)
    } else {
        StepResult(false, "Missing tap coordinates or text")
    }
}

internal suspend fun ScriptEngine.executeSwipe(step: ScriptStep): StepResult {
    val direction = step.params["direction"] as? String ?: "up"
    return performSwipe(direction)
}

internal suspend fun ScriptEngine.executeWait(step: ScriptStep): StepResult {
    val ms = (step.params["ms"] as? Number)?.toLong() ?: 1000
    delay(ms)
    return StepResult(true)
}

internal suspend fun ScriptEngine.executeFindAndTap(step: ScriptStep): StepResult {
    val text = step.params["text"] as? String
    val contains = step.params["contains"] as? String
    val pattern = step.params["pattern"] as? String
    
    log("🔍 FIND_AND_TAP: text=$text, contains=$contains, pattern=$pattern")
    
    val root = getRootNode()
    if (root == null) {
        val error = "无法获取当前窗口"
        debugInterface.recordError("NO_WINDOW", error, context = mapOf(
            "step_type" to "FIND_AND_TAP"
        ), suggestion = "确保无障碍服务已启用且有活动窗口")
        return StepResult(false, error)
    }
    
    // 遍历查找匹配元素（使用增强版）
    val target = findMatchingNodeEnhanced(root, text, contains, pattern)
    if (target != null) {
        val rect = android.graphics.Rect()
        target.getBoundsInScreen(rect)
        log("✅ 找到元素，点击坐标: (${rect.centerX()}, ${rect.centerY()})")
        return performTap(rect.centerX(), rect.centerY())
    }
    
    // 收集当前页面信息用于诊断
    val visibleTexts = mutableListOf<String>()
    collectAllTexts(root, visibleTexts, 30)
    
    val error = "未找到目标元素: text=$text, contains=$contains, pattern=$pattern"
    debugInterface.recordError("ELEMENT_NOT_FOUND", error, context = mapOf(
        "search_text" to (text ?: ""),
        "search_contains" to (contains ?: ""),
        "search_pattern" to (pattern ?: ""),
        "visible_texts" to visibleTexts.take(15).joinToString(", ")
    ), suggestion = "检查目标文本是否正确，或尝试使用 contains 模糊匹配")
    
    return StepResult(false, error)
}

internal suspend fun ScriptEngine.executeScrollUntilFind(step: ScriptStep): StepResult {
    val text = step.params["text"] as? String
    val contains = step.params["contains"] as? String
    val pattern = step.params["pattern"] as? String
    val maxScrolls = (step.params["max_scrolls"] as? Number)?.toInt() ?: 10
    val direction = step.params["direction"] as? String ?: "up"
    val tapAfterFind = step.params["tap"] as? Boolean ?: true
    
    // 🆕 排除条件：避免匹配到直播等无效内容
    val excludes = step.params["excludes"] as? List<*> ?: emptyList<String>()
    val excludePatterns = excludes.mapNotNull { it as? String }
    
    log("🔍 SCROLL_UNTIL_FIND: text=$text, contains=$contains, pattern=$pattern")
    if (excludePatterns.isNotEmpty()) {
        log("🚫 排除关键词: ${excludePatterns.joinToString(", ")}")
    }
    
    var attempts = 0
    val maxAttempts = 3  // 最多找3个匹配项（如果前面的被排除）
    
    for (i in 0 until maxScrolls) {
        val root = getRootNode() ?: continue
        
        // 调试：打印当前可见的文本元素（仅在前3次滚动时）
        if (i < 3) {
            val visibleTexts = mutableListOf<String>()
            collectAllTexts(root, visibleTexts, 20)
            log("📋 当前可见元素 (前20个): ${visibleTexts.take(10).joinToString(", ")}")
        }
        
        // 🆕 使用增强版查找，支持排除条件
        val target = findMatchingNodeWithExcludes(root, text, contains, pattern, excludePatterns)
        
        if (target != null) {
            val matchedText = target.text?.toString() ?: target.contentDescription?.toString() ?: ""
            log("✅ 找到匹配元素: ${matchedText.take(50)}...")
            
            if (tapAfterFind) {
                val rect = android.graphics.Rect()
                target.getBoundsInScreen(rect)
                val tapResult = performTap(rect.centerX(), rect.centerY())
                
                // 🆕 点击后验证：检查是否进入了有效页面（非直播）
                delay(2000)  // 等待页面加载
                val pageValidation = validatePageAfterTap()
                
                if (pageValidation.isValid) {
                    return tapResult
                } else {
                    // 进入了无效页面（如直播），返回重试
                    log("⚠️ 进入了无效页面: ${pageValidation.reason}，返回重试...")
                    service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
                    delay(1000)
                    attempts++
                    
                    if (attempts >= maxAttempts) {
                        return StepResult(false, "尝试 $maxAttempts 次都进入无效页面")
                    }
                    
                    // 继续滚动查找下一个
                    performSwipe(direction)
                    delay(1000)
                    continue
                }
            }
            return StepResult(true)
        }
        
        log("📜 滚动 ${i + 1}/$maxScrolls...")
        performSwipe(direction)
        delay(1000)
    }
    
    val error = "滚动 $maxScrolls 次后未找到目标元素"
    debugInterface.recordError("SCROLL_FIND_FAILED", error, context = mapOf(
        "search_text" to (text ?: ""),
        "search_contains" to (contains ?: ""),
        "search_pattern" to (pattern ?: ""),
        "max_scrolls" to maxScrolls.toString(),
        "direction" to direction
    ), suggestion = "增加 max_scrolls 次数，或检查目标文本是否在页面中存在")
    
    return StepResult(false, error)
}

/**
 * 🆕 验证点击后的页面是否有效（非直播、有评论区等）
 */
private data class PageValidation(val isValid: Boolean, val reason: String)

internal fun ScriptEngine.validatePageAfterTap(): PageValidation {
    val root = getRootNode() ?: return PageValidation(false, "无法获取页面")
    
    val allTexts = mutableListOf<String>()
    collectAllTexts(root, allTexts, 50)
    val pageContent = allTexts.joinToString(" ")
    
    // 检测直播页面特征
    val liveIndicators = listOf("人观看", "正在直播", "直播中", "连麦", "礼物", "在线", "送礼")
    for (indicator in liveIndicators) {
        if (pageContent.contains(indicator)) {
            return PageValidation(false, "这是直播页面 (包含 '$indicator')")
        }
    }
    
    // 检测笔记/视频页面特征（应该有评论相关元素）
    val validIndicators = listOf("评论", "赞", "收藏", "分享", "写评论", "回复")
    val hasValidIndicator = validIndicators.any { pageContent.contains(it) }
    
    if (!hasValidIndicator) {
        return PageValidation(false, "页面缺少评论区特征")
    }
    
    return PageValidation(true, "有效的笔记/视频页面")
}

/**
 * 🆕 带排除条件的节点查找
 */
internal fun ScriptEngine.findMatchingNodeWithExcludes(
    node: android.view.accessibility.AccessibilityNodeInfo,
    text: String?,
    contains: String?,
    pattern: String?,
    excludes: List<String>
): android.view.accessibility.AccessibilityNodeInfo? {
    val nodeText = node.text?.toString() ?: ""
    val nodeDesc = node.contentDescription?.toString() ?: ""
    val combined = "$nodeText $nodeDesc"
    
    // 先检查排除条件
    if (excludes.isNotEmpty()) {
        for (exclude in excludes) {
            if (combined.contains(exclude, ignoreCase = true)) {
                // 被排除，跳过这个节点
                // 但继续检查子节点
                break
            }
        }
    }
    
    // 检查是否匹配且不被排除
    val isMatch = when {
        text != null -> nodeText == text || nodeDesc == text
        contains != null -> combined.contains(contains, ignoreCase = true)
        pattern != null -> Regex(pattern).containsMatchIn(combined)
        else -> false
    }
    
    val isExcluded = excludes.any { combined.contains(it, ignoreCase = true) }
    
    if (isMatch && !isExcluded) {
        log("🎯 匹配: '$combined'")
        return node
    }
    
    // 递归检查子节点
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        val result = findMatchingNodeWithExcludes(child, text, contains, pattern, excludes)
        if (result != null) return result
    }
    
    return null
}

// 收集所有文本元素用于调试
internal fun ScriptEngine.collectAllTexts(node: android.view.accessibility.AccessibilityNodeInfo, results: MutableList<String>, maxCount: Int) {
    if (results.size >= maxCount) return
    val text = node.text?.toString()?.trim()
    val desc = node.contentDescription?.toString()?.trim()
    if (!text.isNullOrEmpty()) results.add(text)
    else if (!desc.isNullOrEmpty()) results.add(desc)
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        collectAllTexts(child, results, maxCount)
    }
}

internal suspend fun ScriptEngine.executeExtractData(step: ScriptStep): StepResult {
    val field = step.params["field"] as? String ?: "data"
    val selector = step.params["selector"] as? String
    val count = (step.params["count"] as? Number)?.toInt() ?: 5
    
    val root = getRootNode() ?: return StepResult(false, "No window")
    val extractedItems = mutableListOf<String>()
    
    // 根据字段类型选择不同的提取策略
    when (field.lowercase()) {
        "comments", "评论" -> extractComments(root, extractedItems, count)
        "likes", "点赞" -> extractLikes(root, extractedItems, count)
        else -> extractTexts(root, extractedItems, count)
    }
    
    log("📊 提取到 ${extractedItems.size} 条 $field 数据")
    
    return StepResult(true, data = mapOf(field to extractedItems))
}

/**
 * 智能提取评论
 * 小红书评论格式特征：
 * 1. 用户名 + 内容，通常包含 ":" 或在相邻节点
 * 2. 评论区通常有 "回复"、"赞" 按钮
 * 3. 过滤掉系统文本（如"展开更多"、"查看全部"）
 */
internal fun ScriptEngine.extractComments(
    node: android.view.accessibility.AccessibilityNodeInfo,
    results: MutableList<String>,
    maxCount: Int
) {
    val allTexts = mutableListOf<Pair<String, android.graphics.Rect>>()
    collectAllTextWithBounds(node, allTexts)
    
    // 过滤出可能是评论的文本
    val systemTexts = setOf(
        "展开更多", "查看全部", "回复", "赞", "分享", "收藏", 
        "评论", "写评论", "发送", "取消", "确定", "全部评论",
        "相关推荐", "猜你喜欢", "更多精彩", "查看更多"
    )
    
    // 评论通常较长，包含用户名和内容
    for ((text, rect) in allTexts) {
        if (results.size >= maxCount) break
        
        // 跳过系统文本
        if (systemTexts.any { text.contains(it) }) continue
        
        // 跳过太短或太长的文本
        if (text.length < 8 || text.length > 500) continue
        
        // 跳过纯数字（可能是点赞数）
        if (text.matches(Regex("""^\d+\.?\d*[万亿]*$"""))) continue
        
        // 评论特征：包含用户名分隔符或明显的评论格式
        val isComment = text.contains(":") || 
                       text.contains("：") ||
                       text.matches(Regex(""".*@.*:.*""")) ||
                       text.matches(Regex(""".{2,20}[:：].{5,}""")) ||  // 用户名:内容
                       (text.length > 15 && !text.contains("\n"))  // 较长的单行文本可能是评论
        
        if (isComment || text.length > 20) {
            results.add(text)
            log("📝 提取评论: ${text.take(50)}...")
        }
    }
    
    // 如果提取不够，降低标准再试
    if (results.size < maxCount) {
        for ((text, rect) in allTexts) {
            if (results.size >= maxCount) break
            if (results.contains(text)) continue
            if (systemTexts.any { text.contains(it) }) continue
            if (text.length in 10..200) {
                results.add(text)
                log("📝 补充评论: ${text.take(50)}...")
            }
        }
    }
}

internal fun ScriptEngine.collectAllTextWithBounds(
    node: android.view.accessibility.AccessibilityNodeInfo,
    results: MutableList<Pair<String, android.graphics.Rect>>
) {
    val text = node.text?.toString()?.trim()
    val desc = node.contentDescription?.toString()?.trim()
    val rect = android.graphics.Rect()
    node.getBoundsInScreen(rect)
    
    if (!text.isNullOrEmpty()) {
        results.add(text to rect)
    } else if (!desc.isNullOrEmpty() && desc.length > 5) {
        results.add(desc to rect)
    }
    
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        collectAllTextWithBounds(child, results)
    }
}

/**
 * 提取点赞数
 */
internal fun ScriptEngine.extractLikes(
    node: android.view.accessibility.AccessibilityNodeInfo,
    results: MutableList<String>,
    maxCount: Int
) {
    val allTexts = mutableListOf<String>()
    extractAllTexts(node, allTexts)
    
    // 查找包含点赞数格式的文本
    val likePattern = Regex("""(\d+\.?\d*[万亿]?\s*(?:赞|点赞|喜欢))|((?:赞|点赞|喜欢)\s*\d+\.?\d*[万亿]?)""")
    for (text in allTexts) {
        if (results.size >= maxCount) break
        if (likePattern.containsMatchIn(text)) {
            results.add(text)
        }
    }
}

internal fun ScriptEngine.extractAllTexts(
    node: android.view.accessibility.AccessibilityNodeInfo,
    results: MutableList<String>
) {
    val text = node.text?.toString()?.trim()
    if (!text.isNullOrEmpty()) {
        results.add(text)
    }
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        extractAllTexts(child, results)
    }
}

internal suspend fun ScriptEngine.executeInputText(step: ScriptStep): StepResult {
    val text = step.params["text"] as? String ?: return StepResult(false, "Missing text")
    
    log("⌨️ 输入文本: $text")
    
    // 方法1：通过无障碍服务的 ACTION_SET_TEXT
    val root = getRootNode()
    if (root != null) {
        // 查找当前聚焦的可编辑元素
        val focusedNode = root.findFocus(android.view.accessibility.AccessibilityNodeInfo.FOCUS_INPUT)
        if (focusedNode != null && focusedNode.isEditable) {
            val args = android.os.Bundle().apply {
                putCharSequence(
                    android.view.accessibility.AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                    text
                )
            }
            val success = focusedNode.performAction(
                android.view.accessibility.AccessibilityNodeInfo.ACTION_SET_TEXT,
                args
            )
            focusedNode.recycle()
            if (success) {
                log("✅ 文本输入成功 (ACTION_SET_TEXT)")
                delay(300) // 等待输入完成
                return StepResult(true)
            }
        }
        
        // 方法2：查找第一个可编辑的输入框
        val editableNode = findFirstEditableNode(root)
        if (editableNode != null) {
            // 先点击获取焦点
            val rect = android.graphics.Rect()
            editableNode.getBoundsInScreen(rect)
            performTap(rect.centerX(), rect.centerY())
            delay(300)
            
            // 然后设置文本
            val args = android.os.Bundle().apply {
                putCharSequence(
                    android.view.accessibility.AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                    text
                )
            }
            val success = editableNode.performAction(
                android.view.accessibility.AccessibilityNodeInfo.ACTION_SET_TEXT,
                args
            )
            editableNode.recycle()
            if (success) {
                log("✅ 文本输入成功 (找到输入框并设置)")
                delay(300)
                return StepResult(true)
            }
        }
    }
    
    // 方法3：通过 ADB input text 命令（备用方案）
    try {
        val runtime = Runtime.getRuntime()
        // 对特殊字符进行转义
        val escapedText = text.replace(" ", "%s")
        val process = runtime.exec(arrayOf("su", "-c", "input text '$escapedText'"))
        val exitCode = process.waitFor()
        if (exitCode == 0) {
            log("✅ 文本输入成功 (input text)")
            delay(300)
            return StepResult(true)
        }
    } catch (e: Exception) {
        log("⚠️ input text 命令失败: ${e.message}")
    }
    
    return StepResult(false, "无法输入文本，请确保输入框已获得焦点")
}

/**
 * 查找第一个可编辑的输入框
 */
internal fun ScriptEngine.findFirstEditableNode(node: android.view.accessibility.AccessibilityNodeInfo): android.view.accessibility.AccessibilityNodeInfo? {
    if (node.isEditable && node.isVisibleToUser) {
        return node
    }
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        val result = findFirstEditableNode(child)
        if (result != null) return result
        child.recycle()
    }
    return null
}

internal suspend fun ScriptEngine.executeBack(step: ScriptStep): StepResult {
    service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
    delay(500)
    return StepResult(true)
}

internal suspend fun ScriptEngine.executeAssert(step: ScriptStep): StepResult {
    val condition = step.condition ?: return StepResult(false, "No condition")
    val root = getRootNode() ?: return StepResult(false, "No window")
    val texts = mutableListOf<String>()
    extractTexts(root, texts, 100)
    val screenText = texts.joinToString("\n")
    val expected = condition.value.toString()
    val target = condition.target
    val matched = when (condition.type) {
        ConditionType.TEXT_CONTAINS, ConditionType.ELEMENT_EXISTS -> {
            screenText.contains(expected, ignoreCase = true) ||
                target.isNotBlank() && screenText.contains(target, ignoreCase = true)
        }
        ConditionType.TEXT_MATCHES -> {
            runCatching { Regex(expected).containsMatchIn(screenText) }
                .getOrElse { screenText.contains(expected, ignoreCase = true) }
        }
        else -> {
            return StepResult(false, "Unsupported assert condition: ${condition.type}")
        }
    }
    return if (matched) {
        StepResult(true)
    } else {
        StepResult(false, "断言失败: ${condition.type} target=${condition.target} expected=$expected")
    }
}

internal suspend fun ScriptEngine.executeAIDecide(step: ScriptStep): StepResult {
    val goal = step.params["goal"] as? String ?: step.description
    log("🤖 AI 决策: $goal")
    
    // 获取当前屏幕状态
    val root = getRootNode() ?: return StepResult(false, "No window")
    val elements = collectElements(root)
    
    // 调用 AI 决策
    val prompt = """
当前屏幕元素:
$elements

目标: $goal

请决定下一步操作，返回 JSON:
{"action":"tap/swipe/wait","params":{...}}
""".trimIndent()
    
    val messages = listOf(Message(role = "user", content = prompt))
    val response = aiClient.chat(messages)
    return executeStructuredAIDecision(response)
}

internal suspend fun ScriptEngine.executeStructuredAIDecision(response: String): StepResult {
    return try {
        val json = extractJson(response)
        val map = gson.fromJson<Map<String, Any>>(json, object : TypeToken<Map<String, Any>>() {}.type)
        val action = (map["action"] as? String)?.lowercase()?.trim()
            ?: return StepResult(false, "AI 决策缺少 action")
        val params = map["params"] as? Map<*, *> ?: emptyMap<String, Any>()
        when (action) {
            "tap", "click" -> {
                val x = numberParam(params, "x")
                val y = numberParam(params, "y")
                val text = params["text"] as? String
                when {
                    x != null && y != null -> performTap(x, y)
                    !text.isNullOrBlank() -> {
                        val root = getRootNode() ?: return StepResult(false, "No window")
                        val node = findMatchingNodeEnhanced(root, text, null, null)
                            ?: findMatchingNodeEnhanced(root, null, text, null)
                            ?: return StepResult(false, "未找到可点击文本: $text")
                        val rect = android.graphics.Rect()
                        node.getBoundsInScreen(rect)
                        performTap(rect.centerX(), rect.centerY())
                    }
                    else -> StepResult(false, "tap 需要 x/y 或 text 参数")
                }
            }
            "swipe" -> {
                val direction = params["direction"] as? String
                    ?: return StepResult(false, "swipe 需要 direction 参数")
                performSwipe(direction)
            }
            "wait" -> {
                val ms = numberParam(params, "ms")
                    ?: numberParam(params, "waitMs")
                    ?: 1000
                delay(ms.toLong().coerceIn(100L, 10_000L))
                StepResult(true)
            }
            "back" -> {
                service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
                delay(500)
                StepResult(true)
            }
            else -> StepResult(false, "不支持的 AI 决策动作: $action")
        }
    } catch (e: Exception) {
        StepResult(false, "AI 决策解析失败: ${e.message}")
    }
}

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
