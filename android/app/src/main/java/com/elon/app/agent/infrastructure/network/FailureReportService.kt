package com.elon.app.agent.infrastructure.network

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

/**
 * 失败案例上报服务
 * 
 * 当 AI 执行任务失败时，自动上报到云端服务器。
 * 这是"自我学习系统"的核心组件。
 * 
 * 上报的数据将被开发者通过 MCP 工具分析，
 * 用于改进下一版本的 AI 能力。
 */
object FailureReportService {
    
    private const val SERVER_URL = "http://119.91.19.232:8080"
    
    /**
     * 失败案例数据结构
     */
    data class FailureReport(
        val deviceId: String?,
        val userId: Int?,
        val appVersion: String,
        val taskGoal: String,              // 用户的原始目标
        val taskContext: JSONObject?,       // 任务执行时的上下文
        val generatedScript: JSONObject?,   // AI 生成的脚本
        val failureStep: Int?,              // 失败在第几步
        val failureReason: String?,         // 失败原因描述
        val errorType: String?,             // 错误类型分类
        val errorMessage: String?,          // 详细错误信息
        val screenXml: String?,             // 失败时的屏幕 XML
        val screenAnalysis: JSONObject?,    // AI 对屏幕的分析结果
        val targetApp: String?              // 目标应用包名
    )
    
    /**
     * 错误类型枚举
     */
    object ErrorTypes {
        const val ELEMENT_NOT_FOUND = "element_not_found"    // 找不到元素
        const val TIMEOUT = "timeout"                         // 操作超时
        const val NAVIGATION_ERROR = "navigation_error"       // 导航失败
        const val PERMISSION_DENIED = "permission_denied"     // 权限不足
        const val APP_NOT_FOUND = "app_not_found"            // 目标应用未安装
        const val CRASH = "crash"                            // 崩溃
        const val AI_ERROR = "ai_error"                      // AI 生成脚本失败
        const val UNKNOWN = "unknown"                        // 未知错误
    }
    
    /**
     * 上报失败案例
     * 
     * @return 上报结果，成功返回 failure_id，失败返回错误信息
     */
    suspend fun report(failure: FailureReport): Result<Int> = withContext(Dispatchers.IO) {
        try {
            val url = URL("$SERVER_URL/api/failures/report")
            val conn = url.openConnection() as HttpURLConnection
            
            conn.requestMethod = "POST"
            conn.setRequestProperty("Content-Type", "application/json")
            conn.doOutput = true
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            
            // 构建请求体
            val body = JSONObject().apply {
                putOpt("device_id", failure.deviceId)
                putOpt("user_id", failure.userId)
                put("app_version", failure.appVersion)
                put("task_goal", failure.taskGoal)
                putOpt("task_context", failure.taskContext)
                putOpt("generated_script", failure.generatedScript)
                putOpt("failure_step", failure.failureStep)
                putOpt("failure_reason", failure.failureReason)
                putOpt("error_type", failure.errorType)
                putOpt("error_message", failure.errorMessage)
                putOpt("screen_xml", failure.screenXml)
                putOpt("screen_analysis", failure.screenAnalysis)
                putOpt("target_app", failure.targetApp)
            }
            
            conn.outputStream.use { os ->
                os.write(body.toString().toByteArray(Charsets.UTF_8))
            }
            
            val responseCode = conn.responseCode
            
            if (responseCode == HttpURLConnection.HTTP_OK) {
                val response = conn.inputStream.bufferedReader().readText()
                val json = JSONObject(response)
                
                if (json.optBoolean("success", false)) {
                    val failureId = json.optInt("failure_id", -1)
                    android.util.Log.i("FailureReport", "上报成功: $failureId - ${json.optString("message")}")
                    Result.success(failureId)
                } else {
                    Result.failure(Exception("上报失败: ${json.optString("message")}"))
                }
            } else {
                val error = conn.errorStream?.bufferedReader()?.readText() ?: "Unknown error"
                Result.failure(Exception("HTTP $responseCode: $error"))
            }
        } catch (e: Exception) {
            android.util.Log.e("FailureReport", "上报异常", e)
            Result.failure(e)
        }
    }
    
    /**
     * 便捷方法：快速上报元素未找到错误
     */
    suspend fun reportElementNotFound(
        taskGoal: String,
        targetElement: String,
        screenXml: String?,
        targetApp: String?,
        appVersion: String,
        deviceId: String? = null
    ): Result<Int> {
        return report(FailureReport(
            deviceId = deviceId,
            userId = null,
            appVersion = appVersion,
            taskGoal = taskGoal,
            taskContext = null,
            generatedScript = null,
            failureStep = null,
            failureReason = "无法找到目标元素: $targetElement",
            errorType = ErrorTypes.ELEMENT_NOT_FOUND,
            errorMessage = "Element not found: $targetElement",
            screenXml = screenXml,
            screenAnalysis = null,
            targetApp = targetApp
        ))
    }
    
    /**
     * 便捷方法：快速上报超时错误
     */
    suspend fun reportTimeout(
        taskGoal: String,
        step: Int,
        stepDescription: String,
        targetApp: String?,
        appVersion: String,
        deviceId: String? = null
    ): Result<Int> {
        return report(FailureReport(
            deviceId = deviceId,
            userId = null,
            appVersion = appVersion,
            taskGoal = taskGoal,
            taskContext = null,
            generatedScript = null,
            failureStep = step,
            failureReason = "操作超时: $stepDescription",
            errorType = ErrorTypes.TIMEOUT,
            errorMessage = "Timeout at step $step: $stepDescription",
            screenXml = null,
            screenAnalysis = null,
            targetApp = targetApp
        ))
    }
    
    /**
     * 便捷方法：上报导航错误
     */
    suspend fun reportNavigationError(
        taskGoal: String,
        fromPage: String,
        toPage: String,
        targetApp: String?,
        appVersion: String,
        screenXml: String? = null,
        deviceId: String? = null
    ): Result<Int> {
        return report(FailureReport(
            deviceId = deviceId,
            userId = null,
            appVersion = appVersion,
            taskGoal = taskGoal,
            taskContext = null,
            generatedScript = null,
            failureStep = null,
            failureReason = "无法从 $fromPage 导航到 $toPage",
            errorType = ErrorTypes.NAVIGATION_ERROR,
            errorMessage = "Navigation failed: $fromPage -> $toPage",
            screenXml = screenXml,
            screenAnalysis = null,
            targetApp = targetApp
        ))
    }
    
    /**
     * 便捷方法：上报 AI 错误
     */
    suspend fun reportAIError(
        taskGoal: String,
        aiResponse: String?,
        errorMessage: String,
        targetApp: String?,
        appVersion: String,
        deviceId: String? = null
    ): Result<Int> {
        return report(FailureReport(
            deviceId = deviceId,
            userId = null,
            appVersion = appVersion,
            taskGoal = taskGoal,
            taskContext = JSONObject().apply {
                putOpt("ai_response", aiResponse)
            },
            generatedScript = null,
            failureStep = null,
            failureReason = "AI 处理失败: $errorMessage",
            errorType = ErrorTypes.AI_ERROR,
            errorMessage = errorMessage,
            screenXml = null,
            screenAnalysis = null,
            targetApp = targetApp
        ))
    }
    
    /**
     * 便捷方法：上报崩溃
     */
    suspend fun reportCrash(
        taskGoal: String,
        exception: Throwable,
        step: Int?,
        targetApp: String?,
        appVersion: String,
        screenXml: String? = null,
        deviceId: String? = null
    ): Result<Int> {
        val stackTrace = exception.stackTraceToString().take(2000) // 限制长度
        
        return report(FailureReport(
            deviceId = deviceId,
            userId = null,
            appVersion = appVersion,
            taskGoal = taskGoal,
            taskContext = null,
            generatedScript = null,
            failureStep = step,
            failureReason = "崩溃: ${exception.message}",
            errorType = ErrorTypes.CRASH,
            errorMessage = stackTrace,
            screenXml = screenXml,
            screenAnalysis = null,
            targetApp = targetApp
        ))
    }
    
    /**
     * 便捷方法：上报完整的执行失败（推荐使用）
     * 
     * 包含完整的执行上下文，方便分析
     */
    suspend fun reportExecutionFailure(
        taskGoal: String,
        generatedScript: JSONObject?,
        failureStep: Int,
        stepDescription: String,
        errorType: String,
        screenXml: String?,
        targetApp: String?,
        appVersion: String,
        additionalContext: Map<String, Any>? = null,
        deviceId: String? = null
    ): Result<Int> {
        return report(FailureReport(
            deviceId = deviceId,
            userId = null,
            appVersion = appVersion,
            taskGoal = taskGoal,
            taskContext = additionalContext?.let { JSONObject(it) },
            generatedScript = generatedScript,
            failureStep = failureStep,
            failureReason = stepDescription,
            errorType = errorType,
            errorMessage = "$errorType at step $failureStep: $stepDescription",
            screenXml = screenXml,
            screenAnalysis = null,
            targetApp = targetApp
        ))
    }
    
    /**
     * 📊 上报性能问题（用户主动提交）
     * 
     * 当任务成功但有性能问题时（如耗时过长、多次重试），
     * 用户可以选择提交此报告帮助改进。
     */
    suspend fun reportPerformanceIssue(
        taskGoal: String,
        success: Boolean,
        totalDurationMs: Long,
        totalRetries: Int,
        slowSteps: List<Int>,
        detailJson: String,
        recommendation: String?,
        appVersion: String = "1.0.0",
        deviceId: String? = null
    ): Result<Int> = withContext(Dispatchers.IO) {
        try {
            val url = URL("$SERVER_URL/api/failures/report")
            val conn = url.openConnection() as HttpURLConnection
            
            conn.requestMethod = "POST"
            conn.setRequestProperty("Content-Type", "application/json")
            conn.doOutput = true
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            
            // 构建请求体 - 使用特殊的 error_type 标识性能问题
            val body = JSONObject().apply {
                putOpt("device_id", deviceId)
                put("app_version", appVersion)
                put("task_goal", taskGoal)
                put("error_type", if (success) "performance_issue" else "execution_failure")
                put("failure_reason", buildPerformanceReason(success, totalDurationMs, totalRetries, slowSteps))
                put("error_message", recommendation ?: "用户主动提交的性能报告")
                
                // 将详细报告放在 task_context 中
                put("task_context", JSONObject().apply {
                    put("report_type", "user_submitted")
                    put("success", success)
                    put("total_duration_ms", totalDurationMs)
                    put("total_retries", totalRetries)
                    put("slow_steps", slowSteps.joinToString(","))
                    put("detail_report", detailJson)
                })
            }
            
            conn.outputStream.use { os ->
                os.write(body.toString().toByteArray(Charsets.UTF_8))
            }
            
            val responseCode = conn.responseCode
            
            if (responseCode == HttpURLConnection.HTTP_OK) {
                val response = conn.inputStream.bufferedReader().readText()
                val json = JSONObject(response)
                
                if (json.optBoolean("success", false)) {
                    val failureId = json.optInt("failure_id", -1)
                    android.util.Log.i("FailureReport", "性能报告上报成功: $failureId")
                    Result.success(failureId)
                } else {
                    Result.failure(Exception(json.optString("error", "未知错误")))
                }
            } else {
                val errorBody = conn.errorStream?.bufferedReader()?.readText() ?: "HTTP $responseCode"
                Result.failure(Exception(errorBody))
            }
        } catch (e: Exception) {
            android.util.Log.e("FailureReport", "性能报告上报失败", e)
            Result.failure(e)
        }
    }
    
    /**
     * 构建性能问题描述
     */
    private fun buildPerformanceReason(
        success: Boolean,
        totalDurationMs: Long,
        totalRetries: Int,
        slowSteps: List<Int>
    ): String {
        val reasons = mutableListOf<String>()
        
        if (!success) {
            reasons.add("任务失败")
        }
        
        if (totalDurationMs > 120000) {
            reasons.add("总耗时过长(${totalDurationMs/1000}秒)")
        }
        
        if (totalRetries > 2) {
            reasons.add("重试次数多(${totalRetries}次)")
        }
        
        if (slowSteps.isNotEmpty()) {
            reasons.add("慢步骤: ${slowSteps.joinToString(",") { "步骤$it" }}")
        }
        
        return if (reasons.isNotEmpty()) {
            reasons.joinToString("; ")
        } else {
            "用户主动提交的优化建议"
        }
    }
}
