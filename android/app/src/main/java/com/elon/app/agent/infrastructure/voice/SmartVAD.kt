// infrastructure/voice/SmartVAD.kt
// module: infrastructure/voice | layer: infrastructure | role: smart-vad
// summary: 智能端点检测 - 结合语义完整性判断用户是否说完

package com.elon.app.agent.infrastructure.voice

import android.util.Log

/**
 * 🎯 智能 VAD (Voice Activity Detection)
 * 
 * 传统 VAD 只检测静音，本实现结合：
 * 1. 静音时长
 * 2. 语义完整性
 * 3. 句子结构
 * 
 * 目标：减少不必要的等待，快速响应完整语句
 */
class SmartVAD {
    
    companion object {
        private const val TAG = "SmartVAD"
        
        // 静音阈值配置 (毫秒)
        const val INSTANT_COMPLETE_SILENCE = 300L    // 高置信度完整 + 短静音 → 立即结束
        const val FAST_COMPLETE_SILENCE = 600L       // 中置信度完整 + 中静音 → 结束
        const val NORMAL_COMPLETE_SILENCE = 1000L    // 低置信度 + 一般静音 → 结束
        const val MAX_SILENCE = 2000L                // 最长等待
        const val INCOMPLETE_MIN_SILENCE = 1500L     // 不完整时至少等这么久
    }
    
    // ==================== 语义完整性规则 ====================
    
    /** 完整的操作模式 */
    private val completePatterns = listOf(
        // 打开类
        Regex("打开.+"),
        Regex("启动.+"),
        Regex("运行.+"),
        
        // 搜索类
        Regex("搜索.+"),
        Regex("查找.+"),
        Regex("找.+"),
        
        // 发送类
        Regex("发送?.+给.+"),
        Regex("给.+发.+"),
        
        // 设置类
        Regex("设置.+为.+"),
        Regex("把.+设置成.+")
    )
    
    /** 不完整的结尾词 */
    private val incompleteEndings = listOf(
        "然后", "接着", "再", "还有", "和", "的", "把", "将", "给", "帮我"
    )
    
    /** 完整的结尾词 */
    private val completeEndings = listOf(
        "吧", "呢", "啊", "了", "好", "行", "可以"
    )
    
    /** 简单问候（可立即响应） */
    private val instantTriggers = listOf(
        "你好", "您好", "嗨", "hi", "hello", "谢谢", "再见", "拜拜"
    )
    
    /** 操作关键词 */
    private val operationKeywords = listOf(
        "打开", "启动", "搜索", "查找", "点击", "发送", "获取", "设置"
    )
    
    /** APP 名称 */
    private val appNames = listOf(
        "微信", "淘宝", "京东", "抖音", "小红书", "支付宝",
        "qq", "微博", "美团", "饿了么", "高德", "百度"
    )
    
    // ==================== 公开方法 ====================
    
    /**
     * 检查语义完整性
     * 
     * @return 完整性结果
     */
    fun checkCompleteness(text: String): CompletenessResult {
        val normalized = text.trim().lowercase()
        
        // 1. 空输入
        if (normalized.isEmpty()) {
            return CompletenessResult(
                isComplete = false,
                confidence = 0f,
                reason = "空输入"
            )
        }
        
        // 2. 即时触发词（最高优先级）
        if (instantTriggers.any { normalized == it }) {
            return CompletenessResult(
                isComplete = true,
                confidence = 1.0f,
                reason = "即时触发词",
                suggestedSilenceMs = INSTANT_COMPLETE_SILENCE
            )
        }
        
        // 3. 明确不完整的结尾
        if (incompleteEndings.any { normalized.endsWith(it) }) {
            return CompletenessResult(
                isComplete = false,
                confidence = 0.9f,
                reason = "不完整结尾: ${incompleteEndings.find { normalized.endsWith(it) }}",
                suggestedSilenceMs = INCOMPLETE_MIN_SILENCE
            )
        }
        
        // 4. 操作词 + APP名 = 高置信度完整
        val hasOperation = operationKeywords.any { normalized.contains(it) }
        val hasApp = appNames.any { normalized.contains(it) }
        
        if (hasOperation && hasApp) {
            return CompletenessResult(
                isComplete = true,
                confidence = 0.95f,
                reason = "操作词+APP名",
                suggestedSilenceMs = INSTANT_COMPLETE_SILENCE
            )
        }
        
        // 5. 单独APP名（隐含"打开"）
        if (hasApp && !hasOperation && text.length <= 10) {
            return CompletenessResult(
                isComplete = true,
                confidence = 0.85f,
                reason = "单独APP名",
                suggestedSilenceMs = FAST_COMPLETE_SILENCE
            )
        }
        
        // 6. 只有操作词，没目标
        if (hasOperation && !hasApp && text.length < 6) {
            return CompletenessResult(
                isComplete = false,
                confidence = 0.8f,
                reason = "操作词无目标",
                suggestedSilenceMs = INCOMPLETE_MIN_SILENCE
            )
        }
        
        // 7. 匹配完整模式
        if (completePatterns.any { it.matches(normalized) }) {
            return CompletenessResult(
                isComplete = true,
                confidence = 0.9f,
                reason = "匹配完整模式",
                suggestedSilenceMs = FAST_COMPLETE_SILENCE
            )
        }
        
        // 8. 完整结尾词
        if (completeEndings.any { normalized.endsWith(it) }) {
            return CompletenessResult(
                isComplete = true,
                confidence = 0.8f,
                reason = "完整结尾词",
                suggestedSilenceMs = FAST_COMPLETE_SILENCE
            )
        }
        
        // 9. 较长句子（8字以上）且有操作词
        if (text.length >= 8 && hasOperation) {
            return CompletenessResult(
                isComplete = true,
                confidence = 0.75f,
                reason = "长句+操作词",
                suggestedSilenceMs = NORMAL_COMPLETE_SILENCE
            )
        }
        
        // 10. 默认：不确定
        return CompletenessResult(
            isComplete = false,
            confidence = 0.5f,
            reason = "无法确定",
            suggestedSilenceMs = NORMAL_COMPLETE_SILENCE
        )
    }
    
    /**
     * 综合判断是否应该结束输入
     * 
     * @param currentText 当前文本
     * @param silenceDuration 静音时长(ms)
     * @return 是否应该结束
     */
    fun shouldEndInput(currentText: String, silenceDuration: Long): EndInputDecision {
        val completeness = checkCompleteness(currentText)
        
        val shouldEnd = when {
            // 高置信度完整 + 短静音 → 立即结束
            completeness.isComplete && completeness.confidence > 0.9 && 
                silenceDuration >= INSTANT_COMPLETE_SILENCE -> true
            
            // 中置信度完整 + 中静音 → 结束
            completeness.isComplete && completeness.confidence > 0.7 && 
                silenceDuration >= FAST_COMPLETE_SILENCE -> true
            
            // 一般完整 + 正常静音 → 结束
            completeness.isComplete && silenceDuration >= NORMAL_COMPLETE_SILENCE -> true
            
            // 不完整但静音太久 → 也结束
            silenceDuration >= MAX_SILENCE -> true
            
            // 明确不完整且静音不够 → 继续
            !completeness.isComplete && silenceDuration < completeness.suggestedSilenceMs -> false
            
            // 超过建议静音时间 → 结束
            silenceDuration >= completeness.suggestedSilenceMs -> true
            
            else -> false
        }
        
        Log.d(TAG, "VAD决策: text='$currentText', silence=${silenceDuration}ms, " +
                "complete=${completeness.isComplete}, confidence=${completeness.confidence}, " +
                "decision=$shouldEnd")
        
        return EndInputDecision(
            shouldEnd = shouldEnd,
            completeness = completeness,
            silenceDuration = silenceDuration
        )
    }
}

/**
 * 完整性检查结果
 */
data class CompletenessResult(
    /** 是否完整 */
    val isComplete: Boolean,
    
    /** 置信度 (0-1) */
    val confidence: Float,
    
    /** 判断原因 */
    val reason: String,
    
    /** 建议的静音等待时间 */
    val suggestedSilenceMs: Long = SmartVAD.NORMAL_COMPLETE_SILENCE
)

/**
 * 结束输入决策
 */
data class EndInputDecision(
    /** 是否应该结束 */
    val shouldEnd: Boolean,
    
    /** 完整性结果 */
    val completeness: CompletenessResult,
    
    /** 当前静音时长 */
    val silenceDuration: Long
)
