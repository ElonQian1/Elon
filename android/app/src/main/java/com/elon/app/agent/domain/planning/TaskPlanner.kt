// domain/planning/TaskPlanner.kt
// module: domain/planning | layer: domain | role: task-planner
// summary: 层次化任务分解器，将复杂目标拆解为可执行的子任务

package com.elon.app.agent.domain.planning

import com.elon.app.agent.domain.agent.Goal

/**
 * 任务规划器
 * 
 * 采用层次任务分析 (HTA - Hierarchical Task Analysis)
 * 将复杂目标分解为可执行的子任务树
 */
interface TaskPlanner {
    /**
     * 分解目标为执行计划
     */
    suspend fun plan(goal: Goal, context: PlanningContext): ExecutionPlan
    
    /**
     * 根据执行反馈调整计划
     */
    suspend fun replan(
        originalPlan: ExecutionPlan,
        feedback: ExecutionFeedback
    ): ExecutionPlan
}

/**
 * 规划上下文
 */
data class PlanningContext(
    val currentScreen: ScreenContext,
    val recentHistory: List<String> = emptyList(),
    val availableApps: List<String> = emptyList(),
    val learnedStrategies: List<LearnedStrategy> = emptyList(),
    val constraints: PlanningConstraints = PlanningConstraints()
)

/**
 * 屏幕上下文
 */
data class ScreenContext(
    val appPackage: String?,
    val activityName: String?,
    val visibleTexts: List<String>,
    val clickableElements: List<String>,
    val hasDialog: Boolean = false,
    val summary: String = ""
)

/**
 * 规划约束
 */
data class PlanningConstraints(
    val maxSubtasks: Int = 10,
    val maxDepth: Int = 3,
    val timeout: Long = 60_000,
    val avoidApps: List<String> = emptyList()
)

/**
 * 学习到的策略
 */
data class LearnedStrategy(
    val pattern: String,
    val steps: List<String>,
    val confidence: Float
)

/**
 * 执行计划
 */
data class ExecutionPlan(
    val id: String = java.util.UUID.randomUUID().toString(),
    val goal: Goal,
    val rootTask: Task,
    val estimatedSteps: Int,
    val createdAt: Long = System.currentTimeMillis()
) {
    /**
     * 获取扁平化的任务列表（深度优先）
     */
    fun flattenTasks(): List<Task> {
        val result = mutableListOf<Task>()
        fun dfs(task: Task) {
            result.add(task)
            task.subtasks.forEach { dfs(it) }
        }
        dfs(rootTask)
        return result
    }
    
    /**
     * 获取下一个待执行的叶子任务
     */
    fun getNextLeafTask(): Task? {
        fun findLeaf(task: Task): Task? {
            if (task.status == TaskStatus.COMPLETED || task.status == TaskStatus.SKIPPED) {
                return null
            }
            if (task.subtasks.isEmpty()) {
                return if (task.status == TaskStatus.PENDING) task else null
            }
            for (subtask in task.subtasks) {
                val leaf = findLeaf(subtask)
                if (leaf != null) return leaf
            }
            return null
        }
        return findLeaf(rootTask)
    }
    
    /**
     * 获取当前进度
     */
    fun getProgress(): Float {
        val all = flattenTasks()
        val completed = all.count { it.status == TaskStatus.COMPLETED || it.status == TaskStatus.SKIPPED }
        return if (all.isEmpty()) 1f else completed.toFloat() / all.size
    }
    
    /**
     * 检查是否完成
     */
    fun isComplete(): Boolean = rootTask.status == TaskStatus.COMPLETED
}

/**
 * 任务节点
 */
data class Task(
    val id: String = java.util.UUID.randomUUID().toString(),
    val description: String,
    val type: TaskType,
    val priority: Int = 0,
    val subtasks: MutableList<Task> = mutableListOf(),
    var status: TaskStatus = TaskStatus.PENDING,
    
    // 执行相关
    val action: TaskAction? = null,
    val precondition: TaskCondition? = null,
    val postcondition: TaskCondition? = null,
    
    // 元数据
    val metadata: Map<String, Any> = emptyMap()
) {
    val isLeaf: Boolean get() = subtasks.isEmpty()
    
    /**
     * 添加子任务
     */
    fun addSubtask(task: Task): Task {
        subtasks.add(task)
        return this
    }
    
    /**
     * 标记完成
     */
    fun markCompleted() {
        status = TaskStatus.COMPLETED
    }
    
    /**
     * 标记失败
     */
    fun markFailed(reason: String? = null) {
        status = TaskStatus.FAILED
    }
}

/**
 * 任务类型
 */
enum class TaskType {
    COMPOSITE,      // 复合任务（有子任务）
    NAVIGATION,     // 导航任务（打开 App、页面跳转）
    INTERACTION,    // 交互任务（点击、输入）
    VERIFICATION,   // 验证任务（检查条件）
    WAIT,           // 等待任务
    RECOVERY        // 恢复任务（处理异常）
}

/**
 * 任务状态
 */
enum class TaskStatus {
    PENDING,        // 待执行
    IN_PROGRESS,    // 执行中
    COMPLETED,      // 已完成
    FAILED,         // 失败
    SKIPPED,        // 跳过
    BLOCKED         // 阻塞
}

/**
 * 任务动作
 */
data class TaskAction(
    val toolName: String,
    val parameters: Map<String, Any> = emptyMap(),
    val description: String = ""
)

/**
 * 任务条件
 */
data class TaskCondition(
    val type: ConditionType,
    val expression: String,
    val timeout: Long = 5000
)

enum class ConditionType {
    SCREEN_CONTAINS,        // 屏幕包含某文本
    SCREEN_NOT_CONTAINS,    // 屏幕不包含某文本
    ELEMENT_EXISTS,         // 元素存在
    ELEMENT_CLICKABLE,      // 元素可点击
    APP_IN_FOREGROUND,      // App 在前台
    CUSTOM                  // 自定义
}

/**
 * 执行反馈
 */
data class ExecutionFeedback(
    val taskId: String,
    val success: Boolean,
    val error: String? = null,
    val currentScreen: ScreenContext,
    val suggestedAction: String? = null
)
