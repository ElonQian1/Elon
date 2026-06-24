// application/AgentRuntime.kt
package com.elon.app.agent.application

import com.elon.app.agent.domain.agent.*
import com.elon.app.agent.domain.screen.UINode
import com.elon.app.agent.domain.tool.Tool
import com.elon.app.agent.domain.tool.ToolRegistry
import com.elon.app.agent.infrastructure.network.FailureReportService
import kotlinx.coroutines.*
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject

/**
 * Agent 运行时
 * 
 * 职责：
 * - 管理 Agent 状态
 * - 协调 AI、工具、屏幕读取
 * - 执行主循环
 * - 失败时自动上报到云端（自我学习系统）
 */
class AgentRuntime(
    private val aiClient: AIClient,
    private val toolRegistry: ToolRegistry,
    private val screenReader: ScreenReader,
    private val mode: AgentMode = AgentMode.SEMI_AUTONOMOUS,
    private val appVersion: String = "1.0.0",
    private val deviceId: String? = null
) {
    private var state = AgentRunState.IDLE
    private val memory = AgentMemory()
    private var currentGoal: Goal? = null
    private var stepCount = 0
    private var errorCount = 0
    
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private var pendingDecision: AIDecision? = null
    private var lastErrorMessage: String? = null
    
    /**
     * 开始执行目标
     */
    suspend fun executeGoal(goal: Goal): AgentExecutionResult {
        currentGoal = goal
        state = AgentRunState.THINKING
        stepCount = 0
        errorCount = 0
        pendingDecision = null
        lastErrorMessage = null
        var success = false
        var message = "目标执行已停止"
        
        memory.workingMemory = WorkingMemory(goal)
        memory.addShortTerm(MemoryEntry(
            timestamp = System.currentTimeMillis(),
            type = MemoryType.THOUGHT,
            content = "开始执行目标: ${goal.description}"
        ))
        
        val startTime = System.currentTimeMillis()
        
        while (state != AgentRunState.STOPPED) {
            // 超时检查
            if (System.currentTimeMillis() - startTime > goal.timeoutSeconds * 1000) {
                Log.w("AgentRuntime", "目标执行超时")
                message = "目标执行超时（${goal.timeoutSeconds} 秒）"
                lastErrorMessage = message
                state = AgentRunState.STOPPED
                break
            }
            
            // 步数检查
            if (stepCount >= goal.maxSteps) {
                Log.w("AgentRuntime", "达到最大步数限制")
                message = "达到最大步数限制（${goal.maxSteps} 步）"
                lastErrorMessage = message
                state = AgentRunState.STOPPED
                break
            }
            
            when (state) {
                AgentRunState.THINKING -> {
                    val completed = handleThinking()
                    if (completed) {
                        success = true
                        message = "AI 判断目标已完成"
                    }
                }
                AgentRunState.EXECUTING -> handleExecuting()
                AgentRunState.OBSERVING -> {
                    val completed = handleObserving()
                    if (completed) {
                        success = true
                        message = "目标完成条件满足"
                    }
                }
                AgentRunState.PAUSED -> delay(100)
                AgentRunState.WAITING_FOR_APPROVAL -> delay(100)
                AgentRunState.RECOVERING -> handleRecovering()
                else -> break
            }
            
            stepCount++
        }
        
        if (!success && errorCount >= 3) {
            message = "连续错误过多，已停止执行"
        }
        Log.i("AgentRuntime", "目标执行完成，共 $stepCount 步，success=$success")
        return AgentExecutionResult(
            success = success,
            stepsExecuted = stepCount,
            message = if (success) message else lastErrorMessage ?: message,
            finalState = state,
            errorCount = errorCount
        )
    }
    
    /**
     * 思考阶段：调用 AI 决定下一步动作
     */
    private suspend fun handleThinking(): Boolean {
        try {
            memory.addShortTerm(MemoryEntry(
                timestamp = System.currentTimeMillis(),
                type = MemoryType.THOUGHT,
                content = "正在调用 AI 分析..."
            ))
            
            // 构建上下文
            val context = buildContext()
            
            // 调用 AI
            val response = aiClient.chat(context)
            
            // 解析 AI 响应（期望 JSON 格式）
            val decision = parseAIResponse(response)
            
            if (decision.isComplete) {
                Log.i("AgentRuntime", "AI 判断目标已完成")
                state = AgentRunState.STOPPED
                return true
            } else if (decision.action != null) {
                pendingDecision = decision
                memory.addShortTerm(MemoryEntry(
                    timestamp = System.currentTimeMillis(),
                    type = MemoryType.THOUGHT,
                    content = "AI 决定: ${decision.thought} | 动作: ${decision.action.name}"
                ))
                state = AgentRunState.EXECUTING
            } else {
                handleExecutionFailure(lastErrorMessage ?: "AI 未给出可执行动作")
            }
            
        } catch (e: Exception) {
            Log.e("AgentRuntime", "思考阶段出错", e)
            errorCount++
            lastErrorMessage = "AI 调用失败: ${e.message}"
            
            // 上报 AI 错误
            reportFailureAsync(
                errorType = FailureReportService.ErrorTypes.AI_ERROR,
                reason = lastErrorMessage ?: "AI 调用失败",
                exception = e
            )
            
            if (errorCount >= 3) {
                state = AgentRunState.STOPPED
            } else {
                state = AgentRunState.RECOVERING
            }
        }
        return false
    }
    
    /**
     * 执行阶段：调用工具执行动作
     */
    private suspend fun handleExecuting() {
        val decision = pendingDecision
        pendingDecision = null
        val tool = decision?.action
        if (decision == null || tool == null) {
            handleExecutionFailure("没有待执行动作")
            return
        }

        memory.addShortTerm(MemoryEntry(
            timestamp = System.currentTimeMillis(),
            type = MemoryType.ACTION,
            content = "执行工具: ${tool.name} params=${decision.params}"
        ))

        try {
            val result = tool.execute(decision.params)
            memory.addShortTerm(MemoryEntry(
                timestamp = System.currentTimeMillis(),
                type = MemoryType.RESULT,
                content = "工具 ${tool.name} 执行结果: success=${result.success}, message=${result.message}"
            ))
            if (result.success) {
                lastErrorMessage = null
                state = AgentRunState.OBSERVING
            } else {
                handleExecutionFailure("工具 ${tool.name} 执行失败: ${result.message}")
            }
        } catch (e: Exception) {
            Log.e("AgentRuntime", "工具执行异常: ${tool.name}", e)
            handleExecutionFailure("工具 ${tool.name} 执行异常: ${e.message}", e)
        }
    }

    private fun handleExecutionFailure(reason: String, exception: Throwable? = null) {
        errorCount++
        lastErrorMessage = reason
        memory.addShortTerm(MemoryEntry(
            timestamp = System.currentTimeMillis(),
            type = MemoryType.RESULT,
            content = reason
        ))
        reportFailureAsync(
            errorType = FailureReportService.ErrorTypes.TOOL_ERROR,
            reason = reason,
            exception = exception
        )
        state = if (errorCount >= 3) AgentRunState.STOPPED else AgentRunState.RECOVERING
    }
    
    /**
     * 观察阶段：获取屏幕状态
     */
    private suspend fun handleObserving(): Boolean {
        try {
            val screen = screenReader.readCurrentScreen()
            val summary = screen.getClickableElementsSummary()
            
            memory.addShortTerm(MemoryEntry(
                timestamp = System.currentTimeMillis(),
                type = MemoryType.OBSERVATION,
                content = "当前屏幕：\n$summary"
            ))
            
            // 检查目标完成条件
            val isGoalComplete = checkGoalCompletion(screen)
            if (isGoalComplete) {
                Log.i("AgentRuntime", "目标完成条件满足")
                state = AgentRunState.STOPPED
                return true
            } else {
                state = AgentRunState.THINKING
            }
            
        } catch (e: Exception) {
            Log.e("AgentRuntime", "观察阶段出错", e)
            errorCount++
            lastErrorMessage = "观察阶段出错: ${e.message}"
            state = AgentRunState.RECOVERING
        }
        return false
    }
    
    /**
     * 恢复阶段：从错误中恢复
     */
    private suspend fun handleRecovering() {
        delay(1000)
        if (errorCount < 3) {
            state = AgentRunState.THINKING
        } else {
            // 达到最大重试次数，上报失败并停止
            reportFailureAsync(
                errorType = FailureReportService.ErrorTypes.UNKNOWN,
                reason = "达到最大错误重试次数 ($errorCount)",
                exception = null
            )
            state = AgentRunState.STOPPED
        }
    }
    
    /**
     * 异步上报失败（不阻塞主流程）
     */
    private fun reportFailureAsync(
        errorType: String,
        reason: String,
        exception: Throwable? = null,
        screenXml: String? = null
    ) {
        val goal = currentGoal ?: return
        
        scope.launch {
            try {
                val result = if (exception != null) {
                    FailureReportService.reportCrash(
                        taskGoal = goal.description,
                        exception = exception,
                        step = stepCount,
                        targetApp = goal.targetApp,
                        appVersion = appVersion,
                        screenXml = screenXml,
                        deviceId = deviceId
                    )
                } else {
                    FailureReportService.reportExecutionFailure(
                        taskGoal = goal.description,
                        generatedScript = null,
                        failureStep = stepCount,
                        stepDescription = reason,
                        errorType = errorType,
                        screenXml = screenXml,
                        targetApp = goal.targetApp,
                        appVersion = appVersion,
                        deviceId = deviceId
                    )
                }
                
                result.onSuccess { failureId ->
                    Log.i("AgentRuntime", "失败案例已上报: #$failureId")
                }.onFailure { e ->
                    Log.w("AgentRuntime", "失败上报失败（非致命）", e)
                }
            } catch (e: Exception) {
                Log.w("AgentRuntime", "失败上报异常（非致命）", e)
            }
        }
    }
    
    /**
     * 构建 AI 上下文
     */
    private fun buildContext(): List<Message> {
        val messages = mutableListOf<Message>()
        
        // 系统提示词
        val systemPrompt = buildSystemPrompt()
        messages.add(Message(role = "system", content = systemPrompt))
        
        // 历史对话
        memory.shortTerm.forEach { entry ->
            val role = when (entry.type) {
                MemoryType.THOUGHT -> "assistant"
                MemoryType.OBSERVATION, MemoryType.RESULT -> "system"
                else -> "user"
            }
            messages.add(Message(role = role, content = entry.content))
        }
        
        return messages
    }
    
    private fun buildSystemPrompt(): String {
        val goal = currentGoal?.description ?: "未知目标"
        val tools = toolRegistry.getToolDescriptions()
        
        return """
            你是手机 AI Agent，当前目标：$goal
            
            ## 可用工具
            $tools
            
            ## 响应格式（JSON）
            {
              "thought": "你的思考过程",
              "action": "工具名称",
              "params": { 工具参数 },
              "is_complete": false
            }
            
            目标完成时设置 "is_complete": true
        """.trimIndent()
    }
    
    private fun parseAIResponse(response: String): AIDecision {
        return try {
            val json = JSONObject(extractJsonObject(response))
            val thought = json.optString("thought", "AI 未提供思考说明")
            val isComplete = json.optBoolean("is_complete", json.optBoolean("isComplete", false))
            val actionName = extractActionName(json)
            val params = json.optJSONObject("params")?.toMap()
                ?: json.optJSONObject("action")?.optJSONObject("params")?.toMap()
                ?: emptyMap()
            val tool = actionName?.let { toolRegistry.get(it) }
            if (!isComplete && actionName != null && tool == null) {
                lastErrorMessage = "AI 请求了未注册工具: $actionName"
            }
            AIDecision(
                thought = thought,
                action = tool,
                params = params,
                isComplete = isComplete
            )
        } catch (e: Exception) {
            Log.e("AgentRuntime", "解析 AI 响应失败: $response", e)
            lastErrorMessage = "解析 AI 响应失败: ${e.message}"
            AIDecision(
                thought = "解析 AI 响应失败",
                action = null,
                isComplete = false
            )
        }
    }

    private fun extractJsonObject(response: String): String {
        val cleaned = response
            .replace("```json", "")
            .replace("```", "")
            .trim()
        val start = cleaned.indexOf('{')
        val end = cleaned.lastIndexOf('}')
        require(start >= 0 && end >= start) { "AI 响应中没有 JSON 对象" }
        return cleaned.substring(start, end + 1)
    }

    private fun extractActionName(json: JSONObject): String? {
        val rawAction = json.opt("action") ?: return null
        return when (rawAction) {
            is String -> rawAction.trim().takeIf { it.isNotEmpty() }
            is JSONObject -> {
                rawAction.optString("name").takeIf { it.isNotBlank() }
                    ?: rawAction.optString("tool").takeIf { it.isNotBlank() }
                    ?: rawAction.optString("type").takeIf { it.isNotBlank() }
            }
            else -> null
        }
    }

    private fun JSONObject.toMap(): Map<String, Any> {
        val result = mutableMapOf<String, Any>()
        val keys = keys()
        while (keys.hasNext()) {
            val key = keys.next()
            result[key] = normalizeJsonValue(get(key))
        }
        return result
    }

    private fun normalizeJsonValue(value: Any?): Any {
        return when (value) {
            null, JSONObject.NULL -> ""
            is JSONObject -> value.toMap()
            is JSONArray -> (0 until value.length()).map { index -> normalizeJsonValue(value.get(index)) }
            else -> value
        }
    }
    
    private fun checkGoalCompletion(screen: UINode): Boolean {
        val condition = currentGoal?.completionCondition ?: return false
        return when (condition) {
            is CompletionCondition.ElementAppears -> screen.containsVisibleToken(condition.text)
            is CompletionCondition.ElementDisappears -> !screen.containsVisibleToken(condition.text)
            is CompletionCondition.ReachPage -> {
                val tokens = screen.collectVisibleTokens()
                tokens.any { it.contains(condition.packageName, ignoreCase = true) } &&
                    tokens.any { it.contains(condition.activityName, ignoreCase = true) }
            }
            is CompletionCondition.Custom -> condition.predicate(
                ScreenSnapshot(
                    timestamp = System.currentTimeMillis(),
                    packageName = null,
                    activityName = null,
                    elements = screen.collectVisibleTokens()
                )
            )
            CompletionCondition.AIDecided -> false
        }
    }

    private fun UINode.containsVisibleToken(expected: String): Boolean {
        if (expected.isBlank()) return false
        return collectVisibleTokens().any { token ->
            token.contains(expected, ignoreCase = true)
        }
    }

    private fun UINode.collectVisibleTokens(): List<String> {
        val tokens = mutableListOf<String>()
        fun visit(node: UINode) {
            listOf(node.text, node.contentDescription, node.resourceId, node.className)
                .filterNotNull()
                .filter { it.isNotBlank() }
                .forEach { tokens.add(it) }
            node.children.forEach(::visit)
        }
        visit(this)
        return tokens
    }
    
    fun getSnapshot(): AgentSnapshot {
        return AgentSnapshot(
            state = state,
            mode = mode,
            currentGoal = currentGoal,
            progress = stepCount,
            lastAction = memory.shortTerm.lastOrNull()?.content,
            errorCount = errorCount
        )
    }
    
    fun pause() { state = AgentRunState.PAUSED }
    fun resume() { state = AgentRunState.THINKING }
    fun stop() { state = AgentRunState.STOPPED }
}

data class AgentExecutionResult(
    val success: Boolean,
    val stepsExecuted: Int,
    val message: String,
    val finalState: AgentRunState,
    val errorCount: Int
)

/**
 * AI 客户端接口
 */
interface AIClient {
    suspend fun chat(messages: List<Message>): String
}

data class Message(val role: String, val content: String)

/**
 * 屏幕读取器接口
 */
interface ScreenReader {
    suspend fun readCurrentScreen(): UINode
}

/**
 * AI 决策结果
 */
data class AIDecision(
    val thought: String,
    val action: Tool? = null,
    val params: Map<String, Any> = emptyMap(),
    val isComplete: Boolean = false
)
