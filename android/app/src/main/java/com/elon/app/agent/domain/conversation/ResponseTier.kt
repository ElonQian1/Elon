// domain/conversation/ResponseTier.kt
// module: domain/conversation | layer: domain | role: response-tier
// summary: 响应层级定义 - 分层响应策略的核心模型

package com.elon.app.agent.domain.conversation

/**
 * 📊 响应层级
 * 
 * 根据响应速度和复杂度分层，实现"边听边响应"
 */
enum class ResponseTier(
    /** 目标延迟上限(ms) */
    val targetLatencyMs: Long,
    /** 描述 */
    val description: String
) {
    /**
     * 即时响应 (<100ms)
     * 
     * 适用：简单问候、确认词、打断响应
     * 实现：本地预设响应，无需网络
     */
    INSTANT(100, "即时响应 - 预设词库"),
    
    /**
     * 快速响应 (<500ms)
     * 
     * 适用：常见问题、简单查询
     * 实现：本地规则匹配 + 模板填充
     */
    FAST(500, "快速响应 - 规则匹配"),
    
    /**
     * 常规响应 (<2000ms)
     * 
     * 适用：一般对话、操作确认
     * 实现：AI 快速生成
     */
    NORMAL(2000, "常规响应 - AI生成"),
    
    /**
     * 深度响应 (>2000ms)
     * 
     * 适用：复杂推理、多步操作
     * 实现：AI 深度思考 + 可能需要执行操作
     */
    DEEP(5000, "深度响应 - 复杂处理")
}

/**
 * 响应结果
 */
data class Response(
    /** 响应层级 */
    val tier: ResponseTier,
    
    /** 文本内容 */
    val text: String,
    
    /** 关联的情感 */
    val emotion: Emotion = Emotion.NEUTRAL,
    
    /** 是否需要后续操作 */
    val requiresAction: Boolean = false,
    
    /** 后续操作描述 */
    val actionDescription: String? = null,
    
    /** 是否应该执行任务（简化判断） */
    val shouldExecute: Boolean = requiresAction,
    
    /** 任务目标（简化访问） */
    val actionGoal: String? = actionDescription,
    
    /** 置信度 */
    val confidence: Float = 1.0f,
    
    /** 生成耗时(ms) */
    val generationTimeMs: Long = 0
)

/**
 * 情感类型
 */
enum class Emotion {
    NEUTRAL,    // 中性
    HAPPY,      // 开心
    CURIOUS,    // 好奇
    THINKING,   // 思考
    HELPFUL,    // 乐于助人
    APOLOGETIC, // 抱歉
    EXCITED,    // 兴奋
    CALM        // 平静
}

/**
 * 快速响应条目
 */
data class QuickResponseEntry(
    /** 触发词/模式 */
    val triggers: List<String>,
    
    /** 响应文本（可多个，随机选择） */
    val responses: List<String>,
    
    /** 响应层级 */
    val tier: ResponseTier = ResponseTier.INSTANT,
    
    /** 关联情感 */
    val emotion: Emotion = Emotion.NEUTRAL
)
