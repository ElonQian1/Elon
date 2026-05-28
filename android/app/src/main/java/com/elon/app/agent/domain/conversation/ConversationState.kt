// domain/conversation/ConversationState.kt
// module: domain/conversation | layer: domain | role: state-definition
// summary: 对话状态定义 - 数字人助手的核心状态机模型

package com.elon.app.agent.domain.conversation

/**
 * 🎭 对话状态
 * 
 * 定义数字人助手在对话过程中的所有状态
 */
enum class ConversationState {
    /** 空闲 - 等待用户唤醒或开始说话 */
    IDLE,
    
    /** 倾听中 - 正在接收用户语音输入 */
    LISTENING,
    
    /** 思考中 - 正在分析理解用户意图 */
    THINKING,
    
    /** 说话中 - 正在播放语音响应 */
    SPEAKING,
    
    /** 执行中 - 正在执行手机操作 */
    EXECUTING,
    
    /** 被打断 - 用户打断了助手说话 */
    INTERRUPTED,
    
    /** 等待补充 - 用户表述不完整，等待继续 */
    AWAITING_MORE
}

/**
 * 状态转换事件
 */
sealed class ConversationEvent {
    /** 检测到用户开始说话 */
    object SpeechStarted : ConversationEvent()
    
    /** 检测到用户停止说话（VAD） */
    object SpeechEnded : ConversationEvent()
    
    /** 语义分析完成 - 完整表述 */
    data class IntentComplete(val intent: String, val confidence: Float) : ConversationEvent()
    
    /** 语义分析完成 - 不完整表述 */
    data class IntentIncomplete(val partialIntent: String) : ConversationEvent()
    
    /** 响应生成完成 */
    data class ResponseReady(val response: String) : ConversationEvent()
    
    /** 响应播放完毕 */
    object ResponseFinished : ConversationEvent()
    
    /** 用户打断 */
    object UserInterrupted : ConversationEvent()
    
    /** 操作开始执行 */
    data class ExecutionStarted(val operation: String) : ConversationEvent()
    
    /** 操作执行完成 */
    data class ExecutionCompleted(val success: Boolean, val result: String) : ConversationEvent()
    
    /** 超时 */
    object Timeout : ConversationEvent()
    
    /** 错误 */
    data class Error(val message: String) : ConversationEvent()
}

/**
 * 对话元数据
 */
data class ConversationMetadata(
    /** 当前轮次ID */
    val turnId: String,
    
    /** 对话开始时间 */
    val startTime: Long = System.currentTimeMillis(),
    
    /** 当前状态进入时间 */
    val stateEnteredAt: Long = System.currentTimeMillis(),
    
    /** 累计用户输入 */
    val accumulatedInput: String = "",
    
    /** 是否首次交互 */
    val isFirstInteraction: Boolean = false
)
