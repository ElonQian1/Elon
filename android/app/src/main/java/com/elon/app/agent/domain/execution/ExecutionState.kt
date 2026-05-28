// domain/execution/ExecutionState.kt
// module: domain/execution | layer: domain | role: execution-state
// summary: 执行状态枚举和数据类 - 用于悬浮窗实时显示执行进度

package com.elon.app.agent.domain.execution

/**
 * 执行状态枚举
 */
enum class ExecutionState {
    /** 空闲状态，无任务执行 */
    IDLE,
    
    /** 正在执行任务 */
    EXECUTING,
    
    /** 正在停止（优雅中断中） */
    STOPPING,
    
    /** 已停止（用户手动停止） */
    STOPPED,
    
    /** 执行成功完成 */
    SUCCESS,
    
    /** 执行失败 */
    FAILED
}

/**
 * 执行信息数据类
 * 
 * 包含当前执行的完整状态信息
 */
data class ExecutionInfo(
    /** 当前状态 */
    val state: ExecutionState = ExecutionState.IDLE,
    
    /** 任务目标描述 */
    val taskGoal: String = "",
    
    /** 当前步骤序号（从 1 开始） */
    val currentStep: Int = 0,
    
    /** 总步骤数 */
    val totalSteps: Int = 0,
    
    /** 当前步骤名称 */
    val stepName: String = "",
    
    /** 执行开始时间（毫秒时间戳） */
    val startTime: Long = 0L,
    
    /** 执行结果消息（成功/失败时填写） */
    val resultMessage: String = ""
) {
    /**
     * 执行进度百分比（0-100）
     */
    val progressPercent: Int
        get() = if (totalSteps > 0) (currentStep * 100 / totalSteps) else 0
    
    /**
     * 已执行时长（毫秒）
     */
    val elapsedTime: Long
        get() = if (startTime > 0) System.currentTimeMillis() - startTime else 0
    
    /**
     * 格式化的进度文本
     */
    val progressText: String
        get() = when (state) {
            ExecutionState.IDLE -> "等待执行"
            ExecutionState.EXECUTING -> "步骤 $currentStep/$totalSteps: $stepName"
            ExecutionState.STOPPING -> "正在停止..."
            ExecutionState.STOPPED -> "已停止"
            ExecutionState.SUCCESS -> "执行完成 ✓"
            ExecutionState.FAILED -> "执行失败"
        }
    
    /**
     * 是否正在运行（执行中或停止中）
     */
    val isRunning: Boolean
        get() = state == ExecutionState.EXECUTING || state == ExecutionState.STOPPING
    
    /**
     * 是否可以停止
     */
    val canStop: Boolean
        get() = state == ExecutionState.EXECUTING
}

/**
 * 执行日志条目
 */
data class ExecutionLogEntry(
    /** 时间戳 */
    val timestamp: Long = System.currentTimeMillis(),
    
    /** 日志消息 */
    val message: String,
    
    /** 日志级别 */
    val level: LogLevel = LogLevel.INFO
) {
    enum class LogLevel {
        DEBUG, INFO, WARNING, ERROR
    }
    
    /**
     * 格式化的时间文本 (HH:mm:ss)
     */
    val timeText: String
        get() {
            val sdf = java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.getDefault())
            return sdf.format(java.util.Date(timestamp))
        }
    
    /**
     * 格式化的完整日志文本
     */
    val formattedText: String
        get() = "[$timeText] $message"
}
