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
    private val service: AccessibilityService
) {
    companion object {
        private const val TAG = "ScriptEngine"
        private const val SCRIPTS_DIR = "scripts"
        private const val MAX_IMPROVE_ATTEMPTS = 3
    }
    
    private val gson: Gson = GsonBuilder().setPrettyPrinting().create()
    private val aiClient = AIClientFactory.create(service)
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    
    // 🔧 调试接口
    private val debugInterface = DebugInterface.getInstance()
    
    // 🛡️ 弹窗清理器
    private val popupDismisser = PopupDismisser(service)
    
    /**
     * 🆕 获取 Root Window 的辅助函数
     * 
     * 先尝试 rootInActiveWindow，如果为 null 则从 windows 中获取活动窗口的 root
     */
    private fun getRootNode(): AccessibilityNodeInfo? {
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
    private var consecutiveFailures = 0
    private var consecutiveSuccesses = 0
    private var totalAiInterventions = 0
    
    // 脚本缓存
    private val scriptsCache = mutableMapOf<String, Script>()
    
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
    private suspend fun executeStep(
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
    private suspend fun executeSearch(step: ScriptStep): StepResult {
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
                // TODO: 输入文本
            }
            return StepResult(true, "Search initiated")
        }
        
        // 如果有contains，当作FIND_AND_TAP处理
        if (contains != null) {
            return executeFindAndTap(step)
        }
        
        return StepResult(false, "Missing search parameters")
    }
    
    private suspend fun executeLaunchApp(step: ScriptStep): StepResult {
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
    private suspend fun ensureXhsHomePage() {
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
    
    private suspend fun executeTap(step: ScriptStep): StepResult {
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
    
    private suspend fun executeSwipe(step: ScriptStep): StepResult {
        val direction = step.params["direction"] as? String ?: "up"
        return performSwipe(direction)
    }
    
    private suspend fun executeWait(step: ScriptStep): StepResult {
        val ms = (step.params["ms"] as? Number)?.toLong() ?: 1000
        delay(ms)
        return StepResult(true)
    }
    
    private suspend fun executeFindAndTap(step: ScriptStep): StepResult {
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
    
    private suspend fun executeScrollUntilFind(step: ScriptStep): StepResult {
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
    
    private fun validatePageAfterTap(): PageValidation {
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
    private fun findMatchingNodeWithExcludes(
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
    private fun collectAllTexts(node: android.view.accessibility.AccessibilityNodeInfo, results: MutableList<String>, maxCount: Int) {
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
    
    private suspend fun executeExtractData(step: ScriptStep): StepResult {
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
    private fun extractComments(
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
    
    private fun collectAllTextWithBounds(
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
    private fun extractLikes(
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
    
    private fun extractAllTexts(
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
    
    private suspend fun executeInputText(step: ScriptStep): StepResult {
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
    private fun findFirstEditableNode(node: android.view.accessibility.AccessibilityNodeInfo): android.view.accessibility.AccessibilityNodeInfo? {
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
    
    private suspend fun executeBack(step: ScriptStep): StepResult {
        service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
        delay(500)
        return StepResult(true)
    }
    
    private suspend fun executeAssert(step: ScriptStep): StepResult {
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
    
    private suspend fun executeAIDecide(step: ScriptStep): StepResult {
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

    private suspend fun executeStructuredAIDecision(response: String): StepResult {
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
    
    private fun performTap(x: Int, y: Int): StepResult {
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
    
    private fun performSwipe(direction: String): StepResult {
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
    
    private fun findAndTapByText(text: String): StepResult {
        val root = getRootNode() ?: return StepResult(false, "No window")
        val node = findMatchingNode(root, text, null, null)
        
        if (node != null) {
            val rect = android.graphics.Rect()
            node.getBoundsInScreen(rect)
            return performTap(rect.centerX(), rect.centerY())
        }
        
        return StepResult(false, "Text not found: $text")
    }
    
    private fun findMatchingNode(
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
    private fun findMatchingNodeEnhanced(
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
    private fun smartContainsMatch(text: String, searchTerm: String): Boolean {
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
    
    private fun extractTexts(
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
    
    private fun collectElements(node: android.view.accessibility.AccessibilityNodeInfo): String {
        val elements = mutableListOf<String>()
        collectElementsRecursive(node, elements, 20)
        return elements.joinToString("\n")
    }
    
    private fun collectElementsRecursive(
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
    
    // ========== 脚本存储 ==========
    
    fun saveScript(script: Script) {
        scriptsCache[script.id] = script
        
        try {
            val scriptsDir = File(service.filesDir, SCRIPTS_DIR)
            if (!scriptsDir.exists()) scriptsDir.mkdirs()
            
            val file = File(scriptsDir, "${script.id}.json")
            file.writeText(gson.toJson(script))
            log("💾 脚本已保存: ${script.name}")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save script", e)
        }
    }
    
    fun loadScript(scriptId: String): Script? {
        scriptsCache[scriptId]?.let { return it }
        
        try {
            val file = File(service.filesDir, "$SCRIPTS_DIR/$scriptId.json")
            if (file.exists()) {
                val script = gson.fromJson(file.readText(), Script::class.java)
                scriptsCache[scriptId] = script
                return script
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load script", e)
        }
        
        return null
    }
    
    fun listScripts(): List<Script> {
        val scripts = mutableListOf<Script>()
        
        try {
            val scriptsDir = File(service.filesDir, SCRIPTS_DIR)
            if (scriptsDir.exists()) {
                scriptsDir.listFiles()?.forEach { file ->
                    if (file.extension == "json") {
                        try {
                            val script = gson.fromJson(file.readText(), Script::class.java)
                            scripts.add(script)
                        } catch (e: Exception) {
                            Log.e(TAG, "Failed to parse script: ${file.name}", e)
                        }
                    }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to list scripts", e)
        }
        
        return scripts
    }
    
    fun deleteScript(scriptId: String): Boolean {
        scriptsCache.remove(scriptId)
        
        try {
            val file = File(service.filesDir, "$SCRIPTS_DIR/$scriptId.json")
            return file.delete()
        } catch (e: Exception) {
            return false
        }
    }
    
    // ========== AI Prompt 构建 ==========
    
    private fun buildScriptGenerationPrompt(goal: String): String {
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
    
    private fun buildImprovementPrompt(script: Script, failResult: ScriptExecutionResult): String {
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
    
    private fun parseScriptFromAI(response: String, goal: String): Script? {
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
    
    private fun parseImprovedSteps(response: String): List<ScriptStep>? {
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
    private fun mapUnknownStepType(typeStr: String): StepType {
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
    
    private fun extractJson(text: String): String {
        // 尝试提取 JSON
        val jsonPattern = Regex("""\{[\s\S]*\}|\[[\s\S]*\]""")
        return jsonPattern.find(text)?.value ?: text
    }
    
    private fun incrementVersion(version: String): String {
        val parts = version.split(".")
        return if (parts.size >= 2) {
            "${parts[0]}.${(parts[1].toIntOrNull() ?: 0) + 1}"
        } else {
            "1.1"
        }
    }
    
    private fun log(message: String) {
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
    private fun autoSwitchScreenMode(scenario: String, targetMode: ScreenCaptureMode) {
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
    fun getCurrentScreenMode(): ScreenCaptureMode {
        return AgentService.getInstance()?.smartScreenReader?.currentMode 
            ?: ScreenCaptureMode.FULL_DUMP
    }
    
    /**
     * 手动设置屏幕模式（覆盖自动切换）
     */
    fun setScreenMode(mode: ScreenCaptureMode) {
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
    private fun shouldUpgradeToAgentMode(): Boolean {
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
    private fun shouldDowngradeToFastMode(): Boolean {
        if (!autoExecutionModeUpgrade) return false
        if (executionMode == ExecutionMode.FAST) return false // 已经是最低级
        
        return consecutiveSuccesses >= 10 && totalAiInterventions == 0
    }
    
    /**
     * 自动调整执行模式
     */
    private fun autoAdjustExecutionMode() {
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
    fun resetExecutionStats() {
        consecutiveFailures = 0
        consecutiveSuccesses = 0
        totalAiInterventions = 0
    }
    
    // ==================== 👁️ MONITOR 模式辅助方法 ====================
    
    /**
     * AI 验证步骤执行结果
     */
    private suspend fun verifyStepWithAI(step: ScriptStep, result: StepResult): AIVerifyResult {
        try {
            val screenState = getScreenStateForAI()
            
            val prompt = """
你是一个 UI 自动化验证专家。请验证以下步骤是否执行成功。

【步骤信息】
- 类型: ${step.type.name}
- 描述: ${step.description}
- 执行结果: ${if (result.success) "代码层面成功" else "代码层面失败: ${result.error}"}

【当前屏幕状态】
$screenState

【验证要求】
1. 判断步骤是否真正执行成功（不只是代码返回成功）
2. 检查页面是否符合预期状态

请用 JSON 格式返回：
{"verified": true/false, "confidence": 0-100, "reason": "简短原因"}
""".trimIndent()
            
            val response = aiClient.chat(listOf(Message("user", prompt)))
            val json = extractJson(response)
            val map = gson.fromJson<Map<String, Any>>(json, object : TypeToken<Map<String, Any>>() {}.type)
            
            return AIVerifyResult(
                verified = map["verified"] as? Boolean ?: result.success,
                confidence = (map["confidence"] as? Number)?.toInt() ?: 50,
                reason = map["reason"] as? String ?: "AI未返回原因"
            )
        } catch (e: Exception) {
            log("⚠️ AI验证异常: ${e.message}")
            // 验证失败时，信任代码层面的结果
            return AIVerifyResult(
                verified = result.success,
                confidence = 50,
                reason = "AI验证异常，使用代码结果"
            )
        }
    }
    
    // ==================== 🤖 AGENT 模式辅助方法 ====================
    
    /**
     * 获取屏幕状态供 AI 分析
     */
    private fun getScreenStateForAI(): String {
        return try {
            val smartReader = AgentService.getInstance()?.smartScreenReader
            val tree = smartReader?.forceFullDump()
            tree?.toSimpleString() ?: "无法获取屏幕状态"
        } catch (e: Exception) {
            "获取屏幕状态失败: ${e.message}"
        }
    }
    
    /**
     * 询问 AI 下一步操作
     */
    private suspend fun askAIForNextAction(
        goal: String,
        currentScreen: String,
        executedSteps: Int,
        scriptSteps: List<String>
    ): ScriptAIDecision {
        try {
            val prompt = """
你是一个智能手机操作代理。根据当前屏幕状态，决定下一步操作。

【任务目标】
$goal

【脚本参考步骤】
${scriptSteps.mapIndexed { i, s -> "${i + 1}. $s" }.joinToString("\n")}

【已执行步骤数】
$executedSteps / ${scriptSteps.size}

【当前屏幕状态】
$currentScreen

【决策选项】
1. EXECUTE_STEP - 执行脚本中的某个步骤（指定步骤索引）
2. CUSTOM_ACTION - 执行自定义操作（脚本中没有的）
3. WAIT - 等待页面加载
4. GOAL_ACHIEVED - 目标已完成
5. GOAL_IMPOSSIBLE - 目标无法完成

请用 JSON 格式返回：
{
  "type": "EXECUTE_STEP|CUSTOM_ACTION|WAIT|GOAL_ACHIEVED|GOAL_IMPOSSIBLE",
  "action": "具体操作描述",
  "stepIndex": 0-N（如果是EXECUTE_STEP）,
  "waitMs": 毫秒数（如果是WAIT）,
  "reason": "决策原因"
}
""".trimIndent()
            
            val response = aiClient.chat(listOf(Message("user", prompt)))
            val json = extractJson(response)
            val map = gson.fromJson<Map<String, Any>>(json, object : TypeToken<Map<String, Any>>() {}.type)
            
            val typeStr = map["type"] as? String ?: "EXECUTE_STEP"
            return ScriptAIDecision(
                type = ScriptAIDecisionType.valueOf(typeStr),
                action = map["action"] as? String ?: "未知操作",
                stepIndex = (map["stepIndex"] as? Number)?.toInt(),
                waitMs = (map["waitMs"] as? Number)?.toLong(),
                reason = map["reason"] as? String ?: ""
            )
        } catch (e: Exception) {
            log("⚠️ AI决策异常: ${e.message}")
            // 默认继续执行下一步
            return ScriptAIDecision(
                type = ScriptAIDecisionType.EXECUTE_STEP,
                action = "继续执行",
                stepIndex = executedSteps,
                reason = "AI异常，默认继续"
            )
        }
    }
    
    /**
     * 执行 AI 自定义操作
     */
    private suspend fun executeCustomAIAction(decision: ScriptAIDecision): Boolean {
        log("🤖 执行AI自定义操作: ${decision.action}")
        val action = decision.action.lowercase()
        return when {
            action.contains("等待") || action.contains("wait") -> {
                delay((decision.waitMs ?: 1000L).coerceIn(100L, 10_000L))
                true
            }
            action.contains("返回") || action.contains("back") -> {
                service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
                delay(500)
                true
            }
            else -> {
                log("❌ 无法安全解析 AI 自定义操作，已拒绝默认成功: ${decision.action}")
                false
            }
        }
    }

    private fun numberParam(params: Map<*, *>, key: String): Int? {
        return when (val value = params[key]) {
            is Number -> value.toInt()
            is String -> value.toIntOrNull()
            else -> null
        }
    }
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
