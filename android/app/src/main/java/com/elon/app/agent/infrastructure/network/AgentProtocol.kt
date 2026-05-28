// infrastructure/network/AgentProtocol.kt
// module: infrastructure/network | layer: infrastructure | role: protocol
// summary: PC-手机协同通信协议定义

package com.elon.app.agent.infrastructure.network

import com.elon.app.agent.domain.agent.AgentRunState
import com.elon.app.agent.domain.planning.ExecutionPlan
import com.elon.app.agent.domain.planning.Task
import com.elon.app.agent.domain.planning.TaskStatus
import com.google.gson.Gson

/**
 * Agent 通信协议
 * 
 * 消息类型：
 * - PC → 手机：goal, command, config, query
 * - 手机 → PC：status, progress, screen, result, error
 */
object AgentProtocol {
    private val gson = Gson()
    
    // ============ PC → 手机 消息类型 ============
    
    /**
     * 设置目标
     */
    fun goalMessage(
        description: String,
        maxSteps: Int = 20,
        timeoutSeconds: Int = 60
    ): OutgoingMessage = OutgoingMessage(
        type = "goal",
        payload = mapOf(
            "description" to description,
            "maxSteps" to maxSteps,
            "timeoutSeconds" to timeoutSeconds
        )
    )
    
    /**
     * 发送命令
     */
    fun commandMessage(
        command: AgentCommand,
        params: Map<String, Any> = emptyMap()
    ): OutgoingMessage = OutgoingMessage(
        type = "command",
        payload = mapOf(
            "command" to command.name,
            "params" to params
        )
    )
    
    /**
     * 查询状态
     */
    fun queryMessage(queryType: QueryType): OutgoingMessage = OutgoingMessage(
        type = "query",
        payload = mapOf("queryType" to queryType.name)
    )
    
    // ============ 手机 → PC 消息类型 ============
    
    /**
     * 状态更新
     */
    fun statusMessage(
        state: AgentRunState,
        currentGoal: String? = null,
        progress: Float = 0f
    ): OutgoingMessage = OutgoingMessage(
        type = "status",
        payload = mapOf(
            "state" to state.name,
            "currentGoal" to currentGoal,
            "progress" to progress
        )
    )
    
    /**
     * 进度更新
     */
    fun progressMessage(
        stepNumber: Int,
        totalSteps: Int,
        currentTask: String,
        taskStatus: TaskStatus
    ): OutgoingMessage = OutgoingMessage(
        type = "progress",
        payload = mapOf(
            "stepNumber" to stepNumber,
            "totalSteps" to totalSteps,
            "currentTask" to currentTask,
            "taskStatus" to taskStatus.name,
            "progressPercent" to (stepNumber.toFloat() / totalSteps * 100).toInt()
        )
    )
    
    /**
     * 屏幕状态
     */
    fun screenMessage(
        appPackage: String?,
        activity: String?,
        visibleTexts: List<String>,
        clickableElements: List<String>,
        screenshotBase64: String? = null
    ): OutgoingMessage = OutgoingMessage(
        type = "screen",
        payload = mapOf(
            "appPackage" to appPackage,
            "activity" to activity,
            "visibleTexts" to visibleTexts.take(50),
            "clickableElements" to clickableElements.take(50),
            "hasScreenshot" to (screenshotBase64 != null),
            "screenshot" to screenshotBase64  // 可选，可能很大
        )
    )
    
    /**
     * 执行结果
     */
    fun resultMessage(
        goalId: String,
        success: Boolean,
        stepsExecuted: Int,
        message: String,
        duration: Long
    ): OutgoingMessage = OutgoingMessage(
        type = "result",
        payload = mapOf(
            "goalId" to goalId,
            "success" to success,
            "stepsExecuted" to stepsExecuted,
            "message" to message,
            "durationMs" to duration
        )
    )
    
    /**
     * 错误消息
     */
    fun errorMessage(
        code: ErrorCode,
        message: String,
        details: String? = null
    ): OutgoingMessage = OutgoingMessage(
        type = "error",
        payload = mapOf(
            "code" to code.name,
            "message" to message,
            "details" to details
        )
    )
    
    /**
     * 日志消息
     */
    fun logMessage(
        level: LogLevel,
        message: String,
        tag: String = "Agent"
    ): OutgoingMessage = OutgoingMessage(
        type = "log",
        payload = mapOf(
            "level" to level.name,
            "tag" to tag,
            "message" to message
        )
    )
    
    /**
     * AI 思考过程
     */
    fun thinkingMessage(
        thought: String,
        decision: String?,
        action: String?
    ): OutgoingMessage = OutgoingMessage(
        type = "thinking",
        payload = mapOf(
            "thought" to thought,
            "decision" to decision,
            "action" to action
        )
    )
    
    /**
     * 计划更新
     */
    fun planMessage(plan: ExecutionPlan): OutgoingMessage = OutgoingMessage(
        type = "plan",
        payload = mapOf(
            "goalId" to plan.goal.id,
            "goalDescription" to plan.goal.description,
            "estimatedSteps" to plan.estimatedSteps,
            "progress" to plan.getProgress(),
            "tasks" to plan.flattenTasks().map { task ->
                mapOf(
                    "id" to task.id,
                    "description" to task.description,
                    "type" to task.type.name,
                    "status" to task.status.name
                )
            }
        )
    )
    
    // ============ 解析消息 ============
    
    /**
     * 解析目标消息
     */
    fun parseGoalMessage(payload: String): GoalPayload? {
        return try {
            gson.fromJson(payload, GoalPayload::class.java)
        } catch (e: Exception) {
            null
        }
    }
    
    /**
     * 解析命令消息
     */
    fun parseCommandMessage(payload: String): CommandPayload? {
        return try {
            gson.fromJson(payload, CommandPayload::class.java)
        } catch (e: Exception) {
            null
        }
    }
}

/**
 * Agent 命令
 */
enum class AgentCommand {
    START,          // 开始执行
    PAUSE,          // 暂停
    RESUME,         // 恢复
    STOP,           // 停止
    GET_STATUS,     // 获取状态
    GET_SCREEN,     // 获取屏幕
    SCREENSHOT,     // 截图
    TAP,            // 点击
    SWIPE,          // 滑动
    INPUT,          // 输入
    PRESS_KEY,      // 按键
    ANALYZE_SCREEN, // 智能分析屏幕（AI Agent）
    GENERATE_SCRIPT // 生成执行脚本（AI Agent）
}

/**
 * 查询类型
 */
enum class QueryType {
    STATUS,         // 当前状态
    SCREEN,         // 屏幕内容
    PLAN,           // 当前计划
    HISTORY,        // 执行历史
    STATS           // 统计数据
}

/**
 * 错误码
 */
enum class ErrorCode {
    UNKNOWN,
    INVALID_MESSAGE,
    GOAL_FAILED,
    TOOL_ERROR,
    AI_ERROR,
    PERMISSION_DENIED,
    TIMEOUT
}

/**
 * 日志级别
 */
enum class LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR
}

// ============ 消息载荷数据类 ============

data class GoalPayload(
    val description: String,
    val maxSteps: Int = 20,
    val timeoutSeconds: Int = 60
)

data class CommandPayload(
    val command: String,
    val params: Map<String, Any>? = null
)
