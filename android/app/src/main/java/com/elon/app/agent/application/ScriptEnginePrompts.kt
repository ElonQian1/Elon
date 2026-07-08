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

// ===== [ScriptEnginePrompts.kt] =====
// ========== AI Prompt 构建 ==========

internal fun ScriptEngine.buildScriptGenerationPrompt(goal: String): String {
    return """
你是一个自动化脚本生成专家。根据用户目标，生成一个可复用的自动化脚本。

## 用户目标
$goal

## 输出格式 (严格 JSON)
{
  "name": "脚本名称",
  "steps": [
{
  "index": 1,
  "type": "LAUNCH_APP|TAP|SWIPE|WAIT|FIND_AND_TAP|SCROLL_UNTIL_FIND|EXTRACT_DATA|BACK|AI_DECIDE",
  "description": "步骤描述",
  "params": { ... },
  "on_fail": "RETRY|SKIP|ABORT|AI_TAKEOVER",
  "max_retries": 3
}
  ],
  "outputs": ["expected_output_1", "expected_output_2"]
}

## 可用步骤类型
1. LAUNCH_APP - 启动应用 {"package": "com.xingin.xhs"}
2. TAP - 点击 {"x": 100, "y": 200} 或 {"text": "搜索"}
3. SWIPE - 滑动 {"direction": "up|down|left|right"}
4. WAIT - 等待 {"ms": 1000}
5. FIND_AND_TAP - 查找并点击 {"text": "精确文本"} 或 {"contains": "包含文本"} 或 {"pattern": "正则表达式"}
6. INPUT_TEXT - 在当前聚焦的输入框中输入文本 {"text": "要输入的内容"}
   ⚠️ 必须先点击输入框使其获得焦点，再使用此步骤！
7. SCROLL_UNTIL_FIND - 滚动直到找到并**自动点击** 
   参数: {"contains": "文本", "max_scrolls": 10, "direction": "up", "excludes": ["排除词1", "排除词2"]}
   ⚠️ 注意：此步骤会自动点击找到的元素，不需要额外的TAP或FIND_AND_TAP步骤！
   ⚠️ 重要：使用 excludes 参数排除不想要的内容类型（如直播）
8. EXTRACT_DATA - 提取数据 {"field": "comments", "count": 5}
9. BACK - 返回 {}
10. AI_DECIDE - AI动态决策 {"goal": "子目标描述"}

## ⚠️ 关键规则
1. **禁止使用占位符文本**！如"笔记标题"、"目标内容"等。必须使用 contains 或 pattern 匹配真实内容
2. **SCROLL_UNTIL_FIND 会自动点击**：找到后会自动点击进入，不需要再加FIND_AND_TAP步骤
3. **数字匹配优先用正则**：查找"点赞过万"应使用 {"contains": "万"}
4. **搜索操作的正确流程**：
   - 点击搜索框 → INPUT_TEXT输入关键词 → 点击搜索按钮
   - ⚠️ 错误做法：直接SCROLL_UNTIL_FIND搜索关键词（这是滚动查找，不是搜索！）
5. **小红书特殊处理**：
   - 笔记点赞数通常显示在笔记卡片右下角，格式如"1.2万"、"8.5w"、"12345"
   - 评论区通常需要向上滑动才能看到
   - ⚠️ **直播卡片没有评论区**！要提取评论时，必须排除直播！使用 excludes: ["直播", "观看", "连麦"]
6. **步骤要精简**：SCROLL_UNTIL_FIND找到并点击后，直接WAIT然后继续下一步

## 常用APP包名（⚠️ 必须使用正确的包名！）
- 小红书: com.xingin.xhs
- 京东: com.jingdong.app.mall
- 淘宝: com.taobao.taobao
- 抖音: com.ss.android.ugc.aweme
- 微信: com.tencent.mm
- QQ: com.tencent.mobileqq
- 微博: com.sina.weibo
- B站: tv.danmaku.bili
- 支付宝: com.eg.android.AlipayGphone
- 钉钉: com.alibaba.android.rimet
- 高德地图: com.autonavi.minimap
- 百度地图: com.baidu.BaiduMap
- 网易云音乐: com.netease.cloudmusic
- 酷狗音乐: com.kugou.android

## 示例：获取小红书热门评论（排除直播）
{
  "name": "获取小红书点赞过万笔记评论",
  "steps": [
{"index": 1, "type": "LAUNCH_APP", "description": "打开小红书", "params": {"package": "com.xingin.xhs"}, "on_fail": "RETRY", "max_retries": 3},
{"index": 2, "type": "WAIT", "description": "等待首页加载", "params": {"ms": 2500}, "on_fail": "SKIP", "max_retries": 1},
{"index": 3, "type": "SCROLL_UNTIL_FIND", "description": "滚动找到点赞过万的笔记并点击进入（排除直播）", "params": {"contains": "万赞", "excludes": ["直播", "观看", "连麦", "在线"], "max_scrolls": 15, "direction": "up"}, "on_fail": "RETRY", "max_retries": 2},
{"index": 4, "type": "WAIT", "description": "等待笔记详情加载", "params": {"ms": 2000}, "on_fail": "SKIP", "max_retries": 1},
{"index": 5, "type": "SWIPE", "description": "向上滑动查看评论区", "params": {"direction": "up"}, "on_fail": "RETRY", "max_retries": 3},
{"index": 6, "type": "EXTRACT_DATA", "description": "提取前5条评论", "params": {"field": "comments", "count": 5}, "on_fail": "AI_TAKEOVER", "max_retries": 2}
  ],
  "outputs": ["comments"]
}

## 示例：在京东搜索商品（⚠️ 搜索操作必须这样做！）
{
  "name": "京东搜索CPU",
  "steps": [
{"index": 1, "type": "LAUNCH_APP", "description": "打开京东", "params": {"package": "com.jingdong.app.mall"}, "on_fail": "RETRY", "max_retries": 3},
{"index": 2, "type": "WAIT", "description": "等待京东首页加载", "params": {"ms": 3000}, "on_fail": "SKIP", "max_retries": 1},
{"index": 3, "type": "FIND_AND_TAP", "description": "点击顶部搜索框", "params": {"contains": "搜索"}, "on_fail": "RETRY", "max_retries": 3},
{"index": 4, "type": "WAIT", "description": "等待搜索页加载", "params": {"ms": 1500}, "on_fail": "SKIP", "max_retries": 1},
{"index": 5, "type": "INPUT_TEXT", "description": "输入搜索关键词", "params": {"text": "CPU"}, "on_fail": "RETRY", "max_retries": 3},
{"index": 6, "type": "FIND_AND_TAP", "description": "点击搜索按钮", "params": {"text": "搜索"}, "on_fail": "RETRY", "max_retries": 3},
{"index": 7, "type": "WAIT", "description": "等待搜索结果加载", "params": {"ms": 3000}, "on_fail": "SKIP", "max_retries": 1}
  ],
  "outputs": ["search_results"]
}

注意：SCROLL_UNTIL_FIND 在第3步找到并点击了笔记，不需要额外的FIND_AND_TAP步骤！

请根据用户目标生成脚本，只返回 JSON，不要其他内容。
""".trimIndent()
}

internal fun ScriptEngine.buildImprovementPrompt(script: Script, failResult: ScriptExecutionResult): String {
    return """
你是脚本优化专家。脚本执行失败，请分析原因并改进。

## 原脚本
${gson.toJson(script)}

## 执行结果
- 成功步骤: ${failResult.stepsExecuted}/${failResult.totalSteps}
- 失败步骤: ${failResult.failedStepIndex?.plus(1) ?: "未知"}
- 错误: ${failResult.error}
- 日志: ${failResult.logs.joinToString("\n")}

## 要求
1. 分析失败原因
2. 改进失败的步骤（增加重试、调整等待时间、换用 AI_DECIDE 等）
3. 返回改进后的 steps 数组（只返回 steps，JSON 格式）

## 改进策略
- 如果是元素找不到：增加等待时间、改用 SCROLL_UNTIL_FIND、或使用 AI_DECIDE
- 如果是点击失败：改用 FIND_AND_TAP、调整坐标
- 如果是超时：增加 max_retries

只返回改进后的 steps JSON 数组，不要其他内容。
""".trimIndent()
}

internal fun ScriptEngine.parseScriptFromAI(response: String, goal: String): Script? {
    return try {
        // 提取 JSON
        val jsonStr = extractJson(response)
        val parsed = gson.fromJson(jsonStr, Map::class.java)
        
        val name = parsed["name"] as? String ?: "未命名脚本"
        val stepsRaw = parsed["steps"] as? List<*> ?: return null
        val outputs = (parsed["outputs"] as? List<*>)?.mapNotNull { it as? String } ?: emptyList()
        
        val steps = stepsRaw.mapIndexed { index, stepRaw ->
            val stepMap = stepRaw as? Map<*, *> ?: return@mapIndexed null
            val typeStr = stepMap["type"] as? String ?: "WAIT"
            
            // 容错处理：映射未知类型到已知类型
            val type = try {
                StepType.valueOf(typeStr)
            } catch (e: IllegalArgumentException) {
                mapUnknownStepType(typeStr)
            }
            
            ScriptStep(
                index = (stepMap["index"] as? Number)?.toInt() ?: (index + 1),
                type = type,
                description = stepMap["description"] as? String ?: "",
                params = (stepMap["params"] as? Map<*, *>)?.mapKeys { it.key.toString() }?.mapValues { it.value as Any } ?: emptyMap(),
                onFail = try { FailAction.valueOf(stepMap["on_fail"] as? String ?: "RETRY") } catch (e: Exception) { FailAction.RETRY },
                maxRetries = (stepMap["max_retries"] as? Number)?.toInt() ?: 3
            )
        }.filterNotNull()
        
        Script(
            id = UUID.randomUUID().toString(),
            name = name,
            goal = goal,
            steps = steps,
            outputs = outputs
        )
    } catch (e: Exception) {
        Log.e(TAG, "Failed to parse script", e)
        null
    }
}

internal fun ScriptEngine.parseImprovedSteps(response: String): List<ScriptStep>? {
    return try {
        val jsonStr = extractJson(response)
        
        // AI 可能返回 { "steps": [...] } 或直接 [...]
        val stepsRaw: List<*> = try {
            // 首先尝试解析为数组
            gson.fromJson(jsonStr, List::class.java) as? List<*> ?: run {
                // 如果失败，尝试解析为对象并提取 steps
                val obj = gson.fromJson(jsonStr, Map::class.java) as? Map<*, *>
                obj?.get("steps") as? List<*> ?: return null
            }
        } catch (e: Exception) {
            // 解析为对象并提取 steps
            val obj = gson.fromJson(jsonStr, Map::class.java) as? Map<*, *>
            obj?.get("steps") as? List<*> ?: return null
        }
        
        stepsRaw.mapIndexed { index, stepRaw ->
            val stepMap = stepRaw as? Map<*, *> ?: return@mapIndexed null
            val typeStr = stepMap["type"] as? String ?: "WAIT"
            
            // 复用相同的类型映射逻辑
            val type = try {
                StepType.valueOf(typeStr)
            } catch (e: IllegalArgumentException) {
                mapUnknownStepType(typeStr)
            }
            
            ScriptStep(
                index = (stepMap["index"] as? Number)?.toInt() ?: (index + 1),
                type = type,
                description = stepMap["description"] as? String ?: "",
                params = (stepMap["params"] as? Map<*, *>)?.mapKeys { it.key.toString() }?.mapValues { it.value as Any } ?: emptyMap(),
                onFail = try { FailAction.valueOf(stepMap["on_fail"] as? String ?: "RETRY") } catch (e: Exception) { FailAction.RETRY },
                maxRetries = (stepMap["max_retries"] as? Number)?.toInt() ?: 3
            )
        }.filterNotNull()
    } catch (e: Exception) {
        Log.e(TAG, "Failed to parse improved steps", e)
        null
    }
}

/**
 * 将未知步骤类型映射到已知类型
 */
internal fun ScriptEngine.mapUnknownStepType(typeStr: String): StepType {
    log("⚠️ 未知步骤类型 '$typeStr'，尝试智能映射...")
    return when {
        typeStr.contains("SEARCH", ignoreCase = true) -> StepType.SEARCH
        typeStr.contains("CLICK", ignoreCase = true) -> StepType.TAP
        typeStr.contains("SCROLL", ignoreCase = true) -> StepType.SCROLL_UNTIL_FIND
        typeStr.contains("FIND", ignoreCase = true) -> StepType.FIND_AND_TAP
        typeStr.contains("INPUT", ignoreCase = true) -> StepType.INPUT_TEXT
        typeStr.contains("TYPE", ignoreCase = true) -> StepType.INPUT_TEXT
        typeStr.contains("DELAY", ignoreCase = true) -> StepType.WAIT
        typeStr.contains("SLEEP", ignoreCase = true) -> StepType.WAIT
        typeStr.contains("OPEN", ignoreCase = true) -> StepType.LAUNCH_APP
        typeStr.contains("LAUNCH", ignoreCase = true) -> StepType.LAUNCH_APP
        typeStr.contains("EXTRACT", ignoreCase = true) -> StepType.EXTRACT_DATA
        typeStr.contains("GET", ignoreCase = true) -> StepType.EXTRACT_DATA
        else -> {
            log("⚠️ 无法映射类型 '$typeStr'，使用 AI_DECIDE")
            StepType.AI_DECIDE
        }
    }
}

internal fun ScriptEngine.extractJson(text: String): String {
    // 尝试提取 JSON
    val jsonPattern = Regex("""\{[\s\S]*\}|\[[\s\S]*\]""")
    return jsonPattern.find(text)?.value ?: text
}

internal fun ScriptEngine.incrementVersion(version: String): String {
    val parts = version.split(".")
    return if (parts.size >= 2) {
        "${parts[0]}.${(parts[1].toIntOrNull() ?: 0) + 1}"
    } else {
        "1.1"
    }
}

internal fun ScriptEngine.log(message: String) {
    Log.d(TAG, message)
    onLog?.invoke(message)
}

// ==================== 📸 屏幕模式自动切换 ====================

/**
 * 根据场景自动切换屏幕获取模式
 * 
 * 切换策略：
 * - 首次分析/AI恢复 → FULL_DUMP（需要完整上下文）
 * - 等待变化/检测 → INCREMENTAL（低延迟监控）  
 * - 验证结果/确认 → DIFF（精确对比）
 */
internal fun ScriptEngine.autoSwitchScreenMode(scenario: String, targetMode: ScreenCaptureMode) {
    if (!autoScreenModeSwitch) {
        log("📸 屏幕模式自动切换已禁用")
        return
    }
    
    val smartReader = AgentService.getInstance()?.smartScreenReader
    if (smartReader == null) {
        log("⚠️ SmartScreenReader 未初始化，跳过模式切换")
        return
    }
    
    val currentMode = smartReader.currentMode
    if (currentMode != targetMode) {
        log("📸 场景「$scenario」: ${currentMode.emoji} ${currentMode.displayName} → ${targetMode.emoji} ${targetMode.displayName}")
        smartReader.setMode(targetMode)
        
        // DIFF 模式自动拍摄基线
        if (targetMode == ScreenCaptureMode.DIFF) {
            smartReader.takeBaselineSnapshot()
            log("📸 已拍摄基线快照")
        }
    }
}

/**
 * 获取当前屏幕模式
 */
internal fun ScriptEngine.getCurrentScreenMode(): ScreenCaptureMode {
    return AgentService.getInstance()?.smartScreenReader?.currentMode 
        ?: ScreenCaptureMode.FULL_DUMP
}

/**
 * 手动设置屏幕模式（覆盖自动切换）
 */
internal fun ScriptEngine.setScreenMode(mode: ScreenCaptureMode) {
    val smartReader = AgentService.getInstance()?.smartScreenReader
    if (smartReader != null) {
        log("📸 手动设置屏幕模式: ${mode.emoji} ${mode.displayName}")
        smartReader.setMode(mode)
    }
}

// ==================== 🎮 执行模式自动切换 ====================

/**
 * 判断是否应该升级到 AGENT 模式
 * 
 * 触发条件：
 * - 连续失败 >= 3 次
 * - AI 介入次数过多（说明脚本不稳定）
 */
internal fun ScriptEngine.shouldUpgradeToAgentMode(): Boolean {
    if (!autoExecutionModeUpgrade) return false
    if (executionMode == ExecutionMode.AGENT) return false // 已经是最高级
    
    return consecutiveFailures >= 3 || totalAiInterventions >= 5
}

/**
 * 判断是否可以降级到 FAST 模式
 * 
 * 触发条件：
 * - 连续成功 >= 10 次
 * - 无 AI 介入
 */
internal fun ScriptEngine.shouldDowngradeToFastMode(): Boolean {
    if (!autoExecutionModeUpgrade) return false
    if (executionMode == ExecutionMode.FAST) return false // 已经是最低级
    
    return consecutiveSuccesses >= 10 && totalAiInterventions == 0
}

/**
 * 自动调整执行模式
 */
internal fun ScriptEngine.autoAdjustExecutionMode() {
    if (!autoExecutionModeUpgrade) return
    
    val oldMode = executionMode
    
    when {
        shouldUpgradeToAgentMode() -> {
            executionMode = ExecutionMode.AGENT
            log("🔄 执行模式自动升级: ${oldMode.emoji} ${oldMode.displayName} → ${executionMode.emoji} ${executionMode.displayName}")
            log("   原因: 连续失败${consecutiveFailures}次，AI介入${totalAiInterventions}次")
        }
        shouldDowngradeToFastMode() -> {
            executionMode = ExecutionMode.FAST
            log("🔄 执行模式自动降级: ${oldMode.emoji} ${oldMode.displayName} → ${executionMode.emoji} ${executionMode.displayName}")
            log("   原因: 连续成功${consecutiveSuccesses}次，执行稳定")
        }
    }
}

/**
 * 重置执行统计
 */
internal fun ScriptEngine.resetExecutionStats() {
    consecutiveFailures = 0
    consecutiveSuccesses = 0
    totalAiInterventions = 0
}

// ==================== 👁️ MONITOR 模式辅助方法 ====================

/**
 * AI 验证步骤执行结果
 */