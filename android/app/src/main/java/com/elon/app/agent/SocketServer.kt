package com.elon.app.agent

import android.accessibilityservice.AccessibilityService
import android.content.Context
import android.graphics.Rect
import android.util.Log
import android.view.accessibility.AccessibilityNodeInfo
import com.elon.app.agent.application.*
import com.elon.app.agent.domain.screen.UINode
import com.elon.app.agent.infrastructure.vision.ScreenAnalyzer
import com.elon.app.agent.infrastructure.vision.ScriptGenerator
import com.google.gson.Gson
import com.google.gson.GsonBuilder
import kotlinx.coroutines.*
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.PrintWriter
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.Executors

data class NodeData(
    val className: String,
    val text: String?,
    val contentDescription: String?,
    val resourceId: String?,
    var bounds: String,
    val children: MutableList<NodeData>
)

class SocketServer(private val service: AccessibilityService) {
    internal var serverSocket: ServerSocket? = null
    internal var isRunning = false
    internal val executor = Executors.newCachedThreadPool()
    internal val gson = GsonBuilder().setPrettyPrinting().create()
    internal val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    
    // 智能分析组件
    internal val screenAnalyzer = ScreenAnalyzer()
    internal val scriptGenerator = ScriptGenerator()
    
    /**
     * 🆕 获取 Root Window 的辅助函数
     * 
     * 先尝试 rootInActiveWindow，如果为 null 则从 windows 中获取活动窗口的 root
     * 这解决了部分设备（特别是小米/MIUI）上 rootInActiveWindow 返回 null 的问题
     */
    internal fun getRootNode(): AccessibilityNodeInfo? {
        // 首先尝试标准方法
        service.rootInActiveWindow?.let { return it }
        
        // 备选方案：从 windows 列表中获取
        try {
            val windows = service.windows
            if (windows != null && windows.isNotEmpty()) {
                Log.d("Agent", "🔍 windows API: 找到 ${windows.size} 个窗口")
                
                // 1. 优先选择 isActive 且 isFocused 的窗口
                for (window in windows) {
                    if (window.isActive && window.isFocused) {
                        window.root?.let { root ->
                            Log.d("Agent", "✅ 使用活动焦点窗口 (类型: ${window.type}, 包: ${root.packageName})")
                            return root
                        }
                    }
                }
                
                // 2. 其次选择 isActive 的应用窗口（TYPE_APPLICATION = 1）
                for (window in windows) {
                    if (window.isActive && window.type == 1) {
                        window.root?.let { root ->
                            Log.d("Agent", "✅ 使用活动应用窗口 (包: ${root.packageName})")
                            return root
                        }
                    }
                }
                
                // 3. 选择任何 isActive 的窗口
                for (window in windows) {
                    if (window.isActive) {
                        window.root?.let { root ->
                            Log.d("Agent", "✅ 使用活动窗口 (类型: ${window.type}, 包: ${root.packageName})")
                            return root
                        }
                    }
                }
                
                // 4. 最后降级：选择任何有 root 的应用窗口
                val appWindow = windows.find { it.type == 1 && it.root != null }
                if (appWindow != null) {
                    Log.d("Agent", "⚠️ 降级：使用首个应用窗口 (包: ${appWindow.root?.packageName})")
                    return appWindow.root
                }
                
                // 5. 兜底：任何有 root 的窗口
                for (window in windows) {
                    window.root?.let { root ->
                        Log.d("Agent", "⚠️ 兜底：使用窗口 (类型: ${window.type}, 包: ${root.packageName})")
                        return root
                    }
                }
            }
        } catch (e: Exception) {
            Log.w("Agent", "windows API 获取失败: ${e.message}")
        }
        
        return null
    }
    
    // 🆕 AI 自主执行引擎 (需要 API Key)
    internal var aiEngine: AIAutonomousEngine? = null
    
    // 🆕 脚本引擎
    internal var scriptEngine: ScriptEngine? = null
    
    // API Key 配置 (用户的腾讯混元 Key)
    internal var apiKey: String = ""
    
    // SharedPreferences 常量 - 与 AgentConfigActivity 保持一致
    private companion object {
        const val PREF_NAME = "agent_config"  // 与 AgentConfigActivity 一致
        const val KEY_API_KEY = "hunyuan_api_key"  // 腾讯混元 API Key
    }
    
    /**
     * 设置 API Key 并持久化保存
     *
     * **重构后**：AI 引擎/脚本引擎不再依赖 apiKey，
     * 调用只是保存 Key 以供后续 [AIClientFactory] 自动选链路。
     */
    fun setApiKey(key: String) {
        apiKey = key
        if (key.isNotBlank()) {
            // 保存到 SharedPreferences
            saveApiKey(key)
            
            aiEngine = AIAutonomousEngine(service)
            scriptEngine = ScriptEngine(service)  // 🆕 初始化脚本引擎
            Log.i("Agent", "AI 引擎和脚本引擎已初始化，API Key 已保存")
        }
    }
    
    /**
     * 保存 API Key 到 SharedPreferences
     */
    private fun saveApiKey(key: String) {
        try {
            val prefs = service.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
            prefs.edit().putString(KEY_API_KEY, key).apply()
            Log.i("Agent", "API Key 已持久化保存")
        } catch (e: Exception) {
            Log.e("Agent", "保存 API Key 失败", e)
        }
    }
    
    /**
     * 从 SharedPreferences 加载 API Key
     *
     * **重构后**：无论是否有 Key 都初始化 AI/脚本引擎，
     * AIClientFactory 会走到服务器 CLI 兜底。
     */
    fun loadSavedApiKey() {
        try {
            val prefs = service.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
            val savedKey = prefs.getString(KEY_API_KEY, "") ?: ""
            if (savedKey.isNotBlank()) {
                apiKey = savedKey
                Log.i("Agent", "已加载保存的 API Key")
            } else {
                Log.i("Agent", "没有保存的 API Key，将依赖服务器 CLI 或其他配置")
            }
            // 无论是否有 Key，都初始化引擎。
            aiEngine = AIAutonomousEngine(service)
            scriptEngine = ScriptEngine(service)
            Log.i("Agent", "AI 引擎和脚本引擎已就绪")
        } catch (e: Exception) {
            Log.e("Agent", "加载 API Key 失败", e)
        }
    }

    fun start(port: Int) {
        isRunning = true
        Thread {
            try {
                serverSocket = ServerSocket(port)
                Log.d("Agent", "Server started on port $port")
                while (isRunning) {
                    val client = serverSocket?.accept()
                    client?.let {
                        executor.submit { handleClient(it) }
                    }
                }
            } catch (e: Exception) {
                Log.e("Agent", "Server error", e)
            }
        }.start()
    }

    fun stop() {
        isRunning = false
        try {
            serverSocket?.close()
        } catch (e: Exception) {
            Log.e("Agent", "Error closing server", e)
        }
    }

    private fun handleClient(socket: Socket) {
        try {
            val input = BufferedReader(InputStreamReader(socket.getInputStream()))
            val output = PrintWriter(socket.getOutputStream(), true)

            val command = input.readLine()?.trim() ?: ""
            Log.d("Agent", "Received command: $command")

            when {
                command == "DUMP" -> {
                    // 原有的 UI 树 dump（使用改进的 getRootNode）
                    val root = getRootNode()
                    if (root != null) {
                        val dump = serializeNode(root)
                        output.println(gson.toJson(dump))
                    } else {
                        output.println("ERROR: No root window")
                    }
                }
                command == "ANALYZE" || command == "ANALYZE_SCREEN" -> {
                    // 🆕 智能屏幕分析
                    handleAnalyzeScreen(output)
                }
                command.startsWith("GENERATE_SCRIPT:") -> {
                    // 🆕 生成脚本
                    val goal = command.removePrefix("GENERATE_SCRIPT:").trim()
                    handleGenerateScript(goal, output)
                }
                command.startsWith("SET_API_KEY:") -> {
                    // 🆕 设置 API Key
                    val key = command.removePrefix("SET_API_KEY:").trim()
                    handleSetApiKey(key, output)
                }
                command.startsWith("RUN_AI_GOAL:") -> {
                    // 🆕 AI 自主执行目标
                    val goal = command.removePrefix("RUN_AI_GOAL:").trim()
                    handleRunAIGoal(goal, output, socket)
                    return  // 特殊处理，不要关闭 socket
                }
                command == "STOP_AI" -> {
                    // 🆕 停止 AI 执行
                    handleStopAI(output)
                }
                // ========== 🆕 脚本系统命令 ==========
                command.startsWith("SMART_EXECUTE:") -> {
                    // 🧠 智能执行：先分析意图，再决定走脚本还是聊天
                    val userInput = command.removePrefix("SMART_EXECUTE:").trim()
                    handleSmartExecute(userInput, output, socket)
                    return
                }
                command.startsWith("ANALYZE_INTENT:") -> {
                    // 仅分析意图，不执行
                    val userInput = command.removePrefix("ANALYZE_INTENT:").trim()
                    handleAnalyzeIntent(userInput, output)
                }
                command.startsWith("SCRIPT_GENERATE:") -> {
                    val goal = command.removePrefix("SCRIPT_GENERATE:").trim()
                    handleScriptGenerate(goal, output, socket)
                    return
                }
                command.startsWith("SCRIPT_EXECUTE:") -> {
                    val scriptId = command.removePrefix("SCRIPT_EXECUTE:").trim()
                    handleScriptExecute(scriptId, output, socket)
                    return
                }
                command.startsWith("SCRIPT_EXECUTE_AUTO:") -> {
                    val scriptId = command.removePrefix("SCRIPT_EXECUTE_AUTO:").trim()
                    handleScriptExecuteAuto(scriptId, output, socket)
                    return
                }
                command.startsWith("SCRIPT_IMPROVE:") -> {
                    val scriptId = command.removePrefix("SCRIPT_IMPROVE:").trim()
                    handleScriptImprove(scriptId, output)
                }
                command.startsWith("SCRIPT_GET:") -> {
                    val scriptId = command.removePrefix("SCRIPT_GET:").trim()
                    handleScriptGet(scriptId, output)
                }
                command == "SCRIPT_LIST" -> {
                    handleScriptList(output)
                }
                command.startsWith("SCRIPT_DELETE:") -> {
                    val scriptId = command.removePrefix("SCRIPT_DELETE:").trim()
                    handleScriptDelete(scriptId, output)
                }
                // ========== 🔧 调试接口命令 ==========
                command == "DEBUG_STATUS" -> {
                    handleDebugStatus(output)
                }
                command == "DEBUG_LAST_ERROR" -> {
                    handleDebugLastError(output)
                }
                command == "DEBUG_ERROR_HISTORY" || command.startsWith("DEBUG_ERROR_HISTORY:") -> {
                    val limit = command.removePrefix("DEBUG_ERROR_HISTORY:").toIntOrNull() ?: 10
                    handleDebugErrorHistory(limit, output)
                }
                command == "DEBUG_EXECUTION_HISTORY" || command.startsWith("DEBUG_EXECUTION_HISTORY:") -> {
                    val limit = command.removePrefix("DEBUG_EXECUTION_HISTORY:").toIntOrNull() ?: 20
                    handleDebugExecutionHistory(limit, output)
                }
                command == "DEBUG_LOGS" || command.startsWith("DEBUG_LOGS:") -> {
                    val limit = command.removePrefix("DEBUG_LOGS:").toIntOrNull() ?: 50
                    handleDebugLogs(limit, output)
                }
                command == "DEBUG_HEALTH" -> {
                    handleDebugHealth(output)
                }
                command == "DEBUG_SCREEN" -> {
                    handleDebugScreen(output)
                }
                command == "DEBUG_CLEAR" -> {
                    handleDebugClear(output)
                }
                command == "DEBUG_HELP" -> {
                    handleDebugHelp(output)
                }
                // ========== 状态检查 ==========
                command == "STATUS" -> {
                    // 状态检查
                    val hasAI = aiEngine != null
                    val hasScript = scriptEngine != null
                    val scriptCount = scriptEngine?.listScripts()?.size ?: 0
                    val currentExecMode = scriptEngine?.executionMode?.name ?: "SMART"
                    val currentScreenMode = getSmartScreenReader()?.currentMode?.name ?: "FULL_DUMP"
                    output.println("""{"status":"ok","version":"3.4","ai_enabled":$hasAI,"script_enabled":$hasScript,"script_count":$scriptCount,"execution_mode":"$currentExecMode","screen_mode":"$currentScreenMode","features":["SMART_EXECUTE","ANALYZE_INTENT","DUMP","ANALYZE","SET_API_KEY","RUN_AI_GOAL","STOP_AI","SCRIPT_GENERATE","SCRIPT_EXECUTE","SCRIPT_EXECUTE_AUTO","SCRIPT_IMPROVE","SCRIPT_GET","SCRIPT_LIST","SCRIPT_DELETE","SET_EXECUTION_MODE","GET_EXECUTION_MODE","LIST_EXECUTION_MODES","TEST_POPUP_DISMISS","SET_SCREEN_MODE","GET_SCREEN_MODE","LIST_SCREEN_MODES","SCREEN_DIFF","SCREEN_CHANGES","SCREEN_SNAPSHOT","SCREEN_STATS","DEBUG_STATUS","DEBUG_LAST_ERROR","DEBUG_ERROR_HISTORY","DEBUG_EXECUTION_HISTORY","DEBUG_LOGS","DEBUG_HEALTH","DEBUG_SCREEN","DEBUG_CLEAR","DEBUG_HELP"]}""")
                }
                // ========== 🎮 执行模式命令 ==========
                command.startsWith("SET_EXECUTION_MODE:") -> {
                    val modeName = command.removePrefix("SET_EXECUTION_MODE:").trim().uppercase()
                    handleSetExecutionMode(modeName, output)
                }
                command == "GET_EXECUTION_MODE" -> {
                    handleGetExecutionMode(output)
                }
                command == "LIST_EXECUTION_MODES" -> {
                    handleListExecutionModes(output)
                }
                command == "TEST_POPUP_DISMISS" -> {
                    handleTestPopupDismiss(output)
                }
                command.startsWith("SCRIPT_EXECUTE_WITH_MODE:") -> {
                    // 格式: SCRIPT_EXECUTE_WITH_MODE:scriptId:MODE
                    val params = command.removePrefix("SCRIPT_EXECUTE_WITH_MODE:").trim()
                    handleScriptExecuteWithMode(params, output, socket)
                    return
                }
                // ========== 📸 屏幕获取模式命令 ==========
                command.startsWith("SET_SCREEN_MODE:") -> {
                    val modeName = command.removePrefix("SET_SCREEN_MODE:").trim().uppercase()
                    handleSetScreenMode(modeName, output)
                }
                command == "GET_SCREEN_MODE" -> {
                    handleGetScreenMode(output)
                }
                command == "LIST_SCREEN_MODES" -> {
                    handleListScreenModes(output)
                }
                command == "SCREEN_DIFF" -> {
                    handleScreenDiff(output)
                }
                command == "SCREEN_CHANGES" -> {
                    handleScreenChanges(output)
                }
                command == "SCREEN_SNAPSHOT" -> {
                    handleScreenSnapshot(output)
                }
                command == "SCREEN_STATS" -> {
                    handleScreenStats(output)
                }
                command == "DEBUG_WINDOWS" -> {
                    handleDebugWindows(output)
                }
                else -> {
                    output.println("""{"error":"UNKNOWN_COMMAND","message":"Unknown command: $command","hint":"发送 DEBUG_HELP 获取所有可用命令"}""")
                }
            }

            socket.close()
        } catch (e: Exception) {
            Log.e("Agent", "Client handling error", e)
        }
    }
    
    // ==================== 🎮 执行模式命令处理 ====================
    
    /**
     * 设置执行模式
     */
    private fun handleSetExecutionMode(modeName: String, output: PrintWriter) {
        try {
            val engine = scriptEngine
            if (engine == null) {
                output.println("""{"error":"NO_ENGINE","message":"脚本引擎未初始化，请先设置 API Key"}""")
                return
            }
            
            val mode = try {
                com.elon.app.agent.domain.execution.ExecutionMode.valueOf(modeName)
            } catch (e: Exception) {
                output.println("""{"error":"INVALID_MODE","message":"无效的执行模式: $modeName","valid_modes":["FAST","SMART","MONITOR","AGENT"]}""")
                return
            }
            
            engine.executionMode = mode
            Log.i("Agent", "执行模式已切换为: ${mode.emoji} ${mode.displayName}")
            
            output.println(gson.toJson(mapOf(
                "success" to true,
                "mode" to mode.name,
                "display_name" to mode.displayName,
                "emoji" to mode.emoji,
                "description" to mode.description,
                "token_cost" to mode.tokenCostLevel.displayName
            )))
        } catch (e: Exception) {
            Log.e("Agent", "SET_EXECUTION_MODE 失败", e)
            output.println("""{"error":"SET_MODE_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
        }
    }
    
    /**
     * 获取当前执行模式
     */
    private fun handleGetExecutionMode(output: PrintWriter) {
        try {
            val engine = scriptEngine
            if (engine == null) {
                output.println("""{"error":"NO_ENGINE","message":"脚本引擎未初始化"}""")
                return
            }
            
            val mode = engine.executionMode
            output.println(gson.toJson(mapOf(
                "success" to true,
                "mode" to mode.name,
                "display_name" to mode.displayName,
                "emoji" to mode.emoji,
                "description" to mode.description,
                "token_cost" to mode.tokenCostLevel.displayName
            )))
        } catch (e: Exception) {
            output.println("""{"error":"GET_MODE_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
        }
    }
    
    /**
     * 列出所有可用执行模式
     */
    private fun handleListExecutionModes(output: PrintWriter) {
        try {
            val modes = com.elon.app.agent.domain.execution.ExecutionMode.values().map { mode ->
                mapOf(
                    "name" to mode.name,
                    "display_name" to mode.displayName,
                    "emoji" to mode.emoji,
                    "description" to mode.description,
                    "token_cost" to mode.tokenCostLevel.displayName,
                    "is_current" to (scriptEngine?.executionMode == mode)
                )
            }
            
            output.println(gson.toJson(mapOf(
                "success" to true,
                "modes" to modes,
                "current" to (scriptEngine?.executionMode?.name ?: "SMART")
            )))
        } catch (e: Exception) {
            output.println("""{"error":"LIST_MODES_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
        }
    }
    
    /**
     * 测试弹窗清理功能
     */
    private fun handleTestPopupDismiss(output: PrintWriter) {
        try {
            val popupDismisser = com.elon.app.agent.infrastructure.popup.PopupDismisser(service)
            
            // 先检测弹窗
            val detection = popupDismisser.detectPopup()
            
            if (!detection.hasPopup) {
                output.println(gson.toJson(mapOf(
                    "success" to true,
                    "has_popup" to false,
                    "message" to "当前屏幕没有检测到弹窗"
                )))
                return
            }
            
            // 尝试关闭弹窗
            val dismissed = popupDismisser.dismissPopupOnce()
            
            output.println(gson.toJson(mapOf(
                "success" to true,
                "has_popup" to true,
                "popup_type" to detection.popupType,
                "close_button" to detection.closeButtonText,
                "confidence" to detection.confidence,
                "dismissed" to dismissed,
                "message" to if (dismissed) "成功关闭弹窗" else "检测到弹窗但关闭失败"
            )))
        } catch (e: Exception) {
            Log.e("Agent", "TEST_POPUP_DISMISS 失败", e)
            output.println("""{"error":"POPUP_TEST_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
        }
    }
    
    /**
     * 使用指定模式执行脚本
     */
    private fun handleScriptExecuteWithMode(params: String, output: PrintWriter, socket: Socket) {
        val parts = params.split(":")
        if (parts.size < 2) {
            output.println("""{"error":"INVALID_PARAMS","message":"格式: SCRIPT_EXECUTE_WITH_MODE:scriptId:MODE"}""")
            return
        }
        
        val scriptId = parts[0]
        val modeName = parts[1].uppercase()
        
        val engine = scriptEngine
        if (engine == null) {
            output.println("""{"error":"NO_ENGINE","message":"脚本引擎未初始化，请先设置 API Key"}""")
            return
        }
        
        val mode = try {
            com.elon.app.agent.domain.execution.ExecutionMode.valueOf(modeName)
        } catch (e: Exception) {
            output.println("""{"error":"INVALID_MODE","message":"无效的执行模式: $modeName"}""")
            return
        }
        
        output.println("""{"status":"STARTED","script_id":"$scriptId","mode":"${mode.name}","mode_display":"${mode.emoji} ${mode.displayName}"}""")
        output.flush()
        
        scope.launch {
            try {
                val result = engine.executeScriptWithMode(scriptId, mode) { current, total, desc ->
                    val progress = mapOf(
                        "status" to "PROGRESS",
                        "current" to current,
                        "total" to total,
                        "description" to desc,
                        "mode" to mode.name
                    )
                    output.println(gson.toJson(progress))
                    output.flush()
                }
                
                val finalResult = mapOf(
                    "status" to if (result.success) "COMPLETED" else "FAILED",
                    "success" to result.success,
                    "steps_executed" to result.stepsExecuted,
                    "total_steps" to result.totalSteps,
                    "error" to result.error,
                    "mode" to mode.name,
                    "popups_dismissed" to result.popupsDismissed,
                    "ai_interventions" to result.aiInterventions,
                    "logs" to result.logs
                )
                output.println(gson.toJson(finalResult))
                output.flush()
                
            } catch (e: Exception) {
                Log.e("Agent", "SCRIPT_EXECUTE_WITH_MODE 失败", e)
                output.println("""{"status":"ERROR","error":"${escapeJson(e.message ?: "Unknown")}"}""")
                output.flush()
            } finally {
                socket.close()
            }
        }
    }
    
    // ==================== 🔧 调试命令处理 ====================
    
    /**
     * 获取完整调试状态
     */
}
