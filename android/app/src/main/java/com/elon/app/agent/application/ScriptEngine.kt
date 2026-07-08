// src/application/ScriptEngine.kt
// module: script | layer: application | role: script-engine
// summary: 脚本引擎 - 负责脚本的生成、执行、存储和自我改进

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

/**
 * 🚀 脚本引擎
 * 核心功能：
 * 1. AI 生成脚本 - 根据目标自动生成可复用脚本
 * 2. 执行脚本 - 按步骤执行脚本（支持多种执行模式）
 * 3. 自我改进 - 执行失败时 AI 自动优化脚本
 * 4. 持久化 - 保存和加载脚本
 *
 * **重构后**：不再接收 apiKey，通过 [AIClientFactory] 自动选择
 * Hunyuan / OpenAI 兼容 / 服务器 CLI 其中一条可用链路。
 */
class ScriptEngine(
    internal val service: AccessibilityService
) {
    companion object {
        private const val TAG = "ScriptEngine"
        private const val SCRIPTS_DIR = "scripts"
        private const val MAX_IMPROVE_ATTEMPTS = 3
    }
    
    internal val gson: Gson = GsonBuilder().setPrettyPrinting().create()
    internal val aiClient = AIClientFactory.create(service)
    internal val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    
    // 🔧 调试接口
    internal val debugInterface = DebugInterface.getInstance()
    
    // 🛡️ 弹窗清理器
    internal val popupDismisser = PopupDismisser(service)
    
    /**
     * 🆕 获取 Root Window 的辅助函数
     * 
     * 先尝试 rootInActiveWindow，如果为 null 则从 windows 中获取活动窗口的 root
     */
    internal fun getRootNode(): AccessibilityNodeInfo? {
        service.rootInActiveWindow?.let { return it }
        
        try {
            val windows = service.windows
            if (windows != null && windows.isNotEmpty()) {
                // 1. 优先选择 isActive 且 isFocused 的窗口
                for (window in windows) {
                    if (window.isActive && window.isFocused) {
                        window.root?.let { return it }
                    }
                }
                // 2. 选择 isActive 的应用窗口
                for (window in windows) {
                    if (window.isActive && window.type == AccessibilityWindowInfo.TYPE_APPLICATION) {
                        window.root?.let { return it }
                    }
                }
                // 3. 选择任何 isActive 的窗口
                for (window in windows) {
                    if (window.isActive) {
                        window.root?.let { return it }
                    }
                }
                // 4. 兜底
                windows.find { it.type == AccessibilityWindowInfo.TYPE_APPLICATION && it.root != null }?.root?.let { return it }
                for (window in windows) {
                    window.root?.let { return it }
                }
            }
        } catch (_: Exception) {}
        
        return null
    }
    
    // 🎮 当前执行模式（默认智能模式）
    var executionMode: ExecutionMode = ExecutionMode.SMART
    
    // 📸 自动屏幕模式切换（默认开启）
    var autoScreenModeSwitch: Boolean = true
    
    // 🎮 自动执行模式升级（默认开启）
    var autoExecutionModeUpgrade: Boolean = true
    
    // 📊 执行统计（用于智能模式切换）
    internal var consecutiveFailures = 0
    internal var consecutiveSuccesses = 0
    internal var totalAiInterventions = 0
    
    // 脚本缓存
    internal val scriptsCache = mutableMapOf<String, Script>()
    
    // 执行日志回调
    var onLog: ((String) -> Unit)? = null
    
    // ==================== 意图识别 ====================
    
    /**
     * 🧠 用户意图类型
     */
    enum class UserIntent {
        /** 手机操作命令（打开APP、搜索、点击等）*/
        PHONE_OPERATION,
        /** 日常聊天/问答 */
        CHAT,
        /** 不确定 */
        UNKNOWN
    }
    
    /**
     * 🎯 意图分析结果
     */
    data class IntentResult(
        val intent: UserIntent,
        val confidence: Float,
        val chatResponse: String? = null,  // 如果是聊天，直接返回回复
        val operationGoal: String? = null, // 如果是操作，返回清理后的目标
        val isComplete: Boolean = true     // 🆕 表述是否完整
    )
    
    /**
     * 🧠 分析用户意图（前置步骤）
     * 
     * 在生成脚本之前调用，判断用户是想：
     * 1. 操作手机（打开APP、搜索、自动化任务）→ 走脚本流程
     * 2. 日常聊天（闲聊、问答）→ AI 直接回复
     */
    suspend fun analyzeIntent(userInput: String): IntentResult = withContext(Dispatchers.IO) {
        try {
            log("🧠 分析用户意图: $userInput")
            
            // 快速规则匹配（无需调用 AI）
            val quickResult = quickIntentMatch(userInput)
            if (quickResult != null) {
                log("⚡ 快速匹配: ${quickResult.intent}")
                return@withContext quickResult
            }
            
            // 调用 AI 进行意图分析
            val prompt = buildIntentAnalysisPrompt(userInput)
            val messages = listOf(Message(role = "user", content = prompt))
            val response = aiClient.chat(messages)
            
            parseIntentFromAI(response, userInput)
        } catch (e: Exception) {
            log("❌ 意图分析失败: ${e.message}")
            // 🔧 修复：失败时安全降级为聊天（不执行任何操作）
            IntentResult(
                intent = UserIntent.CHAT,
                confidence = 0.0f,
                chatResponse = "抱歉，我没太理解你的意思。你可以试着说得更具体一些，比如「打开微信」或「搜索今日新闻」。"
            )
        }
    }
    
    /**
     * ⚡ 快速规则匹配（无需 AI）
     */
    private fun quickIntentMatch(input: String): IntentResult? {
        val normalized = input.trim().lowercase()
        
        // 明确的操作关键词
        val operationKeywords = listOf(
            "打开", "启动", "运行", "进入",
            "搜索", "查找", "查询", "找",
            "点击", "点一下", "按", "触摸",
            "滑动", "翻页", "向上", "向下", "向左", "向右",
            "返回", "后退", "退出",
            "发送", "转发", "分享", "复制",
            "下载", "安装", "卸载",
            "设置", "修改", "更改",
            "获取", "提取", "采集", "抓取",
            "登录", "注册", "输入"
        )
        
        // APP 名称关键词
        val appKeywords = listOf(
            "小红书", "微信", "抖音", "淘宝", "京东", "支付宝",
            "qq", "微博", "b站", "哔哩哔哩", "美团", "饿了么",
            "高德", "百度地图", "网易云", "酷狗", "喜马拉雅",
            "今日头条", "知乎", "豆瓣", "闲鱼", "拼多多"
        )
        
        // 检查是否包含操作关键词
        val hasOperationKeyword = operationKeywords.any { normalized.contains(it) }
        val hasAppKeyword = appKeywords.any { normalized.contains(it) }
        
        if (hasOperationKeyword || hasAppKeyword) {
            // 🆕 判断完整性：操作词 + APP名 = 完整
            val isComplete = (hasOperationKeyword && hasAppKeyword) || 
                             normalized.length >= 6 ||  // 较长的指令通常完整
                             appKeywords.any { normalized == it }  // 单独APP名也算完整（打开它）
            
            return IntentResult(
                intent = UserIntent.PHONE_OPERATION,
                confidence = 0.95f,
                operationGoal = input,
                isComplete = isComplete
            )
        }
        
        // 🔧 修复：简单问候词直接返回 CHAT（不调用 AI，零延迟）
        val greetingResponses = mapOf(
            "你好" to "你好！我是手机自动化助手，可以帮你操作手机。试试说【打开微信】或【搜索附近美食】。",
            "您好" to "您好！有什么可以帮您的吗？比如【打开淘宝】或【查看今日新闻】。",
            "嗨" to "嗨！我可以帮你操作手机，比如打开APP、搜索内容。试试看！",
            "hi" to "Hi! 我是你的手机助手，有什么需要帮忙的吗？",
            "hello" to "Hello! 有什么可以帮你的？试试说【打开微信】。",
            "谢谢" to "不客气！还有什么需要帮忙的吗？",
            "感谢" to "不用谢！随时可以叫我帮忙。",
            "多谢" to "不用客气！",
            "再见" to "再见！有需要随时叫我。",
            "拜拜" to "拜拜！",
            "bye" to "Bye! 下次再见！"
        )
        
        // 精确匹配问候语（输入就是问候语本身）
        greetingResponses[normalized]?.let { response ->
            return IntentResult(
                intent = UserIntent.CHAT,
                confidence = 1.0f,
                chatResponse = response,
                isComplete = true
            )
        }
        
        // 明确的聊天模式（需要更智能的回复）
        val chatPatterns = listOf(
            "你是谁", "你叫什么", "介绍一下你",
            "今天天气", "天气怎么样",
            "几点了", "现在时间",
            "帮我算", "计算一下",
            "什么意思", "是什么", "怎么理解",
            "讲个笑话", "说个故事",
            "你能做什么", "你会什么", "怎么用"
        )
        
        val isChatPattern = chatPatterns.any { normalized.contains(it) }
        if (isChatPattern && !hasOperationKeyword && !hasAppKeyword) {
            // 聊天模式的快速响应
            val quickChatResponses = mapOf(
                "你是谁" to "我是手机自动化助手，可以帮你操作手机上的各种APP。",
                "你叫什么" to "我是 AI Agent，你的手机自动化助手！",
                "你能做什么" to "我可以帮你打开APP、搜索内容、自动操作手机。比如说【打开微信】或【在京东搜索手机】。",
                "你会什么" to "我擅长自动化操作手机，比如打开APP、搜索、点击按钮等。试试说具体的任务！",
                "怎么用" to "直接告诉我你想做什么，比如【打开小红书】或【搜索今日热点】，我就会帮你操作。"
            )
            
            for ((pattern, response) in quickChatResponses) {
                if (normalized.contains(pattern)) {
                    return IntentResult(
                        intent = UserIntent.CHAT,
                        confidence = 0.9f,
                        chatResponse = response,
                        isComplete = true
                    )
                }
            }
            
            // 其他聊天模式交给 AI 处理
            return null
        }
        
        // 🔧 修复：短输入且不包含任何关键词，大概率是聊天/无意义输入
        if (input.length <= 4 && !hasOperationKeyword && !hasAppKeyword) {
            return IntentResult(
                intent = UserIntent.CHAT,
                confidence = 0.8f,
                chatResponse = "你好！请告诉我你想做什么，比如【打开微信】或【搜索天气】。",
                isComplete = true
            )
        }

        return null  // 无法快速判断，需要 AI
    }
    
    /**
     * 构建意图分析 Prompt
     */
    private fun buildIntentAnalysisPrompt(userInput: String): String {
        return """
你是一个手机 AI 助手的意图分析器。分析用户输入，判断：
1. **意图类型**：操作手机 or 日常聊天
2. **表述完整性**：用户是否已经说完整了

## 用户输入
"$userInput"

## 输出格式（严格 JSON）
{
  "intent": "PHONE_OPERATION 或 CHAT",
  "confidence": 0.0-1.0,
  "isComplete": true或false,
  "reason": "判断理由",
  "response": "如果是CHAT，这里填写回复内容；否则填null",
  "goal": "如果是PHONE_OPERATION，这里填清理后的操作目标；否则填null"
}

## 判断标准

### 意图类型
- **PHONE_OPERATION**：包含动作词（打开、搜索、点击、获取、发送等）+ 目标对象
- **CHAT**：打招呼、问候、闲聊、知识问答、不涉及手机操作

### 完整性判断 (isComplete)
- **true（完整）**：可以直接执行的指令，如"打开微信"、"搜索热门笔记"
- **false（不完整）**：半句话，如"打开"、"帮我找"、"然后"、"在小红书"

## 示例
输入："打开微信" → intent=PHONE_OPERATION, isComplete=true, goal="打开微信"
输入："打开" → intent=PHONE_OPERATION, isComplete=false
输入："帮我找" → intent=PHONE_OPERATION, isComplete=false
输入："在小红书找点赞过万的笔记" → intent=PHONE_OPERATION, isComplete=true
输入："搜索" → intent=PHONE_OPERATION, isComplete=false
输入："你好" → intent=CHAT, isComplete=true, response="你好！有什么可以帮你的？"

只返回 JSON，不要其他内容。
""".trimIndent()
    }
    
    /**
     * 解析 AI 返回的意图分析结果
     */
    private fun parseIntentFromAI(response: String, originalInput: String): IntentResult {
        return try {
            val jsonStr = extractJson(response)
            val parsed = gson.fromJson(jsonStr, Map::class.java)
            
            val intentStr = parsed["intent"] as? String ?: "PHONE_OPERATION"
            val confidence = (parsed["confidence"] as? Number)?.toFloat() ?: 0.8f
            val isComplete = parsed["isComplete"] as? Boolean ?: true
            val chatResponse = parsed["response"] as? String
            val goal = parsed["goal"] as? String
            
            val intent = when (intentStr.uppercase()) {
                "CHAT" -> UserIntent.CHAT
                "PHONE_OPERATION" -> UserIntent.PHONE_OPERATION
                else -> UserIntent.UNKNOWN
            }
            
            IntentResult(
                intent = intent,
                confidence = confidence,
                chatResponse = chatResponse,
                operationGoal = goal ?: originalInput,
                isComplete = isComplete
            )
        } catch (e: Exception) {
            Log.e(TAG, "Failed to parse intent", e)
            // 解析失败，默认当作操作命令
            IntentResult(
                intent = UserIntent.PHONE_OPERATION,
                confidence = 0.5f,
                operationGoal = originalInput
            )
        }
    }
    
    // ==================== 脚本生成 ====================
    
    /**
     * 🎯 根据目标生成脚本
     */
    suspend fun generateScript(goal: String): Result<Script> = withContext(Dispatchers.IO) {
        try {
            log("📝 开始为目标生成脚本: $goal")
            debugInterface.onScriptGenerating(goal)
            
            val prompt = buildScriptGenerationPrompt(goal)
            val messages = listOf(Message(role = "user", content = prompt))
            val response = aiClient.chat(messages)
            
            val script = parseScriptFromAI(response, goal)
            if (script != null) {
                saveScript(script)
                log("✅ 脚本生成成功: ${script.name} (${script.steps.size} 步骤)")
                Result.success(script)
            } else {
                log("❌ 脚本解析失败")
                debugInterface.recordError("SCRIPT_PARSE_ERROR", "脚本解析失败", context = mapOf("goal" to goal))
                Result.failure(Exception("Failed to parse script from AI response"))
            }
        } catch (e: Exception) {
            log("❌ 生成脚本失败: ${e.message}")
            debugInterface.recordError("SCRIPT_GENERATE_ERROR", "生成脚本失败: ${e.message}", e, mapOf("goal" to goal))
            Result.failure(e)
        }
    }
    
    /**
     * ▶️ 执行脚本（使用当前执行模式）
     */
    suspend fun executeScript(
        scriptId: String,
        onProgress: ((Int, Int, String) -> Unit)? = null
    ): ScriptExecutionResult = withContext(Dispatchers.IO) {
        val script = loadScript(scriptId)
        if (script == null) {
            return@withContext ScriptExecutionResult(
                success = false,
                stepsExecuted = 0,
                totalSteps = 0,
                error = "Script not found: $scriptId"
            )
        }
        
        executeScriptWithMode(script, executionMode, onProgress)
    }
    
    /**
     * ▶️ 执行脚本（指定执行模式）
     * 
     * @param scriptId 脚本ID
     * @param mode 执行模式（FAST/SMART/MONITOR/AGENT）
     * @param onProgress 进度回调
     */
    suspend fun executeScriptWithMode(
        scriptId: String,
        mode: ExecutionMode,
        onProgress: ((Int, Int, String) -> Unit)? = null
    ): ScriptExecutionResult = withContext(Dispatchers.IO) {
        val script = loadScript(scriptId)
        if (script == null) {
            return@withContext ScriptExecutionResult(
                success = false,
                stepsExecuted = 0,
                totalSteps = 0,
                error = "Script not found: $scriptId"
            )
        }
        
        executeScriptWithMode(script, mode, onProgress)
    }
    
    /**
     * ▶️ 执行脚本对象（指定执行模式）
     */
    suspend fun executeScriptWithMode(
        script: Script,
        mode: ExecutionMode,
        onProgress: ((Int, Int, String) -> Unit)? = null
    ): ScriptExecutionResult = withContext(Dispatchers.IO) {
        log("▶️ 开始执行脚本: ${script.name} [模式: ${mode.emoji} ${mode.displayName}]")
        
        when (mode) {
            ExecutionMode.FAST -> executeScriptInternal(script, onProgress)
            ExecutionMode.SMART -> executeScriptSmartMode(script, onProgress)
            ExecutionMode.MONITOR -> executeScriptMonitorMode(script, onProgress)
            ExecutionMode.AGENT -> executeScriptAgentMode(script, onProgress)
        }
    }
    
    /**
     * 🛡️ 智能模式执行
     */
    private suspend fun executeScriptSmartMode(
        script: Script,
        onProgress: ((Int, Int, String) -> Unit)?
    ): ScriptExecutionResult {
        val logs = mutableListOf<String>()
        val extractedData = mutableMapOf<String, Any>()
        var popupsDismissed = 0
        var aiInterventions = 0
        
        log("🛡️ 智能模式执行中...")
        debugInterface.onTaskStart(script.id, script.name, script.goal, script.steps.size)
        
        // 🆕 通知状态管理器开始执行
        ExecutionStateManager.startExecution(script.goal, script.steps.size)
        
        // 📸 首次执行：切换到全量模式（需要完整上下文）
        autoSwitchScreenMode("首次分析", ScreenCaptureMode.FULL_DUMP)
        
        for ((index, step) in script.steps.withIndex()) {
            // 🆕 每步执行前检查取消令牌
            if (ExecutionStateManager.shouldCancel()) {
                log("⏹️ 用户请求停止，终止执行")
                ExecutionStateManager.executionStopped()
                debugInterface.onTaskComplete(false, "用户停止")
                return ScriptExecutionResult(
                    success = false,
                    stepsExecuted = index,
                    totalSteps = script.steps.size,
                    extractedData = extractedData,
                    error = "用户停止执行",
                    cancelled = true,
                    logs = logs,
                    popupsDismissed = popupsDismissed,
                    aiInterventions = aiInterventions
                )
            }
            
            val stepNum = index + 1
            log("📍 步骤 $stepNum/${script.steps.size}: ${step.description}")
            onProgress?.invoke(stepNum, script.steps.size, step.description)
            debugInterface.onStepStart(stepNum, step.type.name, step.description)
            
            // 🆕 更新状态管理器
            ExecutionStateManager.updateStep(stepNum, step.description)
            
            // 🛡️ 执行前：自动清理弹窗
            val dismissResult = popupDismisser.dismissAllPopups(maxAttempts = 3, delayMs = 300)
            if (dismissResult.popupsCleared > 0) {
                popupsDismissed += dismissResult.popupsCleared
                logs.add("🛡️ 清理了 ${dismissResult.popupsCleared} 个弹窗")
                log("🛡️ 清理了 ${dismissResult.popupsCleared} 个弹窗")
            }
            
            // 📸 执行步骤前：切换到增量模式（等待变化）
            if (stepNum > 1) {
                autoSwitchScreenMode("等待变化", ScreenCaptureMode.INCREMENTAL)
            }
            
            var retries = 0
            var stepSuccess = false
            var lastError: String? = null
            var attemptStartTime = System.currentTimeMillis()
            
            while (retries <= step.maxRetries && !stepSuccess) {
                try {
                    attemptStartTime = System.currentTimeMillis()
                    val stepResult = executeStep(step, extractedData)
                    if (stepResult.success) {
                        stepSuccess = true
                        stepResult.data?.let { extractedData.putAll(it) }
                        logs.add("✅ 步骤 $stepNum 成功")
                        debugInterface.onStepComplete(stepNum, true)
                    } else {
                        lastError = stepResult.error
                        retries++
                        if (retries <= step.maxRetries) {
                            val attemptDuration = System.currentTimeMillis() - attemptStartTime
                            log("⚠️ 步骤失败，重试 $retries/${step.maxRetries}: ${stepResult.error}")
                            
                            // 🛡️ 重试前再清理一次弹窗
                            val retryDismiss = popupDismisser.dismissAllPopups(maxAttempts = 2)
                            if (retryDismiss.popupsCleared > 0) {
                                popupsDismissed += retryDismiss.popupsCleared
                                log("🛡️ 重试前清理了 ${retryDismiss.popupsCleared} 个弹窗")
                            }
                            
                            // 📊 记录详细的重试信息
                            debugInterface.onStepRetryDetail(
                                index = stepNum,
                                attemptNumber = retries,
                                reason = stepResult.error ?: "未知原因",
                                durationMs = attemptDuration,
                                popupsDismissed = retryDismiss.popupsCleared
                            )
                            
                            delay(1000)
                        }
                    }
                } catch (e: Exception) {
                    lastError = e.message
                    retries++
                    val attemptDuration = System.currentTimeMillis() - attemptStartTime
                    logs.add("❌ 步骤 $stepNum 异常: ${e.message}")
                    
                    // 📊 记录异常导致的重试
                    debugInterface.onStepRetryDetail(
                        index = stepNum,
                        attemptNumber = retries,
                        reason = "异常: ${e.message}",
                        durationMs = attemptDuration,
                        popupsDismissed = 0
                    )
                }
            }
            
            // 🤖 智能恢复：步骤失败时尝试 AI 分析
            if (!stepSuccess) {
                log("🤖 步骤失败，尝试 AI 分析恢复...")
                aiInterventions++
                
                // 📸 恢复时：切换到全量模式（AI 需要完整上下文）
                autoSwitchScreenMode("AI分析恢复", ScreenCaptureMode.FULL_DUMP)
                
                val recoveryAttempt = attemptSmartRecovery(step, lastError ?: "未知错误")
                
                // 📊 记录 AI 恢复尝试
                debugInterface.onAiRecoveryAttempt(
                    stepIndex = stepNum,
                    success = recoveryAttempt.recovered,
                    action = recoveryAttempt.action
                )
                
                if (recoveryAttempt.recovered) {
                    log("✅ AI 恢复成功: ${recoveryAttempt.action}")
                    logs.add("🤖 AI 恢复: ${recoveryAttempt.action}")
                    
                    // 📸 恢复后验证：切换到 DIFF 模式（精确检查变化）
                    autoSwitchScreenMode("验证恢复结果", ScreenCaptureMode.DIFF)
                    
                    // 恢复后重试
                    val retryResult = executeStep(step, extractedData)
                    if (retryResult.success) {
                        stepSuccess = true
                        retryResult.data?.let { extractedData.putAll(it) }
                        logs.add("✅ 步骤 $stepNum 恢复后成功")
                        debugInterface.onStepComplete(stepNum, true)
                    }
                }
            }
            
            if (!stepSuccess) {
                val error = "步骤 $stepNum 失败: ${step.description}"
                debugInterface.onStepComplete(stepNum, false, error)
                debugInterface.onTaskComplete(false, error)
                
                // 🆕 通知状态管理器执行失败
                ExecutionStateManager.executionFailed(error)
                
                return ScriptExecutionResult(
                    success = false,
                    stepsExecuted = index,
                    totalSteps = script.steps.size,
                    extractedData = extractedData,
                    error = error,
                    failedStepIndex = index,
                    logs = logs,
                    popupsDismissed = popupsDismissed,
                    aiInterventions = aiInterventions
                )
            }
            
            // 🆕 步骤完成，更新日志
            ExecutionStateManager.addLog("✅ 步骤 $stepNum 完成")
            
            delay(500)
        }
        
        log("✅ 脚本执行完成! (清理弹窗: $popupsDismissed, AI介入: $aiInterventions)")
        debugInterface.onTaskComplete(true)
        
        // 🆕 通知状态管理器执行成功
        ExecutionStateManager.executionSuccess("脚本执行完成")
        
        return ScriptExecutionResult(
            success = true,
            stepsExecuted = script.steps.size,
            totalSteps = script.steps.size,
            extractedData = extractedData,
            logs = logs,
            popupsDismissed = popupsDismissed,
            aiInterventions = aiInterventions
        )
    }
    
    /**
     * 🤖 智能恢复尝试
     */
    private suspend fun attemptSmartRecovery(step: ScriptStep, error: String): SmartRecoveryResult {
        return try {
            // 先尝试清理弹窗
            val dismissResult = popupDismisser.dismissAllPopups(maxAttempts = 3)
            if (dismissResult.popupsCleared > 0) {
                return SmartRecoveryResult(
                    recovered = true,
                    action = "清理了 ${dismissResult.popupsCleared} 个弹窗"
                )
            }
            
            // 弹窗清理无效，调用 AI 分析
            val prompt = """
步骤执行失败，请分析原因并给出简洁的恢复建议。

步骤: ${step.type} - ${step.description}
错误: $error

只返回JSON: {"canRecover": true/false, "action": "恢复操作"}
""".trimIndent()
            
            val response = aiClient.chat(listOf(Message("user", prompt)))
            val jsonStr = extractJson(response)
            val map = gson.fromJson(jsonStr, Map::class.java)
            
            SmartRecoveryResult(
                recovered = map["canRecover"] as? Boolean ?: false,
                action = map["action"] as? String ?: "无法恢复"
            )
        } catch (e: Exception) {
            log("❌ 智能恢复失败: ${e.message}")
            SmartRecoveryResult(recovered = false, action = "恢复失败")
        }
    }
    
    /**
     * 👁️ 监控模式执行（每步 AI 验证）
     * 
     * 与 SMART 模式的区别：
     * - SMART: 只在失败时调用 AI
     * - MONITOR: 每步执行后都让 AI 验证结果是否符合预期
     */
    private suspend fun executeScriptMonitorMode(
        script: Script,
        onProgress: ((Int, Int, String) -> Unit)?
    ): ScriptExecutionResult {
        val logs = mutableListOf<String>()
        val extractedData = mutableMapOf<String, Any>()
        var aiVerifications = 0
        
        log("👁️ 监控模式执行中（每步 AI 验证）...")
        debugInterface.onTaskStart(script.id, script.name, script.goal, script.steps.size)
        
        // 📸 首次执行：切换到全量模式
        autoSwitchScreenMode("监控模式启动", ScreenCaptureMode.FULL_DUMP)
        
        for ((index, step) in script.steps.withIndex()) {
            val stepNum = index + 1
            log("📍 步骤 $stepNum/${script.steps.size}: ${step.description}")
            onProgress?.invoke(stepNum, script.steps.size, step.description)
            debugInterface.onStepStart(stepNum, step.type.name, step.description)
            
            // 🛡️ 清理弹窗
            popupDismisser.dismissAllPopups(maxAttempts = 3, delayMs = 300)
            
            // 📸 执行前拍摄快照（用于 AI 验证）
            autoSwitchScreenMode("执行前快照", ScreenCaptureMode.DIFF)
            val smartReader = AgentService.getInstance()?.smartScreenReader
            smartReader?.takeBaselineSnapshot()
            
            // 执行步骤
            val stepResult = executeStep(step, extractedData)
            
            // 👁️ 每步执行后 AI 验证
            aiVerifications++
            val verifyResult = verifyStepWithAI(step, stepResult)
            
            if (verifyResult.verified) {
                log("✅ AI 验证通过: ${verifyResult.reason}")
                stepResult.data?.let { extractedData.putAll(it) }
                logs.add("✅ 步骤 $stepNum 成功 (AI验证: ${verifyResult.confidence}%)")
                debugInterface.onStepComplete(stepNum, true)
                consecutiveSuccesses++
                consecutiveFailures = 0
            } else {
                log("⚠️ AI 验证未通过: ${verifyResult.reason}")
                consecutiveFailures++
                consecutiveSuccesses = 0
                
                // 尝试 AI 恢复
                autoSwitchScreenMode("AI恢复分析", ScreenCaptureMode.FULL_DUMP)
                val recoveryAttempt = attemptSmartRecovery(step, verifyResult.reason)
                if (recoveryAttempt.recovered) {
                    log("✅ AI 恢复成功: ${recoveryAttempt.action}")
                    val retryResult = executeStep(step, extractedData)
                    if (retryResult.success) {
                        retryResult.data?.let { extractedData.putAll(it) }
                        debugInterface.onStepComplete(stepNum, true)
                    }
                } else {
                    // 检查是否需要升级到 AGENT 模式
                    if (shouldUpgradeToAgentMode()) {
                        log("🔄 连续失败，自动升级到 AGENT 模式...")
                        return executeScriptAgentMode(script, onProgress)
                    }
                    
                    val error = "步骤 $stepNum AI验证失败: ${verifyResult.reason}"
                    debugInterface.onStepComplete(stepNum, false, error)
                    debugInterface.onTaskComplete(false, error)
                    return ScriptExecutionResult(
                        success = false,
                        stepsExecuted = index,
                        totalSteps = script.steps.size,
                        error = error,
                        logs = logs
                    )
                }
            }
            
            delay(800) // 监控模式稍慢一点
        }
        
        log("🏁 监控模式执行完成，AI验证 $aiVerifications 次")
        debugInterface.onTaskComplete(true)
        return ScriptExecutionResult(
            success = true,
            stepsExecuted = script.steps.size,
            totalSteps = script.steps.size,
            extractedData = extractedData,
            logs = logs
        )
    }
    
    /**
     * 🤖 代理模式执行（AI 全程控制）
     * 
     * 与 MONITOR 模式的区别：
     * - MONITOR: AI 验证脚本步骤是否正确
     * - AGENT: AI 自主决定下一步做什么，脚本只是参考
     */
    private suspend fun executeScriptAgentMode(
        script: Script,
        onProgress: ((Int, Int, String) -> Unit)?
    ): ScriptExecutionResult {
        val logs = mutableListOf<String>()
        val extractedData = mutableMapOf<String, Any>()
        var aiDecisions = 0
        var maxDecisions = script.steps.size * 3 // 防止无限循环
        
        log("🤖 代理模式执行中（AI 全程决策）...")
        debugInterface.onTaskStart(script.id, script.name, script.goal, script.steps.size)
        
        // 📸 全程使用全量模式（AI 需要完整上下文）
        autoSwitchScreenMode("代理模式启动", ScreenCaptureMode.FULL_DUMP)
        
        // AI 代理循环：持续决策直到目标完成或达到上限
        var goalAchieved = false
        var currentStepIndex = 0
        
        while (!goalAchieved && aiDecisions < maxDecisions) {
            aiDecisions++
            
            // 获取当前屏幕状态
            val screenState = getScreenStateForAI()
            
            // 让 AI 决定下一步
            val aiDecision = askAIForNextAction(
                goal = script.goal,
                currentScreen = screenState,
                executedSteps = currentStepIndex,
                scriptSteps = script.steps.map { it.description }
            )
            
            log("🤖 AI 决策 #$aiDecisions: ${aiDecision.action}")
            onProgress?.invoke(currentStepIndex + 1, script.steps.size, aiDecision.action)
            
            when (aiDecision.type) {
                ScriptAIDecisionType.EXECUTE_STEP -> {
                    // 执行脚本中的某个步骤
                    val stepIndex = aiDecision.stepIndex ?: currentStepIndex
                    if (stepIndex < script.steps.size) {
                        val step = script.steps[stepIndex]
                        val result = executeStep(step, extractedData)
                        if (result.success) {
                            result.data?.let { extractedData.putAll(it) }
                            logs.add("✅ AI执行步骤: ${step.description}")
                            currentStepIndex = stepIndex + 1
                        } else {
                            logs.add("⚠️ AI执行失败: ${result.error}")
                        }
                    }
                }
                ScriptAIDecisionType.CUSTOM_ACTION -> {
                    // AI 自定义操作（不在脚本中）
                    val customResult = executeCustomAIAction(aiDecision)
                    logs.add("🤖 AI自定义操作: ${aiDecision.action}")
                    if (!customResult) {
                        val error = "AI自定义操作未执行: ${aiDecision.action}"
                        log("❌ $error")
                        return ScriptExecutionResult(
                            success = false,
                            stepsExecuted = currentStepIndex,
                            totalSteps = script.steps.size,
                            error = error,
                            logs = logs
                        )
                    }
                }
                ScriptAIDecisionType.WAIT -> {
                    // AI 决定等待
                    log("⏳ AI 决定等待 ${aiDecision.waitMs}ms")
                    delay(aiDecision.waitMs ?: 1000)
                }
                ScriptAIDecisionType.GOAL_ACHIEVED -> {
                    // AI 判断目标已完成
                    goalAchieved = true
                    log("🎯 AI 判断目标已达成: ${aiDecision.reason}")
                }
                ScriptAIDecisionType.GOAL_IMPOSSIBLE -> {
                    // AI 判断目标无法完成
                    val error = "AI判断目标无法完成: ${aiDecision.reason}"
                    log("❌ $error")
                    debugInterface.onTaskComplete(false, error)
                    return ScriptExecutionResult(
                        success = false,
                        stepsExecuted = currentStepIndex,
                        totalSteps = script.steps.size,
                        error = error,
                        logs = logs
                    )
                }
            }
            
            delay(1000) // 代理模式较慢，给 AI 更多思考时间
        }
        
        if (!goalAchieved) {
            val error = "AI 决策次数达到上限 ($maxDecisions)，目标未完成"
            log("⚠️ $error")
            return ScriptExecutionResult(
                success = false,
                stepsExecuted = currentStepIndex,
                totalSteps = script.steps.size,
                error = error,
                logs = logs
            )
        }
        
        log("🏁 代理模式执行完成，AI决策 $aiDecisions 次")
        debugInterface.onTaskComplete(true)
        return ScriptExecutionResult(
            success = true,
            stepsExecuted = script.steps.size,
            totalSteps = script.steps.size,
            extractedData = extractedData,
            logs = logs
        )
    }
    
    /**
     * 🔄 执行并自动改进脚本
     */
    suspend fun executeWithAutoImprove(
        scriptId: String,
        onProgress: ((Int, Int, String) -> Unit)? = null
    ): ScriptExecutionResult = withContext(Dispatchers.IO) {
        var script: Script = loadScript(scriptId) ?: return@withContext ScriptExecutionResult(
            success = false,
            stepsExecuted = 0,
            totalSteps = 0,
            error = "Script not found: $scriptId"
        )
        
        var attempts = 0
        var result: ScriptExecutionResult
        
        do {
            log("🔄 执行尝试 ${attempts + 1}/$MAX_IMPROVE_ATTEMPTS")
            result = executeScriptWithMode(script, executionMode, onProgress)
            
            if (result.success) {
                // 更新成功计数
                script = script.copy(successCount = script.successCount + 1)
                saveScript(script)
                break
            }
            
            // 执行失败，尝试改进脚本
            attempts++
            if (attempts < MAX_IMPROVE_ATTEMPTS) {
                log("⚠️ 执行失败，尝试 AI 改进脚本...")
                val improvedScript = improveScript(script, result)
                if (improvedScript != null) {
                    script = improvedScript
                    saveScript(script)
                    log("✨ 脚本已改进到版本 ${script.version}")
                } else {
                    log("❌ AI 无法改进脚本")
                    break
                }
            }
        } while (attempts < MAX_IMPROVE_ATTEMPTS)
        
        result
    }
    
    /**
     * 🔧 AI 改进脚本
     */
    suspend fun improveScript(script: Script, failResult: ScriptExecutionResult): Script? {
        return try {
            log("🔧 AI 正在分析失败原因并改进脚本...")
            debugInterface.onScriptImproving("执行失败，尝试优化: ${failResult.error}")
            
            val prompt = buildImprovementPrompt(script, failResult)
            val messages = listOf(Message(role = "user", content = prompt))
            val response = aiClient.chat(messages)
            
            val improvedSteps = parseImprovedSteps(response)
            if (improvedSteps != null) {
                val newVersion = incrementVersion(script.version)
                script.copy(
                    version = newVersion,
                    steps = improvedSteps,
                    failCount = script.failCount + 1
                )
            } else {
                null
            }
        } catch (e: Exception) {
            log("❌ 脚本改进失败: ${e.message}")
            null
        }
    }
    
    /**
     * 内部执行逻辑
     */
    private suspend fun executeScriptInternal(
        script: Script,
        onProgress: ((Int, Int, String) -> Unit)?
    ): ScriptExecutionResult {
        val logs = mutableListOf<String>()
        val extractedData = mutableMapOf<String, Any>()
        
        log("▶️ 开始执行脚本: ${script.name}")
        debugInterface.onTaskStart(script.id, script.name, script.goal, script.steps.size)
        
        // 🆕 通知状态管理器开始执行
        ExecutionStateManager.startExecution(script.goal, script.steps.size)
        
        for ((index, step) in script.steps.withIndex()) {
            // 🆕 每步执行前检查取消令牌
            if (ExecutionStateManager.shouldCancel()) {
                log("⏹️ 用户请求停止，终止执行")
                ExecutionStateManager.executionStopped()
                debugInterface.onTaskComplete(false, "用户停止")
                return ScriptExecutionResult(
                    success = false,
                    stepsExecuted = index,
                    totalSteps = script.steps.size,
                    extractedData = extractedData,
                    error = "用户停止执行",
                    cancelled = true,
                    logs = logs
                )
            }
            
            val stepNum = index + 1
            log("📍 步骤 $stepNum/${script.steps.size}: ${step.description}")
            onProgress?.invoke(stepNum, script.steps.size, step.description)
            debugInterface.onStepStart(stepNum, step.type.name, step.description)
            
            // 🆕 更新状态管理器
            ExecutionStateManager.updateStep(stepNum, step.description)
            
            var retries = 0
            var stepSuccess = false
            
            while (retries <= step.maxRetries && !stepSuccess) {
                // 🆕 重试循环中也检查取消
                if (ExecutionStateManager.shouldCancel()) {
                    log("⏹️ 用户请求停止，终止执行")
                    ExecutionStateManager.executionStopped()
                    return ScriptExecutionResult(
                        success = false,
                        stepsExecuted = index,
                        totalSteps = script.steps.size,
                        extractedData = extractedData,
                        error = "用户停止执行",
                        cancelled = true,
                        logs = logs
                    )
                }
                
                try {
                    val stepResult = executeStep(step, extractedData)
                    if (stepResult.success) {
                        stepSuccess = true
                        stepResult.data?.let { extractedData.putAll(it) }
                        logs.add("✅ 步骤 $stepNum 成功")
                        debugInterface.onStepComplete(stepNum, true)
                        ExecutionStateManager.addLog("✅ 步骤 $stepNum 完成")
                    } else {
                        retries++
                        if (retries <= step.maxRetries) {
                            log("⚠️ 步骤失败，重试 $retries/${step.maxRetries}")
                            debugInterface.onStepRetry(stepNum, retries, stepResult.error ?: "未知原因")
                            ExecutionStateManager.addLog("⚠️ 重试 $retries/${step.maxRetries}")
                            delay(1000)
                        }
                    }
                } catch (e: Exception) {
                    retries++
                    logs.add("❌ 步骤 $stepNum 异常: ${e.message}")
                    debugInterface.recordError("STEP_EXCEPTION", "步骤 $stepNum 异常", e, 
                        mapOf("step" to stepNum, "type" to step.type.name))
                    ExecutionStateManager.addLog("❌ 异常: ${e.message}")
                }
            }
            
            if (!stepSuccess) {
                val error = "步骤 $stepNum 失败: ${step.description}"
                debugInterface.onStepComplete(stepNum, false, error)
                debugInterface.onTaskComplete(false, error)
                debugInterface.recordError("STEP_FAILED", error, context = mapOf(
                    "step_index" to index,
                    "step_type" to step.type.name,
                    "step_description" to step.description,
                    "retries" to retries
                ))
                
                // 🆕 通知状态管理器执行失败
                ExecutionStateManager.executionFailed(error)
                
                return ScriptExecutionResult(
                    success = false,
                    stepsExecuted = index,
                    totalSteps = script.steps.size,
                    extractedData = extractedData,
                    error = error,
                    failedStepIndex = index,
                    logs = logs
                )
            }
            
            // 步骤间延迟
            delay(500)
        }
        
        log("✅ 脚本执行完成!")
        debugInterface.onTaskComplete(true)
        
        // 🆕 通知状态管理器执行成功
        ExecutionStateManager.executionSuccess("脚本执行完成")
        
        return ScriptExecutionResult(
            success = true,
            stepsExecuted = script.steps.size,
            totalSteps = script.steps.size,
            extractedData = extractedData,
            logs = logs
        )
    }
    
    /**
     * 执行单个步骤
     */
}

// ==================== 辅助数据类 ====================

/**
 * 步骤执行结果
 */
data class StepResult(
    val success: Boolean,
    val error: String? = null,
    val data: Map<String, Any>? = null
)

/**
 * AI 验证结果
 */
data class AIVerifyResult(
    val verified: Boolean,
    val confidence: Int,
    val reason: String
)

/**
 * 脚本执行 AI 决策类型
 */
enum class ScriptAIDecisionType {
    EXECUTE_STEP,    // 执行脚本步骤
    CUSTOM_ACTION,   // 自定义操作
    WAIT,            // 等待
    GOAL_ACHIEVED,   // 目标完成
    GOAL_IMPOSSIBLE  // 目标无法完成
}

/**
 * 脚本执行 AI 决策
 */
data class ScriptAIDecision(
    val type: ScriptAIDecisionType,
    val action: String,
    val stepIndex: Int? = null,
    val waitMs: Long? = null,
    val reason: String = ""
)