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

internal fun SocketServer.handleSetScreenMode(modeName: String, output: PrintWriter) {
    try {
        val reader = getSmartScreenReader()
        if (reader == null) {
            output.println("""{"error":"NO_READER","message":"SmartScreenReader 未初始化"}""")
            return
        }
        
        val mode = com.elon.app.agent.domain.screen.ScreenCaptureMode.fromString(modeName)
        reader.setMode(mode)
        
        Log.i("Agent", "屏幕获取模式已切换为: ${mode.emoji} ${mode.displayName}")
        
        output.println(gson.toJson(mapOf(
            "success" to true,
            "mode" to mode.name,
            "display_name" to mode.displayName,
            "emoji" to mode.emoji,
            "description" to mode.description,
            "token_cost" to mode.tokenCost
        )))
    } catch (e: Exception) {
        Log.e("Agent", "SET_SCREEN_MODE 失败", e)
        output.println("""{"error":"SET_MODE_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取当前屏幕获取模式
 */
internal fun SocketServer.handleGetScreenMode(output: PrintWriter) {
    try {
        val reader = getSmartScreenReader()
        if (reader == null) {
            output.println("""{"error":"NO_READER","message":"SmartScreenReader 未初始化"}""")
            return
        }
        
        val mode = reader.currentMode
        output.println(gson.toJson(mapOf(
            "success" to true,
            "mode" to mode.name,
            "display_name" to mode.displayName,
            "emoji" to mode.emoji,
            "description" to mode.description,
            "token_cost" to mode.tokenCost
        )))
    } catch (e: Exception) {
        output.println("""{"error":"GET_MODE_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 列出所有屏幕获取模式
 */
internal fun SocketServer.handleListScreenModes(output: PrintWriter) {
    try {
        val reader = getSmartScreenReader()
        val currentMode = reader?.currentMode
        
        val modes = com.elon.app.agent.domain.screen.ScreenCaptureMode.values().map { mode ->
            mapOf(
                "name" to mode.name,
                "display_name" to mode.displayName,
                "emoji" to mode.emoji,
                "description" to mode.description,
                "token_cost" to mode.tokenCost,
                "is_current" to (mode == currentMode)
            )
        }
        
        output.println(gson.toJson(mapOf(
            "success" to true,
            "modes" to modes,
            "current" to (currentMode?.name ?: "FULL_DUMP")
        )))
    } catch (e: Exception) {
        output.println("""{"error":"LIST_MODES_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取屏幕差异（与基准快照对比）
 */
internal fun SocketServer.handleScreenDiff(output: PrintWriter) {
    try {
        val reader = getSmartScreenReader()
        if (reader == null) {
            output.println("""{"error":"NO_READER","message":"SmartScreenReader 未初始化"}""")
            return
        }
        
        val diff = reader.getDiffFromBaseline()
        val summary = reader.getDiffSummaryForAI()
        
        output.println(gson.toJson(mapOf(
            "success" to true,
            "has_changes" to diff.hasChanges,
            "summary" to diff.summary,
            "ai_summary" to summary,
            "added_count" to diff.addedNodes.size,
            "removed_count" to diff.removedNodes.size,
            "modified_count" to diff.modifiedNodes.size,
            "added_preview" to diff.addedNodes.take(5).map { it.text ?: it.className },
            "removed_preview" to diff.removedNodes.take(5).map { it.text ?: it.className }
        )))
    } catch (e: Exception) {
        Log.e("Agent", "SCREEN_DIFF 失败", e)
        output.println("""{"error":"DIFF_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取增量变化（自上次查询以来）
 */
internal fun SocketServer.handleScreenChanges(output: PrintWriter) {
    try {
        val reader = getSmartScreenReader()
        if (reader == null) {
            output.println("""{"error":"NO_READER","message":"SmartScreenReader 未初始化"}""")
            return
        }
        
        val changes = reader.getIncrementalChanges()
        val summary = reader.getChangesSummary()
        
        output.println(gson.toJson(mapOf(
            "success" to true,
            "has_changes" to changes.isNotEmpty(),
            "change_count" to changes.size,
            "summary" to summary,
            "changes" to changes.map { event ->
                mapOf(
                    "type" to event.eventType.name,
                    "timestamp" to event.timestamp,
                    "package" to event.packageName,
                    "description" to event.description,
                    "node_text" to event.changedNode?.text
                )
            }
        )))
    } catch (e: Exception) {
        Log.e("Agent", "SCREEN_CHANGES 失败", e)
        output.println("""{"error":"CHANGES_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 拍摄基准快照（用于 DIFF 模式）
 */
internal fun SocketServer.handleScreenSnapshot(output: PrintWriter) {
    try {
        val reader = getSmartScreenReader()
        if (reader == null) {
            output.println("""{"error":"NO_READER","message":"SmartScreenReader 未初始化"}""")
            return
        }
        
        reader.takeBaselineSnapshot()
        
        output.println(gson.toJson(mapOf(
            "success" to true,
            "message" to "基准快照已拍摄",
            "timestamp" to System.currentTimeMillis()
        )))
    } catch (e: Exception) {
        Log.e("Agent", "SCREEN_SNAPSHOT 失败", e)
        output.println("""{"error":"SNAPSHOT_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}

/**
 * 获取屏幕读取统计
 */
internal fun SocketServer.handleScreenStats(output: PrintWriter) {
    try {
        val reader = getSmartScreenReader()
        if (reader == null) {
            output.println("""{"error":"NO_READER","message":"SmartScreenReader 未初始化"}""")
            return
        }
        
        val stats = reader.getStats()
        output.println(stats.toJson())
    } catch (e: Exception) {
        Log.e("Agent", "SCREEN_STATS 失败", e)
        output.println("""{"error":"STATS_FAILED","message":"${escapeJson(e.message ?: "Unknown")}"}""")
    }
}
