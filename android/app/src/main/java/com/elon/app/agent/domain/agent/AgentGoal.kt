// domain/agent/AgentGoal.kt
package com.elon.app.agent.domain.agent

import java.util.UUID

/**
 * Agent 执行目标
 */
data class Goal(
    val id: String = UUID.randomUUID().toString(),
    val description: String,
    val completionCondition: CompletionCondition,
    val maxSteps: Int = 50,
    val timeoutSeconds: Long = 300,
    val targetApp: String? = null  // 目标应用包名，用于失败上报
)

/**
 * 目标完成条件
 */
sealed class CompletionCondition {
    /** UI 出现特定元素 */
    data class ElementAppears(val text: String) : CompletionCondition()
    
    /** UI 不再包含特定元素 */
    data class ElementDisappears(val text: String) : CompletionCondition()
    
    /** 到达特定页面 */
    data class ReachPage(val packageName: String, val activityName: String) : CompletionCondition()
    
    /** AI 判断完成 */
    object AIDecided : CompletionCondition()
    
    /** 自定义条件 */
    data class Custom(val predicate: (ScreenSnapshot) -> Boolean) : CompletionCondition()
}

/**
 * 屏幕快照（简化版，完整定义在 domain/screen/）
 */
data class ScreenSnapshot(
    val timestamp: Long,
    val packageName: String?,
    val activityName: String?,
    val elements: List<String>  // 简化：只存文本
)
