// domain/agent/AgentMemory.kt
package com.elon.app.agent.domain.agent

/**
 * Agent 记忆系统
 */
data class AgentMemory(
    /** 短期记忆：最近的动作和观察 */
    val shortTerm: MutableList<MemoryEntry> = mutableListOf(),
    
    /** 工作记忆：当前目标相关的上下文 */
    var workingMemory: WorkingMemory? = null,
    
    /** 长期记忆：学到的策略和经验 */
    val longTerm: MutableList<LearnedPattern> = mutableListOf()
) {
    fun addShortTerm(entry: MemoryEntry) {
        shortTerm.add(entry)
        if (shortTerm.size > 50) {
            shortTerm.removeAt(0)
        }
    }
    
    fun clearShortTerm() {
        shortTerm.clear()
    }
}

/**
 * 记忆条目
 */
data class MemoryEntry(
    val timestamp: Long,
    val type: MemoryType,
    val content: String
)

enum class MemoryType {
    ACTION,      // 执行的动作
    OBSERVATION, // 观察到的屏幕状态
    THOUGHT,     // AI 的思考过程
    RESULT       // 动作执行结果
}

/**
 * 工作记忆（当前任务上下文）
 */
data class WorkingMemory(
    val goal: Goal,
    val stepCount: Int = 0,
    val startTime: Long = System.currentTimeMillis()
)

/**
 * 学到的模式（长期记忆）
 */
data class LearnedPattern(
    val goalPattern: String,        // 目标模式（如"打开微信"）
    val successfulStrategy: List<AgentAction>,  // 成功的策略
    val successRate: Float,         // 成功率
    val timesUsed: Int              // 使用次数
)
