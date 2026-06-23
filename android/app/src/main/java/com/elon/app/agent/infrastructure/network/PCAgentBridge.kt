// infrastructure/network/PCAgentBridge.kt
// module: infrastructure/network | layer: infrastructure | role: pc-bridge
// summary: PC-手机协同桥接器，处理双向通信

package com.elon.app.agent.infrastructure.network

import android.util.Log
import com.elon.app.agent.application.AgentRuntime
import com.elon.app.agent.domain.agent.AgentRunState
import com.elon.app.agent.domain.agent.Goal
import com.elon.app.agent.domain.agent.CompletionCondition
import com.elon.app.agent.infrastructure.accessibility.UITreeParser
import com.elon.app.agent.infrastructure.vision.ScreenshotCapture
import com.elon.app.agent.infrastructure.vision.ScreenAnalyzer
import com.elon.app.agent.infrastructure.vision.ScriptGenerator
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

/**
 * PC-手机协同桥接器
 * 
 * 职责：
 * - 接收 PC 端的命令和目标
 * - 转发给 AgentRuntime 执行
 * - 将执行状态和结果上报给 PC 端
 * - 实时同步屏幕状态
 */
class PCAgentBridge(
    private val webSocketServer: WebSocketServer,
    private val agentRuntimeProvider: () -> AgentRuntime?,
    private val uiTreeParser: UITreeParser,
    private val screenshotCapture: ScreenshotCapture?
) {
    companion object {
        private const val TAG = "PCAgentBridge"
        private const val SCREEN_SYNC_INTERVAL = 2000L  // 屏幕同步间隔
        
        /**
         * 转义 JSON 字符串中的特殊字符
         */
        private fun escapeJson(text: String): String {
            return text
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t")
        }
    }
    
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private var screenSyncJob: Job? = null
    private var currentGoalJob: Job? = null
    private var currentGoalId: String? = null
    
    /**
     * 初始化桥接器
     */
    fun initialize() {
        // 注册消息处理器
        registerHandlers()
        
        // 监听服务器事件
        scope.launch {
            webSocketServer.events.collect { event ->
                when (event) {
                    is ServerEvent.ClientConnected -> onClientConnected(event.clientId)
                    is ServerEvent.ClientDisconnected -> onClientDisconnected(event.clientId)
                    else -> {}
                }
            }
        }
        
        Log.i(TAG, "PC Agent 桥接器已初始化")
    }
    
    /**
     * 注册消息处理器
     */
    private fun registerHandlers() {
        // 目标处理
        webSocketServer.registerHandler("goal") { clientId, payload ->
            handleGoal(clientId, payload)
        }
        
        // 命令处理
        webSocketServer.registerHandler("command") { clientId, payload ->
            handleCommand(clientId, payload)
        }
        
        // 查询处理
        webSocketServer.registerHandler("query") { clientId, payload ->
            handleQuery(clientId, payload)
        }
    }
    
    /**
     * 处理目标消息
     */
    private suspend fun handleGoal(clientId: String, payload: String): OutgoingMessage? {
        val goalPayload = AgentProtocol.parseGoalMessage(payload)
        if (goalPayload == null) {
            return AgentProtocol.errorMessage(
                ErrorCode.INVALID_MESSAGE,
                "无效的目标消息"
            )
        }
        
        Log.i(TAG, "收到目标: ${goalPayload.description}")
        
        val runtime = agentRuntimeProvider()
        if (runtime == null) {
            return AgentProtocol.errorMessage(
                ErrorCode.UNKNOWN,
                "Agent 运行时未初始化"
            )
        }
        if (currentGoalJob?.isActive == true) {
            return AgentProtocol.errorMessage(
                ErrorCode.UNKNOWN,
                "已有目标正在执行，请先停止当前目标"
            )
        }
        
        // 创建目标
        val goal = Goal(
            description = goalPayload.description,
            completionCondition = CompletionCondition.AIDecided,
            maxSteps = goalPayload.maxSteps,
            timeoutSeconds = goalPayload.timeoutSeconds.toLong()
        )
        
        currentGoalId = goal.id
        
        // 异步执行目标
        currentGoalJob = scope.launch {
            try {
                val startTime = System.currentTimeMillis()
                
                // 发送开始状态
                webSocketServer.broadcast(AgentProtocol.statusMessage(
                    state = AgentRunState.THINKING,
                    currentGoal = goal.description,
                    progress = 0f
                ))
                
                // 执行目标
                val result = runtime.executeGoal(goal)
                
                val duration = System.currentTimeMillis() - startTime
                
                // 发送完成结果
                webSocketServer.broadcast(AgentProtocol.resultMessage(
                    goalId = goal.id,
                    success = result.success,
                    stepsExecuted = result.stepsExecuted,
                    message = result.message,
                    duration = duration
                ))
                
            } catch (e: Exception) {
                Log.e(TAG, "目标执行失败", e)
                webSocketServer.broadcast(AgentProtocol.errorMessage(
                    ErrorCode.GOAL_FAILED,
                    "目标执行失败: ${e.message}",
                    e.stackTraceToString()
                ))
            } finally {
                currentGoalJob = null
            }
        }
        
        return AgentProtocol.statusMessage(
            state = AgentRunState.THINKING,
            currentGoal = goal.description
        )
    }
    
    /**
     * 处理命令消息
     */
    private suspend fun handleCommand(clientId: String, payload: String): OutgoingMessage? {
        val commandPayload = AgentProtocol.parseCommandMessage(payload)
        if (commandPayload == null) {
            return AgentProtocol.errorMessage(
                ErrorCode.INVALID_MESSAGE,
                "无效的命令消息"
            )
        }
        
        val command = try {
            AgentCommand.valueOf(commandPayload.command.uppercase())
        } catch (e: Exception) {
            return AgentProtocol.errorMessage(
                ErrorCode.INVALID_MESSAGE,
                "未知命令: ${commandPayload.command}"
            )
        }
        
        Log.i(TAG, "收到命令: $command")
        
        return when (command) {
            AgentCommand.GET_STATUS -> getStatus()
            AgentCommand.GET_SCREEN -> getScreen(includeScreenshot = false)
            AgentCommand.SCREENSHOT -> getScreen(includeScreenshot = true)
            AgentCommand.PAUSE -> {
                agentRuntimeProvider()?.pause()
                AgentProtocol.statusMessage(AgentRunState.PAUSED)
            }
            AgentCommand.RESUME -> {
                agentRuntimeProvider()?.resume()
                AgentProtocol.statusMessage(AgentRunState.THINKING)
            }
            AgentCommand.STOP -> {
                agentRuntimeProvider()?.stop()
                currentGoalJob?.cancel()
                currentGoalJob = null
                AgentProtocol.statusMessage(AgentRunState.STOPPED)
            }
            AgentCommand.ANALYZE_SCREEN -> {
                analyzeScreen()
            }
            AgentCommand.GENERATE_SCRIPT -> {
                val goal = commandPayload.params?.get("goal") as? String ?: ""
                generateScript(goal)
            }
            else -> {
                AgentProtocol.errorMessage(
                    ErrorCode.UNKNOWN,
                    "命令暂未实现: $command"
                )
            }
        }
    }
    
    /**
     * 处理查询消息
     */
    private suspend fun handleQuery(clientId: String, payload: String): OutgoingMessage? {
        return getStatus()
    }
    
    /**
     * 获取当前状态
     */
    private fun getStatus(): OutgoingMessage {
        val snapshot = agentRuntimeProvider()?.getSnapshot()
        val goal = snapshot?.currentGoal
        val progress = if (goal != null && goal.maxSteps > 0) {
            (snapshot.progress.toFloat() / goal.maxSteps.toFloat()).coerceIn(0f, 1f)
        } else {
            0f
        }
        return AgentProtocol.statusMessage(
            state = snapshot?.state ?: AgentRunState.IDLE,
            currentGoal = goal?.description,
            progress = progress
        )
    }
    
    /**
     * 获取屏幕状态
     */
    private suspend fun getScreen(includeScreenshot: Boolean): OutgoingMessage {
        return try {
            val uiTree = uiTreeParser.readCurrentScreen()
            
            // 提取信息
            val visibleTexts = mutableListOf<String>()
            val clickableElements = mutableListOf<String>()
            
            fun traverse(node: com.elon.app.agent.domain.screen.UINode?) {
                if (node == null) return
                
                node.text?.let { if (it.isNotBlank()) visibleTexts.add(it) }
                node.contentDescription?.let { if (it.isNotBlank()) visibleTexts.add(it) }
                
                if (node.isClickable) {
                    val text = node.text ?: node.contentDescription ?: node.resourceId ?: ""
                    if (text.isNotBlank()) clickableElements.add(text)
                }
                
                node.children.forEach { traverse(it) }
            }
            
            traverse(uiTree)
            
            // 截图（可选）
            val screenshot = if (includeScreenshot) {
                val result = screenshotCapture?.captureScreenBase64()
                (result as? com.elon.app.agent.infrastructure.vision.ScreenshotResult.Success)?.base64
            } else null
            
            AgentProtocol.screenMessage(
                appPackage = null,  // UINode 不含包名
                activity = null,
                visibleTexts = visibleTexts,
                clickableElements = clickableElements,
                screenshotBase64 = screenshot
            )
        } catch (e: Exception) {
            Log.e(TAG, "获取屏幕失败", e)
            AgentProtocol.errorMessage(
                ErrorCode.UNKNOWN,
                "获取屏幕失败: ${e.message}"
            )
        }
    }
    
    /**
     * 智能分析屏幕（AI Agent 能力）
     * 返回结构化的屏幕分析结果，包含页面类型、可交互元素、数据元素等
     */
    private suspend fun analyzeScreen(): OutgoingMessage {
        return try {
            val uiTree = uiTreeParser.readCurrentScreen()
            if (uiTree == null) {
                return AgentProtocol.errorMessage(
                    ErrorCode.UNKNOWN,
                    "无法读取屏幕内容"
                )
            }
            
            val analyzer = ScreenAnalyzer()
            val analysis = analyzer.analyze(uiTree)
            
            // 将分析结果转换为 JSON 格式的消息
            OutgoingMessage(
                type = "analyze_result",
                payload = buildString {
                    append("{")
                    append("\"app_context\":\"${analysis.appContext}\",")
                    append("\"page_type\":\"${analysis.pageType}\",")
                    append("\"interactive_elements\":[")
                    append(analysis.interactiveElements.joinToString(",") { elem ->
                        "{\"text\":\"${escapeJson(elem.text)}\",\"className\":\"${elem.className}\",\"bounds\":\"${elem.bounds}\"}"
                    })
                    append("],")
                    append("\"data_elements\":[")
                    append(analysis.dataElements.joinToString(",") { elem ->
                        "{\"rawText\":\"${escapeJson(elem.rawText)}\",\"type\":\"${elem.type}\",\"value\":${elem.value}}"
                    })
                    append("],")
                    append("\"hot_content\":[")
                    append(analysis.hotContent.joinToString(",") { hot ->
                        "{\"text\":\"${escapeJson(hot.text)}\",\"value\":${hot.value}}"
                    })
                    append("],")
                    append("\"navigation\":[")
                    append(analysis.navigationElements.joinToString(",") { nav ->
                        "{\"text\":\"${escapeJson(nav.text)}\",\"position\":\"${nav.position}\"}"
                    })
                    append("],")
                    append("\"ai_summary\":\"${escapeJson(analyzer.generateAISummary(analysis))}\"")
                    append("}")
                }
            )
        } catch (e: Exception) {
            Log.e(TAG, "屏幕分析失败", e)
            AgentProtocol.errorMessage(
                ErrorCode.UNKNOWN,
                "屏幕分析失败: ${e.message}"
            )
        }
    }
    
    /**
     * 生成执行脚本（AI Agent 能力）
     * 根据目标描述生成可执行的步骤脚本
     */
    private suspend fun generateScript(goal: String): OutgoingMessage {
        return try {
            if (goal.isBlank()) {
                return AgentProtocol.errorMessage(
                    ErrorCode.INVALID_MESSAGE,
                    "目标不能为空"
                )
            }
            
            val uiTree = uiTreeParser.readCurrentScreen()
            if (uiTree == null) {
                return AgentProtocol.errorMessage(
                    ErrorCode.UNKNOWN,
                    "无法读取屏幕内容"
                )
            }
            
            val analyzer = ScreenAnalyzer()
            val analysis = analyzer.analyze(uiTree)
            
            val generator = ScriptGenerator()
            val script = generator.generateScript(goal, analysis)
            
            // 返回生成的脚本
            OutgoingMessage(
                type = "script_result",
                payload = generator.toJson(script)
            )
        } catch (e: Exception) {
            Log.e(TAG, "脚本生成失败", e)
            AgentProtocol.errorMessage(
                ErrorCode.UNKNOWN,
                "脚本生成失败: ${e.message}"
            )
        }
    }
    
    /**
     * 客户端连接回调
     */
    private fun onClientConnected(clientId: String) {
        Log.i(TAG, "PC 客户端连接: $clientId")
        
        // 开始屏幕同步（如果有客户端）
        if (webSocketServer.getClientCount() == 1) {
            startScreenSync()
        }
    }
    
    /**
     * 客户端断开回调
     */
    private fun onClientDisconnected(clientId: String) {
        Log.i(TAG, "PC 客户端断开: $clientId")
        
        // 停止屏幕同步（如果没有客户端）
        if (webSocketServer.getClientCount() == 0) {
            stopScreenSync()
        }
    }
    
    /**
     * 开始屏幕同步
     */
    private fun startScreenSync() {
        if (screenSyncJob?.isActive == true) return
        
        screenSyncJob = scope.launch {
            while (isActive) {
                try {
                    val screen = getScreen(includeScreenshot = false)
                    webSocketServer.broadcast(screen)
                } catch (e: Exception) {
                    Log.w(TAG, "屏幕同步失败", e)
                }
                delay(SCREEN_SYNC_INTERVAL)
            }
        }
        
        Log.i(TAG, "屏幕同步已启动")
    }
    
    /**
     * 停止屏幕同步
     */
    private fun stopScreenSync() {
        screenSyncJob?.cancel()
        screenSyncJob = null
        Log.i(TAG, "屏幕同步已停止")
    }
    
    /**
     * 发送日志到 PC
     */
    suspend fun sendLog(level: LogLevel, message: String, tag: String = "Agent") {
        if (webSocketServer.getClientCount() > 0) {
            webSocketServer.broadcast(AgentProtocol.logMessage(level, message, tag))
        }
    }
    
    /**
     * 发送 AI 思考过程
     */
    suspend fun sendThinking(thought: String, decision: String?, action: String?) {
        webSocketServer.broadcast(AgentProtocol.thinkingMessage(thought, decision, action))
    }
    
    /**
     * 发送进度更新
     */
    suspend fun sendProgress(stepNumber: Int, totalSteps: Int, currentTask: String) {
        webSocketServer.broadcast(AgentProtocol.progressMessage(
            stepNumber = stepNumber,
            totalSteps = totalSteps,
            currentTask = currentTask,
            taskStatus = com.elon.app.agent.domain.planning.TaskStatus.IN_PROGRESS
        ))
    }
    
    /**
     * 释放资源
     */
    fun release() {
        stopScreenSync()
        currentGoalJob?.cancel()
        currentGoalJob = null
        scope.cancel()
    }
}
