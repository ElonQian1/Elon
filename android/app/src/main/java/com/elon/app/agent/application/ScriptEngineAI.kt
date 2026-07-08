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

// ===== [ScriptEngineAI.kt] =====
internal suspend fun ScriptEngine.verifyStepWithAI(step: ScriptStep, result: StepResult): AIVerifyResult {
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
internal fun ScriptEngine.getScreenStateForAI(): String {
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
internal suspend fun ScriptEngine.askAIForNextAction(
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
internal suspend fun ScriptEngine.executeCustomAIAction(decision: ScriptAIDecision): Boolean {
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

internal fun ScriptEngine.numberParam(params: Map<*, *>, key: String): Int? {
    return when (val value = params[key]) {
        is Number -> value.toInt()
        is String -> value.toIntOrNull()
        else -> null
    }
}