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

internal fun SocketServer.handleDebugStatus(output: PrintWriter) {
    try {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        output.println(debugInterface.getFullStatus(service, scriptEngine))
    } catch (e: Exception) {
        Log.e("Agent", "DEBUG_STATUS 失败", e)
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取最后一个错误
 */
internal fun SocketServer.handleDebugLastError(output: PrintWriter) {
    try {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        output.println(debugInterface.getLastError())
    } catch (e: Exception) {
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取错误历史
 */
internal fun SocketServer.handleDebugErrorHistory(limit: Int, output: PrintWriter) {
    try {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        output.println(debugInterface.getErrorHistory(limit))
    } catch (e: Exception) {
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取执行历史
 */
internal fun SocketServer.handleDebugExecutionHistory(limit: Int, output: PrintWriter) {
    try {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        output.println(debugInterface.getExecutionHistory(limit))
    } catch (e: Exception) {
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取调试日志
 */
internal fun SocketServer.handleDebugLogs(limit: Int, output: PrintWriter) {
    try {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        output.println(debugInterface.getRecentLogs(limit))
    } catch (e: Exception) {
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 健康检查
 */
internal fun SocketServer.handleDebugHealth(output: PrintWriter) {
    try {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        output.println(debugInterface.getHealthCheck(service, scriptEngine))
    } catch (e: Exception) {
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取当前屏幕信息（使用改进的 getRootNode）
 */
internal fun SocketServer.handleDebugScreen(output: PrintWriter) {
    try {
        val root = getRootNode()
        if (root == null) {
            output.println("""{"error":"NO_ROOT","message":"无法获取 UI 树，请确保目标应用在前台，或尝试重新开启无障碍服务"}""")
            return
        }
        
        val packageName = root.packageName?.toString() ?: "unknown"
        val className = root.className?.toString() ?: "unknown"
        
        // 收集基本元素信息
        val elements = mutableListOf<Map<String, Any?>>()
        collectScreenElements(root, elements, 0, 20) // 最多收集 20 个元素
        
        val response = mapOf(
            "success" to true,
            "package" to packageName,
            "activity" to className,
            "element_count" to elements.size,
            "elements" to elements
        )
        
        output.println(gson.toJson(response))
    } catch (e: Exception) {
        Log.e("Agent", "DEBUG_SCREEN 失败", e)
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 🆕 调试：获取所有窗口信息
 */
internal fun SocketServer.handleDebugWindows(output: PrintWriter) {
    try {
        val windowInfos = mutableListOf<Map<String, Any?>>()
        
        // 检查 rootInActiveWindow
        val rootActive = service.rootInActiveWindow
        val rootActiveInfo = if (rootActive != null) {
            mapOf(
                "source" to "rootInActiveWindow",
                "package" to rootActive.packageName?.toString(),
                "class" to rootActive.className?.toString(),
                "child_count" to rootActive.childCount
            )
        } else {
            mapOf("source" to "rootInActiveWindow", "status" to "NULL")
        }
        windowInfos.add(rootActiveInfo)
        
        // 检查 windows API
        val windows = service.windows
        if (windows != null) {
            for ((index, window) in windows.withIndex()) {
                val root = window.root
                windowInfos.add(mapOf(
                    "source" to "windows[$index]",
                    "type" to window.type,
                    "type_name" to when(window.type) {
                        1 -> "APPLICATION"
                        2 -> "INPUT_METHOD"
                        3 -> "SYSTEM"
                        4 -> "ACCESSIBILITY_OVERLAY"
                        else -> "UNKNOWN(${window.type})"
                    },
                    "is_active" to window.isActive,
                    "is_focused" to window.isFocused,
                    "has_root" to (root != null),
                    "package" to root?.packageName?.toString(),
                    "child_count" to (root?.childCount ?: 0)
                ))
            }
        } else {
            windowInfos.add(mapOf("source" to "windows", "status" to "NULL"))
        }
        
        output.println(gson.toJson(mapOf(
            "success" to true,
            "window_count" to windows?.size,
            "windows" to windowInfos
        )))
    } catch (e: Exception) {
        Log.e("Agent", "DEBUG_WINDOWS 失败", e)
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 递归收集屏幕元素
 */
internal fun SocketServer.collectScreenElements(
    node: AccessibilityNodeInfo, 
    elements: MutableList<Map<String, Any?>>,
    depth: Int,
    maxElements: Int
) {
    if (elements.size >= maxElements) return
    
    val text = node.text?.toString()
    val desc = node.contentDescription?.toString()
    val resourceId = node.viewIdResourceName
    
    // 只收集有文本或可点击的元素
    if (!text.isNullOrBlank() || !desc.isNullOrBlank() || node.isClickable) {
        val bounds = Rect()
        node.getBoundsInScreen(bounds)
        
        elements.add(mapOf(
            "text" to text,
            "description" to desc,
            "resource_id" to resourceId,
            "class" to node.className?.toString()?.substringAfterLast('.'),
            "clickable" to node.isClickable,
            "bounds" to "${bounds.left},${bounds.top},${bounds.right},${bounds.bottom}",
            "depth" to depth
        ))
    }
    
    // 递归子节点
    for (i in 0 until node.childCount) {
        val child = node.getChild(i) ?: continue
        collectScreenElements(child, elements, depth + 1, maxElements)
    }
}

/**
 * 清除调试历史
 */
internal fun SocketServer.handleDebugClear(output: PrintWriter) {
    try {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        debugInterface.clearHistory()
        output.println("""{"success":true,"message":"调试历史已清除"}""")
    } catch (e: Exception) {
        output.println("""{"error":"DEBUG_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 调试命令帮助
 */
internal fun SocketServer.handleDebugHelp(output: PrintWriter) {
    val help = mapOf(
        "version" to "3.1",
        "description" to "Agent 调试接口 - 为外部 AI (如 Copilot) 提供实时状态查询",
        "commands" to mapOf(
            "DEBUG_STATUS" to "获取完整运行状态（推荐首选）",
            "DEBUG_LAST_ERROR" to "获取最后一个错误详情",
            "DEBUG_ERROR_HISTORY" to "获取错误历史 (可选 :limit，如 DEBUG_ERROR_HISTORY:5)",
            "DEBUG_EXECUTION_HISTORY" to "获取执行历史 (可选 :limit)",
            "DEBUG_LOGS" to "获取最近调试日志 (可选 :limit)",
            "DEBUG_HEALTH" to "系统健康检查",
            "DEBUG_SCREEN" to "获取当前屏幕元素信息",
            "DEBUG_CLEAR" to "清除调试历史",
            "DEBUG_HELP" to "显示此帮助"
        ),
        "other_commands" to listOf(
            "STATUS - 系统状态",
            "DUMP - 获取完整 UI 树",
            "ANALYZE - 智能屏幕分析",
            "SCRIPT_LIST - 脚本列表",
            "SCRIPT_EXECUTE:id - 执行脚本",
            "SET_API_KEY:key - 设置 API Key"
        ),
        "usage_example" to "echo 'DEBUG_STATUS' | nc localhost 11451"
    )
    output.println(gson.toJson(help))
}

// ==================== 🧠 智能意图处理 ====================

/**
 * 🧠 智能执行：先分析意图，再决定走脚本还是聊天
 */
