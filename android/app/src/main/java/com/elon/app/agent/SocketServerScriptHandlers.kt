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

// ===== 从 SocketServer.kt 提取的扩展函数 =====

internal fun SocketServer.handleSmartExecute(userInput: String, output: PrintWriter, socket: Socket) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_ENGINE","message":"请先设置 API Key"}""")
        return
    }
    
    output.println("""{"status":"analyzing","input":"${escapeJson(userInput)}"}""")
    output.flush()
    
    scope.launch {
        try {
            // 第一步：分析意图
            val intentResult = engine.analyzeIntent(userInput)
            
            when (intentResult.intent) {
                com.elon.app.agent.application.ScriptEngine.UserIntent.CHAT -> {
                    // 聊天意图 - 直接返回 AI 回复
                    output.println("""{"status":"chat","response":"${escapeJson(intentResult.chatResponse ?: "我可以帮你操作手机，比如'打开小红书'、'搜索CPU价格'等。")}","confidence":${intentResult.confidence}}""")
                    output.flush()
                    socket.close()
                }
                
                com.elon.app.agent.application.ScriptEngine.UserIntent.PHONE_OPERATION -> {
                    // 操作意图 - 走脚本流程
                    val goal = intentResult.operationGoal ?: userInput
                    output.println("""{"status":"operation","goal":"${escapeJson(goal)}","confidence":${intentResult.confidence}}""")
                    output.flush()
                    
                    // 继续脚本生成和执行流程
                    handleScriptGenerateInternal(goal, output, socket, engine)
                }
                
                else -> {
                    // 不确定 - 默认当作操作处理
                    output.println("""{"status":"operation","goal":"${escapeJson(userInput)}","confidence":${intentResult.confidence},"note":"意图不明确，尝试作为操作处理"}""")
                    output.flush()
                    handleScriptGenerateInternal(userInput, output, socket, engine)
                }
            }
        } catch (e: Exception) {
            Log.e("Agent", "智能执行失败", e)
            output.println("""{"status":"error","error":"${escapeJson(e.message ?: "Unknown")}"}""")
            output.flush()
            socket.close()
        }
    }
}

/**
 * 仅分析意图（不执行）
 */
internal fun SocketServer.handleAnalyzeIntent(userInput: String, output: PrintWriter) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_ENGINE","message":"请先设置 API Key"}""")
        return
    }
    
    scope.launch {
        try {
            val result = engine.analyzeIntent(userInput)
            output.println("""{
                "success": true,
                "intent": "${result.intent}",
                "confidence": ${result.confidence},
                "chat_response": ${if (result.chatResponse != null) "\"${escapeJson(result.chatResponse)}\"" else "null"},
                "operation_goal": ${if (result.operationGoal != null) "\"${escapeJson(result.operationGoal)}\"" else "null"}
            }""".trimIndent().replace("\n", "").replace("  ", ""))
        } catch (e: Exception) {
            output.println("""{"error":"ANALYZE_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
        }
    }
}

/**
 * 脚本生成内部实现（供智能执行调用）
 */
internal suspend fun SocketServer.handleScriptGenerateInternal(
    goal: String, 
    output: PrintWriter, 
    socket: Socket, 
    engine: com.elon.app.agent.application.ScriptEngine
) {
    engine.onLog = { log ->
        output.println("""{"log":"${escapeJson(log)}"}""")
        output.flush()
    }
    
    val result = engine.generateScript(goal)
    result.onSuccess { script ->
        output.println("""{"status":"generated","script_id":"${script.id}","name":"${escapeJson(script.name)}","steps":${script.steps.size}}""")
        output.flush()
        
        // 自动执行
        output.println("""{"status":"executing"}""")
        output.flush()
        
        val execResult = engine.executeWithAutoImprove(script.id) { current, total, desc ->
            output.println("""{"progress":$current,"total":$total,"step":"${escapeJson(desc)}"}""")
            output.flush()
        }
        
        if (execResult.success) {
            output.println("""{"status":"completed","steps_executed":${execResult.stepsExecuted},"extracted_data":${gson.toJson(execResult.extractedData)}}""")
        } else {
            output.println("""{"status":"failed","error":"${escapeJson(execResult.error ?: "Unknown")}","steps_executed":${execResult.stepsExecuted},"failed_step":${execResult.failedStepIndex ?: -1}}""")
        }
        output.flush()
    }
    result.onFailure { e ->
        output.println("""{"status":"error","error":"${escapeJson(e.message ?: "Unknown")}"}""")
        output.flush()
    }
    
    socket.close()
}

/**
 * 🆕 智能屏幕分析（使用改进的 getRootNode）
 */
internal fun SocketServer.handleAnalyzeScreen(output: PrintWriter) {
    try {
        val root = getRootNode()
        if (root == null) {
            output.println("""{"error":"NO_ROOT","message":"No root window available"}""")
            return
        }
        
        // 将 AccessibilityNodeInfo 转换为 UINode
        val uiNode = convertToUINode(root)
        
        // 使用 ScreenAnalyzer 进行智能分析
        val analysis = screenAnalyzer.analyze(uiNode)
        
        // 构建 JSON 响应
        val response = buildString {
            append("{")
            append("\"success\":true,")
            append("\"app_context\":\"${analysis.appContext}\",")
            append("\"page_type\":\"${analysis.pageType}\",")
            append("\"summary\":\"${escapeJson(analysis.summary)}\",")
            append("\"interactive_count\":${analysis.interactiveElements.size},")
            append("\"data_count\":${analysis.dataElements.size},")
            append("\"hot_count\":${analysis.hotContent.size},")
            append("\"interactive_elements\":[")
            append(analysis.interactiveElements.take(10).joinToString(",") { elem ->
                "{\"text\":\"${escapeJson(elem.text)}\",\"class\":\"${elem.className}\",\"bounds\":\"${elem.bounds}\"}"
            })
            append("],")
            append("\"hot_content\":[")
            append(analysis.hotContent.joinToString(",") { hot ->
                "{\"text\":\"${escapeJson(hot.text)}\",\"value\":${hot.value}}"
            })
            append("],")
            append("\"ai_summary\":\"${escapeJson(screenAnalyzer.generateAISummary(analysis))}\"")
            append("}")
        }
        
        output.println(response)
        Log.i("Agent", "分析完成: ${analysis.interactiveElements.size} 个交互元素, ${analysis.hotContent.size} 个热门内容")
        
    } catch (e: Exception) {
        Log.e("Agent", "分析失败", e)
        output.println("""{"error":"ANALYZE_FAILED","message":"${escapeJson(e.message ?: "Unknown error")}"}""")
    }
}

/**
 * 🆕 生成执行脚本（使用改进的 getRootNode）
 */
internal fun SocketServer.handleGenerateScript(goal: String, output: PrintWriter) {
    try {
        if (goal.isBlank()) {
            output.println("""{"error":"EMPTY_GOAL","message":"Goal cannot be empty"}""")
            return
        }
        
        val root = getRootNode()
        if (root == null) {
            output.println("""{"error":"NO_ROOT","message":"No root window available"}""")
            return
        }
        
        // 先分析屏幕
        val uiNode = convertToUINode(root)
        val analysis = screenAnalyzer.analyze(uiNode)
        
        // 生成脚本
        val script = scriptGenerator.generateScript(goal, analysis)
        val scriptJson = scriptGenerator.toJson(script)
        
        output.println(scriptJson)
        Log.i("Agent", "脚本生成完成: ${script.steps.size} 步骤")
        
    } catch (e: Exception) {
        Log.e("Agent", "脚本生成失败", e)
        output.println("""{"error":"SCRIPT_FAILED","message":"${escapeJson(e.message ?: "Unknown error")}"}""")
    }
}

/**
 * 🆕 设置 API Key
 */
internal fun SocketServer.handleSetApiKey(key: String, output: PrintWriter) {
    if (key.isBlank()) {
        output.println("""{"error":"EMPTY_KEY","message":"API key cannot be empty"}""")
        return
    }
    
    setApiKey(key)
    output.println("""{"success":true,"message":"API Key 已设置，AI 引擎已就绪"}""")
}

/**
 * 🆕 AI 自主执行目标 (流式返回执行日志)
 */
internal fun SocketServer.handleRunAIGoal(goal: String, output: PrintWriter, socket: Socket) {
    if (goal.isBlank()) {
        output.println("""{"error":"EMPTY_GOAL","message":"Goal cannot be empty"}""")
        socket.close()
        return
    }
    
    val engine = aiEngine
    if (engine == null) {
        output.println("""{"error":"NO_AI_ENGINE","message":"请先使用 SET_API_KEY:your_key 设置 API Key"}""")
        socket.close()
        return
    }
    
    // 启动协程执行
    scope.launch {
        try {
            output.println("""{"status":"started","goal":"${escapeJson(goal)}"}""")
            output.flush()
            
            val result = engine.executeGoal(goal)
            
            // 返回执行结果
            val logsJson = result.logs.joinToString(",") { log ->
                """{"time":${log.timestamp},"type":"${log.type}","content":"${escapeJson(log.content)}"}"""
            }
            
            output.println("""{"status":"completed","success":${result.success},"message":"${escapeJson(result.message)}","steps_executed":${result.stepsExecuted},"logs":[$logsJson]}""")
            output.flush()
            
        } catch (e: Exception) {
            Log.e("Agent", "AI 执行失败", e)
            output.println("""{"status":"error","message":"${escapeJson(e.message ?: "Unknown error")}"}""")
        } finally {
            socket.close()
        }
    }
}

/**
 * 🆕 停止 AI 执行
 */
internal fun SocketServer.handleStopAI(output: PrintWriter) {
    aiEngine?.stop()
    output.println("""{"success":true,"message":"已发送停止信号"}""")
}

/**
 * 将 AccessibilityNodeInfo 转换为 UINode
 */
internal fun SocketServer.convertToUINode(node: AccessibilityNodeInfo): UINode {
    val rect = Rect()
    node.getBoundsInScreen(rect)
    
    val children = mutableListOf<UINode>()
    for (i in 0 until node.childCount) {
        node.getChild(i)?.let { child ->
            children.add(convertToUINode(child))
        }
    }
    
    return UINode(
        className = node.className?.toString() ?: "",
        text = node.text?.toString(),
        contentDescription = node.contentDescription?.toString(),
        resourceId = node.viewIdResourceName,
        bounds = rect,
        isClickable = node.isClickable,
        isEnabled = node.isEnabled,
        isPassword = node.isPassword,
        children = children
    )
}

/**
 * JSON 字符串转义
 */
internal fun SocketServer.escapeJson(text: String): String {
    return text
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

internal fun SocketServer.serializeNode(node: AccessibilityNodeInfo): NodeData {
    val rect = Rect()
    node.getBoundsInScreen(rect)
    
    val data = NodeData(
        className = node.className?.toString() ?: "",
        text = node.text?.toString(),
        contentDescription = node.contentDescription?.toString(),
        resourceId = node.viewIdResourceName,
        bounds = "${rect.left},${rect.top},${rect.right},${rect.bottom}",
        children = mutableListOf()
    )

    for (i in 0 until node.childCount) {
        node.getChild(i)?.let { child ->
            data.children.add(serializeNode(child))
        }
    }
    return data
}

// ========== 🆕 脚本系统命令处理 ==========

/**
 * 生成脚本
 */
internal fun SocketServer.handleScriptGenerate(goal: String, output: PrintWriter, socket: Socket) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_SCRIPT_ENGINE","message":"请先设置 API Key"}""")
        socket.close()
        return
    }
    
    output.println("""{"status":"generating","goal":"$goal"}""")
    output.flush()
    
    scope.launch {
        try {
            engine.onLog = { log ->
                output.println("""{"log":"${escapeJson(log)}"}""")
                output.flush()
            }
            
            val result = engine.generateScript(goal)
            
            result.fold(
                onSuccess = { script ->
                    output.println("""{"status":"success","script":${gson.toJson(script)}}""")
                },
                onFailure = { error ->
                    output.println("""{"status":"error","error":"${escapeJson(error.message ?: "Unknown error")}"}""")
                }
            )
        } catch (e: Exception) {
            output.println("""{"status":"error","error":"${escapeJson(e.message ?: "Unknown error")}"}""")
        } finally {
            engine.onLog = null
            socket.close()
        }
    }
}

/**
 * 执行脚本
 */
internal fun SocketServer.handleScriptExecute(scriptId: String, output: PrintWriter, socket: Socket) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_SCRIPT_ENGINE","message":"请先设置 API Key"}""")
        socket.close()
        return
    }
    
    output.println("""{"status":"executing","script_id":"$scriptId"}""")
    output.flush()
    
    scope.launch {
        try {
            engine.onLog = { log ->
                output.println("""{"log":"${escapeJson(log)}"}""")
                output.flush()
            }
            
            val result = engine.executeScript(scriptId) { step, total, desc ->
                output.println("""{"progress":{"step":$step,"total":$total,"description":"${escapeJson(desc)}"}}""")
                output.flush()
            }
            
            output.println("""{"status":"complete","result":${gson.toJson(result)}}""")
        } catch (e: Exception) {
            output.println("""{"status":"error","error":"${escapeJson(e.message ?: "Unknown error")}"}""")
        } finally {
            engine.onLog = null
            socket.close()
        }
    }
}

/**
 * 执行脚本（自动改进模式）
 */
internal fun SocketServer.handleScriptExecuteAuto(scriptId: String, output: PrintWriter, socket: Socket) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_SCRIPT_ENGINE","message":"请先设置 API Key"}""")
        socket.close()
        return
    }
    
    output.println("""{"status":"executing_auto","script_id":"$scriptId"}""")
    output.flush()
    
    scope.launch {
        try {
            engine.onLog = { log ->
                output.println("""{"log":"${escapeJson(log)}"}""")
                output.flush()
            }
            
            val result = engine.executeWithAutoImprove(scriptId) { step, total, desc ->
                output.println("""{"progress":{"step":$step,"total":$total,"description":"${escapeJson(desc)}"}}""")
                output.flush()
            }
            
            output.println("""{"status":"complete","result":${gson.toJson(result)}}""")
        } catch (e: Exception) {
            output.println("""{"status":"error","error":"${escapeJson(e.message ?: "Unknown error")}"}""")
        } finally {
            engine.onLog = null
            socket.close()
        }
    }
}

/**
 * 手动改进脚本
 */
internal fun SocketServer.handleScriptImprove(scriptId: String, output: PrintWriter) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_SCRIPT_ENGINE","message":"请先设置 API Key"}""")
        return
    }
    
    val script = engine.loadScript(scriptId)
    if (script == null) {
        output.println("""{"error":"SCRIPT_NOT_FOUND","message":"脚本不存在: $scriptId"}""")
        return
    }
    
    // 创建一个模拟的失败结果用于改进
    val mockFailResult = com.elon.app.agent.domain.script.ScriptExecutionResult(
        success = false,
        stepsExecuted = 0,
        totalSteps = script.steps.size,
        error = "手动触发改进",
        logs = listOf("用户请求改进脚本")
    )
    
    scope.launch {
        val improvedScript = engine.improveScript(script, mockFailResult)
        if (improvedScript != null) {
            engine.saveScript(improvedScript)
            output.println("""{"status":"improved","script":${gson.toJson(improvedScript)}}""")
        } else {
            output.println("""{"status":"no_improvement","message":"AI 未提供改进建议"}""")
        }
    }
}

/**
 * 获取脚本详情
 */
internal fun SocketServer.handleScriptGet(scriptId: String, output: PrintWriter) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_SCRIPT_ENGINE","message":"请先设置 API Key"}""")
        return
    }
    
    val script = engine.loadScript(scriptId)
    if (script != null) {
        output.println(gson.toJson(script))
    } else {
        output.println("""{"error":"SCRIPT_NOT_FOUND","message":"脚本不存在: $scriptId"}""")
    }
}

/**
 * 列出所有脚本
 */
internal fun SocketServer.handleScriptList(output: PrintWriter) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_SCRIPT_ENGINE","message":"请先设置 API Key"}""")
        return
    }
    
    val scripts = engine.listScripts()
    val summaries = scripts.map { script ->
        mapOf(
            "id" to script.id,
            "name" to script.name,
            "goal" to script.goal,
            "version" to script.version,
            "steps_count" to script.steps.size,
            "success_count" to script.successCount,
            "fail_count" to script.failCount,
            "created_at" to script.createdAt
        )
    }
    output.println(gson.toJson(summaries))
}

/**
 * 删除脚本
 */
internal fun SocketServer.handleScriptDelete(scriptId: String, output: PrintWriter) {
    val engine = scriptEngine
    if (engine == null) {
        output.println("""{"error":"NO_SCRIPT_ENGINE","message":"请先设置 API Key"}""")
        return
    }
    
    val success = engine.deleteScript(scriptId)
    if (success) {
        output.println("""{"status":"deleted","script_id":"$scriptId"}""")
    } else {
        output.println("""{"error":"DELETE_FAILED","message":"删除失败: $scriptId"}""")
    }
}

// ==================== 📸 屏幕获取模式命令处理 ====================

/**
 * 获取 SmartScreenReader（从 AgentService）
 */
internal fun SocketServer.getSmartScreenReader(): com.elon.app.agent.infrastructure.accessibility.SmartScreenReader? {
    return (service as? AgentService)?.smartScreenReader
}

/**
 * 设置屏幕获取模式
 */
