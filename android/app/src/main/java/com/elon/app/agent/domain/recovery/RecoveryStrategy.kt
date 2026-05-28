// domain/recovery/RecoveryStrategy.kt
// module: domain/recovery | layer: domain | role: recovery-strategy
// summary: 错误恢复策略接口和常见实现

package com.elon.app.agent.domain.recovery

import com.elon.app.agent.domain.planning.ScreenContext

/**
 * 恢复策略接口
 */
interface RecoveryStrategy {
    /**
     * 策略名称
     */
    val name: String
    
    /**
     * 策略优先级（越小越优先）
     */
    val priority: Int
    
    /**
     * 判断策略是否适用于当前情况
     */
    fun isApplicable(context: RecoveryContext): Boolean
    
    /**
     * 执行恢复动作
     * @return 恢复结果
     */
    suspend fun recover(context: RecoveryContext): RecoveryResult
}

/**
 * 恢复上下文
 */
data class RecoveryContext(
    val errorType: ErrorType,
    val errorMessage: String?,
    val currentScreen: ScreenContext,
    val lastAction: LastAction?,
    val retryCount: Int = 0,
    val metadata: Map<String, Any> = emptyMap()
)

/**
 * 错误类型
 */
enum class ErrorType {
    ELEMENT_NOT_FOUND,      // 元素未找到
    ELEMENT_NOT_CLICKABLE,  // 元素不可点击
    UNEXPECTED_DIALOG,      // 意外弹窗
    APP_CRASH,              // 应用崩溃
    TIMEOUT,                // 操作超时
    SCREEN_CHANGED,         // 屏幕意外变化
    PERMISSION_DENIED,      // 权限被拒绝
    NETWORK_ERROR,          // 网络错误
    UNKNOWN                 // 未知错误
}

/**
 * 上一个动作
 */
data class LastAction(
    val toolName: String,
    val parameters: Map<String, Any>,
    val timestamp: Long
)

/**
 * 恢复结果
 */
sealed class RecoveryResult {
    /**
     * 恢复成功，可以继续执行
     */
    data class Success(
        val message: String,
        val shouldRetry: Boolean = false,  // 是否需要重试上一个动作
        val suggestedAction: SuggestedAction? = null
    ) : RecoveryResult()
    
    /**
     * 恢复失败，无法继续
     */
    data class Failure(
        val message: String,
        val fatal: Boolean = false  // 是否是致命错误
    ) : RecoveryResult()
    
    /**
     * 需要人工干预
     */
    data class NeedHumanIntervention(
        val reason: String,
        val instructions: String? = null
    ) : RecoveryResult()
}

/**
 * 建议的后续动作
 */
data class SuggestedAction(
    val toolName: String,
    val parameters: Map<String, Any>,
    val reason: String
)

/**
 * 恢复策略注册表
 */
class RecoveryStrategyRegistry {
    private val strategies = mutableListOf<RecoveryStrategy>()
    
    fun register(strategy: RecoveryStrategy) {
        strategies.add(strategy)
        strategies.sortBy { it.priority }
    }
    
    fun unregister(name: String) {
        strategies.removeAll { it.name == name }
    }
    
    /**
     * 获取适用的策略（按优先级排序）
     */
    fun getApplicableStrategies(context: RecoveryContext): List<RecoveryStrategy> {
        return strategies.filter { it.isApplicable(context) }
    }
    
    /**
     * 尝试所有适用策略直到成功
     */
    suspend fun tryRecover(context: RecoveryContext): RecoveryResult {
        val applicable = getApplicableStrategies(context)
        
        if (applicable.isEmpty()) {
            return RecoveryResult.Failure("没有适用的恢复策略")
        }
        
        for (strategy in applicable) {
            val result = strategy.recover(context)
            when (result) {
                is RecoveryResult.Success -> return result
                is RecoveryResult.NeedHumanIntervention -> return result
                is RecoveryResult.Failure -> {
                    if (result.fatal) return result
                    // 继续尝试下一个策略
                }
            }
        }
        
        return RecoveryResult.Failure("所有恢复策略都失败了")
    }
}
