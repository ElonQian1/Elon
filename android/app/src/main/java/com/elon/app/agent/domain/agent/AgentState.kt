// domain/agent/AgentState.kt
package com.elon.app.agent.domain.agent

/**
 * Agent 运行状态
 */
enum class AgentRunState {
    /** 空闲 */
    IDLE,
    /** 思考中（调用 AI） */
    THINKING,
    /** 执行中（调用工具） */
    EXECUTING,
    /** 观察中（获取屏幕状态） */
    OBSERVING,
    /** 暂停 */
    PAUSED,
    /** 等待人工确认 */
    WAITING_FOR_APPROVAL,
    /** 错误恢复中 */
    RECOVERING,
    /** 已停止 */
    STOPPED
}

/**
 * Agent 运行模式
 */
enum class AgentMode {
    /** 完全自主 */
    AUTONOMOUS,
    /** 半自主（高风险需确认） */
    SEMI_AUTONOMOUS,
    /** 监督模式（所有动作需确认） */
    SUPERVISED
}

/**
 * Agent 状态快照
 */
data class AgentSnapshot(
    val state: AgentRunState,
    val mode: AgentMode,
    val currentGoal: Goal?,
    val progress: Int,
    val lastAction: String?,
    val errorCount: Int
)
