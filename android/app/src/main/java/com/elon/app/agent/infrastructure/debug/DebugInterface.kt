// infrastructure/debug/DebugInterface.kt
// module: debug | layer: infrastructure | role: debug-interface
// summary: 调试接口服务 - 为外部 AI (如 Copilot) 提供实时状态查询和调试能力

package com.elon.app.agent.infrastructure.debug

import android.accessibilityservice.AccessibilityService
import android.os.Build
import android.util.Log
import com.elon.app.agent.application.*
import com.elon.app.agent.domain.screen.UINode
import com.google.gson.GsonBuilder
import java.text.SimpleDateFormat
import java.util.*
import java.util.concurrent.ConcurrentLinkedDeque

/**
 * 🔧 调试接口服务
 * 
 * 为外部 AI 代理（如 VS Code Copilot）提供：
 * - 实时状态查询
 * - 错误追踪
 * - 执行历史
 * - UI 状态快照
 * - 性能指标
 */
class DebugInterface private constructor() {
    
    companion object {
        private const val TAG = "DebugInterface"
        private const val MAX_ERROR_HISTORY = 50
        private const val MAX_EXECUTION_HISTORY = 100
        private const val MAX_LOG_ENTRIES = 200
        
        @Volatile
        private var instance: DebugInterface? = null
        
        fun getInstance(): DebugInterface {
            return instance ?: synchronized(this) {
                instance ?: DebugInterface().also { instance = it }
            }
        }
        
        /**
         * 获取最近一次执行报告（静态便捷方法）
         */
        fun getLastExecutionReport(): ExecutionReport? {
            return getInstance().lastExecutionReport
        }
    }
    
    private val gson = GsonBuilder()
        .setPrettyPrinting()
        .disableHtmlEscaping()
        .create()
    
    private val dateFormat = SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.getDefault())
    
    // ==================== 状态存储 ====================
    
    /** 最近的错误记录 */
    private val errorHistory = ConcurrentLinkedDeque<ErrorRecord>()
    
    /** 执行历史记录 */
    private val executionHistory = ConcurrentLinkedDeque<ExecutionRecord>()
    
    /** 调试日志 */
    private val debugLogs = ConcurrentLinkedDeque<LogEntry>()
    
    /** 当前执行状态 */
    @Volatile
    var currentState: ExecutionState = ExecutionState.IDLE
        private set
    
    /** 当前任务信息 */
    @Volatile
    var currentTask: TaskInfo? = null
        private set
    
    /** 当前步骤信息 */
    @Volatile
    var currentStep: StepInfo? = null
        private set
    
    /** 服务启动时间 */
    private val startTime = System.currentTimeMillis()
    
    /** 统计计数器 */
    private var totalTasksExecuted = 0
    private var totalTasksSucceeded = 0
    private var totalTasksFailed = 0
    private var totalStepsExecuted = 0
    
    /** 📊 当前任务的步骤详情记录 */
    private val currentTaskStepDetails = mutableListOf<StepDetailRecord>()
    
    /** 当前步骤的重试详情 */
    private val currentStepRetries = mutableListOf<RetryDetail>()
    
    /** 当前任务清理的弹窗总数 */
    private var currentTaskPopupsDismissed = 0
    
    /** 当前任务 AI 介入次数 */
    private var currentTaskAiInterventions = 0
    
    /** 最近一次执行报告 */
    @Volatile
    var lastExecutionReport: ExecutionReport? = null
        private set
    
    // ==================== 数据类 ====================
    
    data class ErrorRecord(
        val timestamp: String,
        val timestampMs: Long,
        val type: String,
        val message: String,
        val stackTrace: String?,
        val context: Map<String, Any?>,
        val suggestion: String?
    )
    
    data class ExecutionRecord(
        val timestamp: String,
        val taskId: String,
        val taskName: String,
        val goal: String,
        val status: String,
        val durationMs: Long,
        val stepsTotal: Int,
        val stepsCompleted: Int,
        val error: String?
    )
    
    data class LogEntry(
        val timestamp: String,
        val level: String,
        val tag: String,
        val message: String
    )
    
    data class TaskInfo(
        val id: String,
        val name: String,
        val goal: String,
        val startTime: Long,
        val totalSteps: Int
    )
    
    data class StepInfo(
        val index: Int,
        val type: String,
        val description: String,
        val startTime: Long,
        val retryCount: Int
    )
    
    /**
     * 📊 步骤详情记录 - 记录每个步骤的完整执行过程
     */
    data class StepDetailRecord(
        val stepIndex: Int,
        val stepType: String,
        val description: String,
        val startTime: Long,
        val endTime: Long,
        val durationMs: Long,
        val success: Boolean,
        val retryCount: Int,
        val retryDetails: List<RetryDetail>,      // 每次重试的详情
        val aiRecoveryAttempted: Boolean,         // 是否触发了 AI 恢复
        val aiRecoverySuccess: Boolean,           // AI 恢复是否成功
        val aiRecoveryAction: String?,            // AI 恢复的动作
        val errorMessage: String?,                // 最终错误信息
        val performanceWarning: String?           // 性能警告（如耗时过长）
    )
    
    /**
     * 重试详情
     */
    data class RetryDetail(
        val attemptNumber: Int,
        val timestamp: Long,
        val reason: String,                       // 失败原因
        val durationMs: Long,                     // 这次尝试的耗时
        val popupsDismissed: Int                  // 清理的弹窗数量
    )
    
    /**
     * 📋 完整执行报告 - 用户可读的详细报告
     */
    data class ExecutionReport(
        val taskId: String,
        val taskName: String,
        val goal: String,
        val success: Boolean,
        val totalDurationMs: Long,
        val totalSteps: Int,
        val completedSteps: Int,
        val stepDetails: List<StepDetailRecord>,
        val summary: ReportSummary,
        val recommendation: String?,              // 优化建议
        val shouldReport: Boolean                 // 是否建议上报（有性能问题或异常）
    )
    
    /**
     * 报告摘要
     */
    data class ReportSummary(
        val totalRetries: Int,                    // 总重试次数
        val aiInterventions: Int,                 // AI 介入次数
        val popupsDismissed: Int,                 // 清理弹窗总数
        val slowSteps: List<Int>,                 // 慢步骤索引（>10秒）
        val failedSteps: List<Int>,               // 失败步骤索引
        val performanceScore: String              // 性能评分：GOOD/FAIR/POOR
    )
    
    enum class ExecutionState {
        IDLE,           // 空闲
        GENERATING,     // 正在生成脚本
        EXECUTING,      // 正在执行
        PAUSED,         // 已暂停
        IMPROVING,      // 正在改进脚本
        ERROR           // 出错
    }
    
    // ==================== 状态更新方法 ====================
    
    /**
     * 记录任务开始
     */
    fun onTaskStart(taskId: String, taskName: String, goal: String, totalSteps: Int) {
        currentState = ExecutionState.EXECUTING
        currentTask = TaskInfo(
            id = taskId,
            name = taskName,
            goal = goal,
            startTime = System.currentTimeMillis(),
            totalSteps = totalSteps
        )
        currentStep = null
        totalTasksExecuted++
        
        // 重置当前任务的详情记录
        currentTaskStepDetails.clear()
        currentStepRetries.clear()
        currentTaskPopupsDismissed = 0
        currentTaskAiInterventions = 0
        
        log("INFO", TAG, "📋 任务开始: $taskName (ID: $taskId)")
    }
    
    /**
     * 记录步骤开始
     */
    fun onStepStart(index: Int, type: String, description: String) {
        // 先保存上一个步骤的详情（如果有）
        saveCurrentStepDetail(success = true)
        
        currentStep = StepInfo(
            index = index,
            type = type,
            description = description,
            startTime = System.currentTimeMillis(),
            retryCount = 0
        )
        currentStepRetries.clear()
        totalStepsExecuted++
        
        log("DEBUG", TAG, "📍 步骤 $index: $description")
    }
    
    /**
     * 📊 记录步骤重试详情
     */
    fun onStepRetryDetail(
        index: Int, 
        attemptNumber: Int, 
        reason: String, 
        durationMs: Long,
        popupsDismissed: Int = 0
    ) {
        currentStep = currentStep?.copy(retryCount = attemptNumber)
        
        currentStepRetries.add(RetryDetail(
            attemptNumber = attemptNumber,
            timestamp = System.currentTimeMillis(),
            reason = reason,
            durationMs = durationMs,
            popupsDismissed = popupsDismissed
        ))
        
        currentTaskPopupsDismissed += popupsDismissed
        
        log("WARN", TAG, "🔄 步骤 $index 重试 #$attemptNumber: $reason (${durationMs}ms)")
    }
    
    /**
     * 记录步骤重试
     */
    fun onStepRetry(index: Int, retryCount: Int, reason: String) {
        currentStep = currentStep?.copy(retryCount = retryCount)
        log("WARN", TAG, "🔄 步骤 $index 重试 ($retryCount): $reason")
    }
    
    /**
     * 📊 记录 AI 恢复尝试
     */
    fun onAiRecoveryAttempt(stepIndex: Int, success: Boolean, action: String?) {
        currentTaskAiInterventions++
        log(
            if (success) "INFO" else "WARN", 
            TAG, 
            "${if (success) "✅" else "❌"} AI 恢复 ${if (success) "成功" else "失败"}: ${action ?: "无动作"}"
        )
    }

    /**
     * 记录步骤完成
     */
    fun onStepComplete(index: Int, success: Boolean, message: String? = null) {
        val step = currentStep ?: return
        val duration = System.currentTimeMillis() - step.startTime
        
        // 生成性能警告
        val perfWarning = when {
            duration > 60000 -> "⚠️ 严重: 步骤耗时超过1分钟 (${duration/1000}秒)"
            duration > 30000 -> "⚠️ 警告: 步骤耗时超过30秒 (${duration/1000}秒)"
            duration > 10000 && currentStepRetries.isNotEmpty() -> "⚠️ 注意: 步骤耗时较长，有重试"
            else -> null
        }
        
        // 保存步骤详情
        currentTaskStepDetails.add(StepDetailRecord(
            stepIndex = index,
            stepType = step.type,
            description = step.description,
            startTime = step.startTime,
            endTime = System.currentTimeMillis(),
            durationMs = duration,
            success = success,
            retryCount = currentStepRetries.size,
            retryDetails = currentStepRetries.toList(),
            aiRecoveryAttempted = currentStepRetries.size >= 3, // 如果重试3次以上，通常会触发AI
            aiRecoverySuccess = success && currentStepRetries.size >= 3,
            aiRecoveryAction = null, // 会在 onAiRecoveryAttempt 中更新
            errorMessage = if (!success) message else null,
            performanceWarning = perfWarning
        ))
        
        log(if (success) "INFO" else "WARN", TAG, 
            "✅ 步骤 $index ${if (success) "成功" else "失败"} (${duration}ms)${message?.let { ": $it" } ?: ""}")
    }
    
    /**
     * 保存当前步骤详情（内部方法）
     */
    private fun saveCurrentStepDetail(success: Boolean) {
        // 这个方法在步骤切换时调用，确保上一步的数据被保存
        // 实际保存在 onStepComplete 中完成
    }
    
    /**
     * 记录任务完成
     */
    fun onTaskComplete(success: Boolean, error: String? = null) {
        val task = currentTask ?: return
        val duration = System.currentTimeMillis() - task.startTime
        
        if (success) {
            totalTasksSucceeded++
        } else {
            totalTasksFailed++
        }
        
        // 添加到执行历史
        addExecutionRecord(ExecutionRecord(
            timestamp = dateFormat.format(Date()),
            taskId = task.id,
            taskName = task.name,
            goal = task.goal,
            status = if (success) "SUCCESS" else "FAILED",
            durationMs = duration,
            stepsTotal = task.totalSteps,
            stepsCompleted = currentStep?.index ?: 0,
            error = error
        ))
        
        // 📊 生成完整执行报告
        lastExecutionReport = generateExecutionReport(task, success, duration, error)
        
        currentState = if (success) ExecutionState.IDLE else ExecutionState.ERROR
        currentTask = null
        currentStep = null
        
        log(if (success) "INFO" else "ERROR", TAG, 
            "${if (success) "✅" else "❌"} 任务${if (success) "成功" else "失败"}: ${task.name} (${duration}ms)")
    }
    
    /**
     * 📊 生成完整执行报告
     */
    private fun generateExecutionReport(
        task: TaskInfo, 
        success: Boolean, 
        duration: Long, 
        error: String?
    ): ExecutionReport {
        val totalRetries = currentTaskStepDetails.sumOf { it.retryCount }
        val slowSteps = currentTaskStepDetails
            .filter { it.durationMs > 10000 }
            .map { it.stepIndex }
        val failedSteps = currentTaskStepDetails
            .filter { !it.success }
            .map { it.stepIndex }
        
        // 计算性能评分
        val perfScore = when {
            totalRetries == 0 && slowSteps.isEmpty() && success -> "GOOD"
            totalRetries <= 2 && slowSteps.size <= 1 && success -> "FAIR"
            else -> "POOR"
        }
        
        // 生成优化建议
        val recommendation = generateRecommendation(
            success, totalRetries, slowSteps, failedSteps, currentTaskStepDetails
        )
        
        // 判断是否建议上报
        val shouldReport = !success || 
                           totalRetries > 2 || 
                           slowSteps.isNotEmpty() || 
                           currentTaskAiInterventions > 0 ||
                           duration > 120000  // 超过2分钟
        
        return ExecutionReport(
            taskId = task.id,
            taskName = task.name,
            goal = task.goal,
            success = success,
            totalDurationMs = duration,
            totalSteps = task.totalSteps,
            completedSteps = currentTaskStepDetails.size,
            stepDetails = currentTaskStepDetails.toList(),
            summary = ReportSummary(
                totalRetries = totalRetries,
                aiInterventions = currentTaskAiInterventions,
                popupsDismissed = currentTaskPopupsDismissed,
                slowSteps = slowSteps,
                failedSteps = failedSteps,
                performanceScore = perfScore
            ),
            recommendation = recommendation,
            shouldReport = shouldReport
        )
    }
    
    /**
     * 生成优化建议
     */
    private fun generateRecommendation(
        success: Boolean,
        totalRetries: Int,
        slowSteps: List<Int>,
        failedSteps: List<Int>,
        stepDetails: List<StepDetailRecord>
    ): String? {
        val suggestions = mutableListOf<String>()
        
        if (!success) {
            suggestions.add("任务未能完成，建议上报以帮助改进")
        }
        
        if (totalRetries > 3) {
            val retryReasons = stepDetails
                .flatMap { it.retryDetails }
                .groupBy { it.reason }
                .maxByOrNull { it.value.size }
            if (retryReasons != null) {
                suggestions.add("多次重试，主要原因: ${retryReasons.key}")
            }
        }
        
        if (slowSteps.isNotEmpty()) {
            val slowestStep = stepDetails
                .filter { it.stepIndex in slowSteps }
                .maxByOrNull { it.durationMs }
            if (slowestStep != null) {
                suggestions.add("步骤 ${slowestStep.stepIndex} (${slowestStep.description}) 耗时 ${slowestStep.durationMs/1000} 秒，可能需要优化")
            }
        }
        
        return if (suggestions.isNotEmpty()) {
            suggestions.joinToString("\n")
        } else null
    }
    
    /**
     * 📋 获取用户可读的执行报告（用于悬浮窗显示）
     */
    fun getHumanReadableReport(): String {
        val report = lastExecutionReport ?: return "暂无执行报告"
        
        val sb = StringBuilder()
        sb.appendLine("═══════════════════════════════════")
        sb.appendLine("📋 执行报告")
        sb.appendLine("═══════════════════════════════════")
        sb.appendLine()
        sb.appendLine("🎯 任务: ${report.goal}")
        sb.appendLine("${if (report.success) "✅" else "❌"} 状态: ${if (report.success) "成功" else "失败"}")
        sb.appendLine("⏱️ 总耗时: ${formatDuration(report.totalDurationMs)}")
        sb.appendLine("📊 性能评分: ${report.summary.performanceScore}")
        sb.appendLine()
        sb.appendLine("───────────────────────────────────")
        sb.appendLine("📈 执行摘要")
        sb.appendLine("───────────────────────────────────")
        sb.appendLine("• 步骤完成: ${report.completedSteps}/${report.totalSteps}")
        sb.appendLine("• 重试次数: ${report.summary.totalRetries}")
        sb.appendLine("• AI介入: ${report.summary.aiInterventions} 次")
        sb.appendLine("• 弹窗清理: ${report.summary.popupsDismissed} 个")
        
        if (report.summary.slowSteps.isNotEmpty()) {
            sb.appendLine()
            sb.appendLine("⚠️ 慢步骤:")
            report.stepDetails
                .filter { it.stepIndex in report.summary.slowSteps }
                .forEach { step ->
                    sb.appendLine("  • 步骤${step.stepIndex}: ${step.description}")
                    sb.appendLine("    耗时: ${formatDuration(step.durationMs)}")
                    if (step.retryCount > 0) {
                        sb.appendLine("    重试: ${step.retryCount} 次")
                        step.retryDetails.forEach { retry ->
                            sb.appendLine("      - 原因: ${retry.reason}")
                        }
                    }
                }
        }
        
        if (report.recommendation != null) {
            sb.appendLine()
            sb.appendLine("───────────────────────────────────")
            sb.appendLine("💡 优化建议")
            sb.appendLine("───────────────────────────────────")
            sb.appendLine(report.recommendation)
        }
        
        if (report.shouldReport) {
            sb.appendLine()
            sb.appendLine("═══════════════════════════════════")
            sb.appendLine("📤 建议提交此报告帮助我们改进")
            sb.appendLine("═══════════════════════════════════")
        }
        
        return sb.toString()
    }
    
    /**
     * 格式化时长
     */
    private fun formatDuration(ms: Long): String {
        return when {
            ms < 1000 -> "${ms}毫秒"
            ms < 60000 -> "%.1f秒".format(ms / 1000.0)
            else -> "%.1f分钟".format(ms / 60000.0)
        }
    }
    
    /**
     * 📤 获取可上报的 JSON 格式报告
     */
    fun getReportForSubmission(): String {
        return gson.toJson(lastExecutionReport)
    }
    
    /**
     * 记录脚本生成中
     */
    fun onScriptGenerating(goal: String) {
        currentState = ExecutionState.GENERATING
        log("INFO", TAG, "🤖 正在为目标生成脚本: $goal")
    }
    
    /**
     * 记录脚本改进中
     */
    fun onScriptImproving(reason: String) {
        currentState = ExecutionState.IMPROVING
        log("INFO", TAG, "🔧 正在改进脚本: $reason")
    }
    
    /**
     * 记录错误
     */
    fun recordError(
        type: String,
        message: String,
        exception: Throwable? = null,
        context: Map<String, Any?> = emptyMap(),
        suggestion: String? = null
    ) {
        val record = ErrorRecord(
            timestamp = dateFormat.format(Date()),
            timestampMs = System.currentTimeMillis(),
            type = type,
            message = message,
            stackTrace = exception?.stackTraceToString()?.take(2000),
            context = context,
            suggestion = suggestion ?: generateSuggestion(type, message)
        )
        
        errorHistory.addFirst(record)
        while (errorHistory.size > MAX_ERROR_HISTORY) {
            errorHistory.removeLast()
        }
        
        Log.e(TAG, "❌ 错误记录: [$type] $message")
    }
    
    /**
     * 生成错误建议
     */
    private fun generateSuggestion(type: String, message: String): String {
        return when {
            message.contains("BLOCKED", ignoreCase = true) -> 
                "Android 11+ 包可见性限制。需要在 AndroidManifest.xml 添加 <queries> 声明目标应用包名"
            message.contains("No root window", ignoreCase = true) ->
                "无法获取 UI 树。请确保无障碍服务已开启且目标应用在前台"
            message.contains("timeout", ignoreCase = true) ->
                "操作超时。可能是网络问题或目标元素加载慢，尝试增加等待时间"
            message.contains("not found", ignoreCase = true) ->
                "元素未找到。检查选择器是否正确，或元素是否需要滚动才能显示"
            message.contains("API", ignoreCase = true) ->
                "API 调用失败。检查 API Key 是否有效，网络是否正常"
            message.contains("JSON", ignoreCase = true) ->
                "JSON 解析错误。AI 返回的格式可能不正确，可能需要调整 prompt"
            else -> "请检查日志获取更多详情"
        }
    }
    
    // ==================== 查询方法 ====================
    
    /**
     * 获取完整状态（供外部 AI 调用）
     */
    fun getFullStatus(service: AccessibilityService? = null, scriptEngine: ScriptEngine? = null): String {
        val status = mutableMapOf<String, Any?>()
        
        // 基本状态
        status["timestamp"] = dateFormat.format(Date())
        status["state"] = currentState.name
        status["uptime_ms"] = System.currentTimeMillis() - startTime
        
        // 当前任务
        currentTask?.let { task ->
            status["current_task"] = mapOf(
                "id" to task.id,
                "name" to task.name,
                "goal" to task.goal,
                "elapsed_ms" to (System.currentTimeMillis() - task.startTime),
                "total_steps" to task.totalSteps
            )
        }
        
        // 当前步骤
        currentStep?.let { step ->
            status["current_step"] = mapOf(
                "index" to step.index,
                "type" to step.type,
                "description" to step.description,
                "elapsed_ms" to (System.currentTimeMillis() - step.startTime),
                "retry_count" to step.retryCount
            )
        }
        
        // 统计信息
        status["statistics"] = mapOf(
            "total_tasks" to totalTasksExecuted,
            "succeeded" to totalTasksSucceeded,
            "failed" to totalTasksFailed,
            "success_rate" to if (totalTasksExecuted > 0) 
                "%.1f%%".format(totalTasksSucceeded * 100.0 / totalTasksExecuted) else "N/A",
            "total_steps" to totalStepsExecuted
        )
        
        // 最近错误
        status["last_error"] = errorHistory.firstOrNull()
        status["error_count"] = errorHistory.size
        
        // 脚本引擎状态
        scriptEngine?.let { engine ->
            val scripts = engine.listScripts()
            status["script_engine"] = mapOf(
                "available" to true,
                "script_count" to scripts.size,
                "scripts" to scripts.take(10).map { s ->
                    mapOf(
                        "id" to s.id,
                        "name" to s.name,
                        "success_count" to s.successCount,
                        "fail_count" to s.failCount
                    )
                }
            )
        } ?: run {
            status["script_engine"] = mapOf("available" to false, "reason" to "未初始化，需要设置 API Key")
        }
        
        // 设备信息
        status["device"] = mapOf(
            "model" to Build.MODEL,
            "sdk" to Build.VERSION.SDK_INT,
            "manufacturer" to Build.MANUFACTURER
        )
        
        // 屏幕状态
        service?.let { svc ->
            val root = svc.rootInActiveWindow
            status["screen"] = if (root != null) {
                mapOf(
                    "available" to true,
                    "package" to (root.packageName?.toString() ?: "unknown"),
                    "window_count" to svc.windows?.size
                )
            } else {
                mapOf("available" to false, "reason" to "无法获取 root window")
            }
        }
        
        return gson.toJson(status)
    }
    
    /**
     * 获取最后一个错误（简化版）
     */
    fun getLastError(): String {
        val error = errorHistory.firstOrNull()
        return if (error != null) {
            gson.toJson(mapOf(
                "has_error" to true,
                "error" to error
            ))
        } else {
            """{"has_error":false,"message":"没有错误记录"}"""
        }
    }
    
    /**
     * 获取错误历史
     */
    fun getErrorHistory(limit: Int = 10): String {
        return gson.toJson(mapOf(
            "count" to errorHistory.size,
            "errors" to errorHistory.take(limit)
        ))
    }
    
    /**
     * 获取执行历史
     */
    fun getExecutionHistory(limit: Int = 20): String {
        return gson.toJson(mapOf(
            "count" to executionHistory.size,
            "executions" to executionHistory.take(limit)
        ))
    }
    
    /**
     * 获取最近日志
     */
    fun getRecentLogs(limit: Int = 50): String {
        return gson.toJson(mapOf(
            "count" to debugLogs.size,
            "logs" to debugLogs.take(limit)
        ))
    }
    
    /**
     * 获取健康检查结果
     */
    fun getHealthCheck(service: AccessibilityService?, scriptEngine: ScriptEngine?): String {
        val checks = mutableListOf<Map<String, Any>>()
        
        // 检查无障碍服务
        checks.add(mapOf(
            "name" to "accessibility_service",
            "status" to if (service != null) "OK" else "FAIL",
            "message" to if (service != null) "无障碍服务运行中" else "无障碍服务未运行"
        ))
        
        // 检查 Root Window
        val hasRoot = service?.rootInActiveWindow != null
        checks.add(mapOf(
            "name" to "root_window",
            "status" to if (hasRoot) "OK" else "WARN",
            "message" to if (hasRoot) "可获取 UI 树" else "无法获取 UI 树，可能需要切换到目标应用"
        ))
        
        // 检查脚本引擎
        checks.add(mapOf(
            "name" to "script_engine",
            "status" to if (scriptEngine != null) "OK" else "WARN",
            "message" to if (scriptEngine != null) "脚本引擎已就绪" else "脚本引擎未初始化，需要设置 API Key"
        ))
        
        // 检查错误状态
        val recentErrors = errorHistory.count { 
            System.currentTimeMillis() - it.timestampMs < 60000 
        }
        checks.add(mapOf(
            "name" to "error_rate",
            "status" to when {
                recentErrors == 0 -> "OK"
                recentErrors < 3 -> "WARN"
                else -> "FAIL"
            },
            "message" to "最近1分钟 $recentErrors 个错误"
        ))
        
        val allOk = checks.all { it["status"] == "OK" }
        
        return gson.toJson(mapOf(
            "healthy" to allOk,
            "timestamp" to dateFormat.format(Date()),
            "checks" to checks
        ))
    }
    
    // ==================== 辅助方法 ====================
    
    private fun log(level: String, tag: String, message: String) {
        debugLogs.addFirst(LogEntry(
            timestamp = dateFormat.format(Date()),
            level = level,
            tag = tag,
            message = message
        ))
        while (debugLogs.size > MAX_LOG_ENTRIES) {
            debugLogs.removeLast()
        }
        
        // 同时输出到 Android Log
        when (level) {
            "DEBUG" -> Log.d(tag, message)
            "INFO" -> Log.i(tag, message)
            "WARN" -> Log.w(tag, message)
            "ERROR" -> Log.e(tag, message)
        }
    }
    
    private fun addExecutionRecord(record: ExecutionRecord) {
        executionHistory.addFirst(record)
        while (executionHistory.size > MAX_EXECUTION_HISTORY) {
            executionHistory.removeLast()
        }
    }
    
    /**
     * 清除所有历史记录
     */
    fun clearHistory() {
        errorHistory.clear()
        executionHistory.clear()
        debugLogs.clear()
        log("INFO", TAG, "🧹 历史记录已清除")
    }
    
    /**
     * 重置统计计数
     */
    fun resetStatistics() {
        totalTasksExecuted = 0
        totalTasksSucceeded = 0
        totalTasksFailed = 0
        totalStepsExecuted = 0
        log("INFO", TAG, "📊 统计数据已重置")
    }
}
