// application/conversation/ConversationManager.kt
// module: application/conversation | layer: application | role: conversation-orchestrator
// summary: 对话协调器 - 数字人助手的核心控制中枢

package com.elon.app.agent.application.conversation

import android.util.Log
import com.elon.app.agent.domain.conversation.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import java.util.UUID
import java.util.concurrent.atomic.AtomicReference

/**
 * 🎭 对话协调器
 * 
 * 核心职责：
 * 1. 管理对话状态机
 * 2. 协调语音输入、理解、响应各模块
 * 3. 处理打断和并发
 * 4. 提供统一的对话回调接口
 * 
 * 设计原则：
 * - 状态机驱动，所有行为由状态决定
 * - 支持随时打断
 * - 边听边分析，减少延迟
 */
class ConversationManager {
    
    companion object {
        private const val TAG = "ConversationManager"
        
        // 默认超时配置
        private const val LISTENING_TIMEOUT_MS = 10_000L   // 听取超时
        private const val THINKING_TIMEOUT_MS = 5_000L     // 思考超时
        private const val SPEAKING_TIMEOUT_MS = 30_000L    // 说话超时
    }
    
    // ==================== 状态管理 ====================
    
    private val _currentState = MutableStateFlow(ConversationState.IDLE)
    /** 当前状态（可观察） */
    val currentState: StateFlow<ConversationState> = _currentState.asStateFlow()
    
    private val _metadata = AtomicReference(ConversationMetadata(turnId = generateTurnId()))
    /** 当前对话元数据 */
    val metadata: ConversationMetadata get() = _metadata.get()
    
    // ==================== 协程管理 ====================
    
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private var currentJob: Job? = null
    private var timeoutJob: Job? = null
    
    // ==================== 回调接口 ====================
    
    /** 对话回调监听器 */
    var listener: ConversationListener? = null
    
    /** 快速响应提供者 */
    var quickResponseProvider: QuickResponseProvider? = null
    
    /** 意图分析器 */
    var intentAnalyzer: StreamingIntentAnalyzer? = null
    
    /** 响应生成器 */
    var responseGenerator: ResponseGenerator? = null
    
    // ==================== 累积输入 ====================
    
    private val accumulatedInput = StringBuilder()
    private var lastPartialResult: String = ""
    
    // ==================== 公开方法 ====================
    
    /**
     * 🎤 用户开始说话
     */
    fun onSpeechStart() {
        Log.d(TAG, "🎤 检测到语音开始")
        
        when (_currentState.value) {
            ConversationState.IDLE -> {
                // 从空闲进入倾听
                transitionTo(ConversationState.LISTENING)
                startNewTurn()
                listener?.onListeningStarted()
            }
            
            ConversationState.SPEAKING -> {
                // 用户打断！
                Log.i(TAG, "⚡ 用户打断了助手")
                handleInterruption()
            }
            
            ConversationState.AWAITING_MORE -> {
                // 用户继续说
                transitionTo(ConversationState.LISTENING)
                listener?.onListeningStarted()
            }
            
            else -> {
                Log.w(TAG, "在状态 ${_currentState.value} 收到语音开始，忽略")
            }
        }
    }
    
    /**
     * 📝 收到部分识别结果（流式）
     * 
     * @param text 部分识别文本
     * @param confidence 置信度（可选，默认1.0）
     */
    fun onPartialResult(text: String, confidence: Float = 1.0f) {
        if (_currentState.value != ConversationState.LISTENING) return
        
        lastPartialResult = text
        val fullText = if (accumulatedInput.isNotEmpty()) {
            "${accumulatedInput}$text"
        } else {
            text
        }
        
        Log.d(TAG, "📝 部分结果: $text (置信度: $confidence)")
        listener?.onPartialResult(fullText)
        listener?.onPartialText(fullText) // 兼容两种回调
        
        // 🚀 边听边分析：尝试快速响应
        scope.launch {
            tryQuickResponse(fullText)
        }
    }
    
    /**
     * ✅ 收到最终识别结果
     */
    fun onFinalResult(text: String) {
        if (_currentState.value != ConversationState.LISTENING) return
        
        Log.i(TAG, "✅ 最终结果: $text")
        
        // 累积输入
        if (accumulatedInput.isNotEmpty()) {
            accumulatedInput.append(" ")
        }
        accumulatedInput.append(text)
        
        val fullInput = accumulatedInput.toString()
        listener?.onFinalResult(fullInput)
        listener?.onUserInputComplete(fullInput) // 兼容两种回调
        
        // 进入思考状态
        transitionTo(ConversationState.THINKING)
        listener?.onThinkingStarted(fullInput)
        
        // 开始分析意图
        analyzeIntent(fullInput)
    }
    
    /**
     * 🛑 语音结束（VAD检测到静音）
     * 
     * @param finalText 最终文本（可选）
     */
    fun onSpeechEnd(finalText: String? = null) {
        Log.d(TAG, "🛑 语音结束 (VAD)")
        
        // 如果传入了最终文本，使用它
        val text = finalText ?: lastPartialResult
        
        // 如果有最后的部分结果，当作最终结果处理
        if (_currentState.value == ConversationState.LISTENING && text.isNotEmpty()) {
            onFinalResult(text)
        }
    }
    
    /**
     * 📝 文字输入（非语音）
     */
    fun onTextInput(text: String) {
        if (text.isBlank()) return
        
        Log.i(TAG, "📝 文字输入: $text")
        
        // 直接作为最终结果处理
        transitionTo(ConversationState.LISTENING)
        onFinalResult(text)
    }
    
    /**
     * ⚡ 主动打断
     */
    fun interrupt() {
        if (_currentState.value == ConversationState.SPEAKING) {
            handleInterruption()
        }
    }
    
    /**
     * 🚀 开始执行任务
     */
    fun startExecution(goal: String) {
        Log.i(TAG, "🚀 开始执行: $goal")
        transitionTo(ConversationState.EXECUTING)
        listener?.onExecutionStarted(goal)
    }
    
    /**
     * ✅ 执行完成
     */
    fun onExecutionComplete(success: Boolean, result: String) {
        Log.i(TAG, "✅ 执行完成: success=$success, result=$result")
        listener?.onExecutionCompleted(success, result)
        transitionTo(ConversationState.IDLE)
        resetTurn()
    }
    
    /**
     * ❌ 错误
     */
    fun onError(error: String) {
        Log.e(TAG, "❌ 错误: $error")
        listener?.onError(error)
        transitionTo(ConversationState.IDLE)
        resetTurn()
    }
    
    /**
     * 🗣️ 语音播放完毕
     */
    fun onSpeakingFinished() {
        Log.d(TAG, "🗣️ 语音播放完毕")
        onResponseFinished()
    }
    
    /**
     * 🗣️ 响应播放完毕
     */
    fun onResponseFinished() {
        Log.d(TAG, "🗣️ 响应播放完毕")
        
        if (_currentState.value == ConversationState.SPEAKING) {
            listener?.onSpeakingFinished()
            transitionTo(ConversationState.IDLE)
            listener?.onConversationIdle()
            listener?.onIdleReturned() // 兼容回调
            
            // 清理本轮数据
            resetTurn()
        }
    }
    
    /**
     * 🔄 重置对话
     */
    fun reset() {
        Log.i(TAG, "🔄 重置对话")
        cancelAllJobs()
        transitionTo(ConversationState.IDLE)
        resetTurn()
        listener?.onConversationIdle()
        listener?.onIdleReturned() // 兼容回调
    }
    
    /**
     * 🧹 释放资源
     */
    fun destroy() {
        cancelAllJobs()
        scope.cancel()
    }
    
    // ==================== 内部方法 ====================
    
    /**
     * 状态转换
     */
    private fun transitionTo(newState: ConversationState) {
        val oldState = _currentState.value
        if (oldState == newState) return
        
        Log.i(TAG, "📍 状态转换: $oldState → $newState")
        
        // 取消旧状态的超时
        timeoutJob?.cancel()
        
        // 更新状态
        _currentState.value = newState
        _metadata.updateAndGet { it.copy(stateEnteredAt = System.currentTimeMillis()) }
        
        // 设置新状态的超时
        setupTimeout(newState)
        
        // 通知状态变化
        listener?.onStateChanged(oldState, newState)
    }
    
    /**
     * 设置状态超时
     */
    private fun setupTimeout(state: ConversationState) {
        val timeoutMs = when (state) {
            ConversationState.LISTENING -> LISTENING_TIMEOUT_MS
            ConversationState.THINKING -> THINKING_TIMEOUT_MS
            ConversationState.SPEAKING -> SPEAKING_TIMEOUT_MS
            else -> return
        }
        
        timeoutJob = scope.launch {
            delay(timeoutMs)
            Log.w(TAG, "⏰ 状态 $state 超时")
            handleTimeout(state)
        }
    }
    
    /**
     * 处理超时
     */
    private fun handleTimeout(state: ConversationState) {
        when (state) {
            ConversationState.LISTENING -> {
                // 听取超时 - 如果有部分输入，尝试处理
                if (accumulatedInput.isNotEmpty() || lastPartialResult.isNotEmpty()) {
                    val input = accumulatedInput.toString().ifEmpty { lastPartialResult }
                    onFinalResult(input)
                } else {
                    // 没有输入，回到空闲
                    transitionTo(ConversationState.IDLE)
                    listener?.onConversationIdle()
                }
            }
            
            ConversationState.THINKING -> {
                // 思考超时 - 返回默认响应
                deliverResponse(Response(
                    tier = ResponseTier.FAST,
                    text = "抱歉，我思考得太久了。能再说一遍吗？",
                    emotion = Emotion.APOLOGETIC
                ))
            }
            
            ConversationState.SPEAKING -> {
                // 说话超时 - 强制结束
                onResponseFinished()
            }
            
            else -> {}
        }
    }
    
    /**
     * 处理打断
     */
    private fun handleInterruption() {
        cancelAllJobs()
        
        // 通知停止播放
        listener?.onInterrupted()
        
        // 转入倾听状态
        transitionTo(ConversationState.LISTENING)
        startNewTurn()
        listener?.onListeningStarted()
    }
    
    /**
     * 尝试快速响应
     */
    private suspend fun tryQuickResponse(input: String) {
        val provider = quickResponseProvider ?: return
        
        val quickResponse = provider.tryGetQuickResponse(input)
        if (quickResponse != null && quickResponse.tier == ResponseTier.INSTANT) {
            Log.i(TAG, "⚡ 触发即时响应: ${quickResponse.text}")
            // 即时响应不中断听取，只是给一个反馈
            listener?.onQuickFeedback(quickResponse.text, quickResponse.emotion)
        }
    }
    
    /**
     * 分析意图
     */
    private fun analyzeIntent(input: String) {
        Log.i(TAG, "🧠 [意图分析开始] input=$input")
        
        currentJob = scope.launch {
            try {
                val analyzer = intentAnalyzer
                if (analyzer == null) {
                    Log.w(TAG, "🧠 [无分析器] 使用简单规则")
                    // 没有分析器，使用快速响应
                    handleNoAnalyzer(input)
                    return@launch
                }
                
                val result = analyzer.analyze(input)
                Log.i(TAG, "🧠 [分析结果] complete=${result.isComplete}, operation=${result.isOperation}, confidence=${result.confidence}")
                
                when {
                    result.isComplete -> {
                        Log.i(TAG, "🧠 [路由] → 表述完整，生成响应")
                        // 表述完整，生成响应
                        generateResponse(result)
                    }
                    result.isOperation && !result.isComplete -> {
                        Log.i(TAG, "🧠 [路由] → 操作不完整，追问用户")
                        // 是操作请求但不完整，追问用户
                        handleIncompleteInput(input, result.hint)
                    }
                    else -> {
                        // 非操作请求（闲聊/问答），即使"不完整"也当作完整处理
                        // 因为闲聊不需要追问，直接生成对话回复
                        Log.i(TAG, "🧠 [路由] → 非操作请求，生成对话回复: $input")
                        generateResponse(result.copy(isComplete = true))
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "🧠 [意图分析失败]", e)
                handleAnalysisError(e)
            }
        }
    }
    
    /**
     * 处理不完整输入
     */
    private fun handleIncompleteInput(input: String, hint: String?) {
        Log.i(TAG, "📝 表述不完整: $input")
        
        transitionTo(ConversationState.AWAITING_MORE)
        
        val prompt = hint ?: "请继续..."
        listener?.onAwaitingMore(prompt)
        
        // 给一个提示后继续听
        deliverResponse(Response(
            tier = ResponseTier.INSTANT,
            text = prompt,
            emotion = Emotion.CURIOUS
        ), returnToListening = true)
    }
    
    /**
     * 生成响应
     */
    private fun generateResponse(intentResult: IntentAnalysisResult) {
        scope.launch {
            try {
                // 先尝试快速响应
                val quickResponse = quickResponseProvider?.tryGetQuickResponse(intentResult.normalizedInput)
                if (quickResponse != null && quickResponse.tier <= ResponseTier.FAST) {
                    deliverResponse(quickResponse)
                    return@launch
                }
                
                // 使用响应生成器
                val generator = responseGenerator
                if (generator == null) {
                    // 没有生成器，使用默认响应
                    deliverDefaultResponse(intentResult)
                    return@launch
                }
                
                val response = generator.generate(intentResult)
                deliverResponse(response)
                
            } catch (e: Exception) {
                Log.e(TAG, "响应生成失败", e)
                deliverResponse(Response(
                    tier = ResponseTier.FAST,
                    text = "抱歉，我遇到了一点问题。",
                    emotion = Emotion.APOLOGETIC
                ))
            }
        }
    }
    
    /**
     * 投递响应
     */
    private fun deliverResponse(response: Response, returnToListening: Boolean = false) {
        Log.i(TAG, "📤 投递响应: [${response.tier}] ${response.text}")
        
        if (!returnToListening) {
            transitionTo(ConversationState.SPEAKING)
            listener?.onSpeakingStarted(response.text)
        }
        
        listener?.onResponse(response)
        listener?.onResponseReady(response) // 兼容回调
        
        // 如果需要执行操作
        if (response.requiresAction && response.actionDescription != null) {
            listener?.onOperationRequired(response.actionDescription)
        }
    }
    
    /**
     * 没有分析器时的处理
     */
    private fun handleNoAnalyzer(input: String) {
        // 使用简单规则判断
        val inputLower = input.lowercase()
        
        // 1. 检查是否是纯问候语（不包含操作词）
        val greetingWords = listOf("你好", "嗨", "hello", "hi", "早上好", "晚上好", "下午好")
        val isOnlyGreeting = greetingWords.any { inputLower.contains(it) } && 
                             input.length < 10 // 短问候语
        
        // 2. 检查是否包含操作性词语
        val operationKeywords = listOf(
            "打开", "启动", "运行", "进入",  // 打开类
            "搜索", "搜一下", "查一下", "找", "查找",  // 搜索类
            "发送", "发", "回复", "转发",  // 发送类
            "点击", "点", "按", "选择",  // 点击类
            "帮我", "帮忙", "请", "麻烦",  // 请求前缀
            "看看", "看一下", "浏览",  // 浏览类
            "返回", "退出", "关闭"  // 导航类
        )
        val isOperation = operationKeywords.any { inputLower.contains(it) }
        
        val response = when {
            isOnlyGreeting -> {
                // 纯问候语，友好回应
                Response(
                    tier = ResponseTier.INSTANT,
                    text = "你好！有什么可以帮你的？",
                    emotion = Emotion.HAPPY
                )
            }
            isOperation -> {
                // 操作性请求，确认后执行
                Log.i(TAG, "🎯 检测到操作请求: $input")
                Response(
                    tier = ResponseTier.FAST,
                    text = "好的，我来帮你$input",
                    emotion = Emotion.HELPFUL,
                    requiresAction = true,
                    actionDescription = input  // 将原始输入作为任务目标
                )
            }
            else -> {
                // 其他普通对话
                Response(
                    tier = ResponseTier.FAST,
                    text = "我听到了：$input。你想让我做什么呢？",
                    emotion = Emotion.CURIOUS
                )
            }
        }
        
        deliverResponse(response)
    }
    
    /**
     * 分析错误处理
     */
    private fun handleAnalysisError(e: Exception) {
        deliverResponse(Response(
            tier = ResponseTier.FAST,
            text = "抱歉，我没有理解。可以再说一遍吗？",
            emotion = Emotion.APOLOGETIC
        ))
    }
    
    /**
     * 默认响应
     */
    private fun deliverDefaultResponse(intentResult: IntentAnalysisResult) {
        val text = when {
            intentResult.isOperation -> "好的，我来帮你${intentResult.normalizedInput}"
            else -> "我听到了：${intentResult.normalizedInput}"
        }
        
        deliverResponse(Response(
            tier = ResponseTier.NORMAL,
            text = text,
            emotion = Emotion.HELPFUL,
            requiresAction = intentResult.isOperation,
            actionDescription = if (intentResult.isOperation) intentResult.normalizedInput else null
        ))
    }
    
    /**
     * 开始新轮次
     */
    private fun startNewTurn() {
        _metadata.set(ConversationMetadata(
            turnId = generateTurnId(),
            startTime = System.currentTimeMillis(),
            isFirstInteraction = accumulatedInput.isEmpty()
        ))
    }
    
    /**
     * 重置轮次数据
     */
    private fun resetTurn() {
        accumulatedInput.clear()
        lastPartialResult = ""
    }
    
    /**
     * 取消所有任务
     */
    private fun cancelAllJobs() {
        currentJob?.cancel()
        timeoutJob?.cancel()
        currentJob = null
        timeoutJob = null
    }
    
    /**
     * 生成轮次ID
     */
    private fun generateTurnId(): String = UUID.randomUUID().toString().take(8)
}

// ==================== 回调接口 ====================

/**
 * 对话监听器
 * 
 * 提供完整的对话生命周期回调
 */
interface ConversationListener {
    /** 状态变化 */
    fun onStateChanged(oldState: ConversationState, newState: ConversationState) {}
    
    /** 开始倾听 */
    fun onListeningStarted() {}
    
    /** 收到部分结果 */
    fun onPartialResult(text: String) {}
    
    /** 收到部分文本（与 onPartialResult 等价，用于兼容） */
    fun onPartialText(text: String) {}
    
    /** 收到最终结果 */
    fun onFinalResult(text: String) {}
    
    /** 用户输入完成（与 onFinalResult 等价，用于兼容） */
    fun onUserInputComplete(fullText: String) {}
    
    /** 开始思考 */
    fun onThinkingStarted(input: String) {}
    
    /** 等待用户补充 */
    fun onAwaitingMore(prompt: String) {}
    
    /** 收到快速反馈（不中断听取） */
    fun onQuickFeedback(text: String, emotion: Emotion) {}
    
    /** 收到响应 - 新版回调 */
    fun onResponse(response: Response) {}
    
    /** 响应就绪 - 兼容回调 */
    fun onResponseReady(response: Response) {}
    
    /** 开始说话 */
    fun onSpeakingStarted(text: String) {}
    
    /** 说话结束 */
    fun onSpeakingFinished() {}
    
    /** 被打断 */
    fun onInterrupted() {}
    
    /** 需要执行操作 */
    fun onOperationRequired(operation: String) {}
    
    /** 开始执行任务 */
    fun onExecutionStarted(goal: String) {}
    
    /** 执行完成 */
    fun onExecutionCompleted(success: Boolean, result: String) {}
    
    /** 发生错误 */
    fun onError(error: String) {}
    
    /** 对话空闲 */
    fun onConversationIdle() {}
    
    /** 回到空闲状态 - 与 onConversationIdle 等价 */
    fun onIdleReturned() {}
}

// ==================== 辅助接口 ====================

/**
 * 快速响应提供者
 */
interface QuickResponseProvider {
    fun tryGetQuickResponse(input: String): Response?
}

/**
 * 流式意图分析器
 */
interface StreamingIntentAnalyzer {
    suspend fun analyze(input: String): IntentAnalysisResult
}

/**
 * 响应生成器
 */
interface ResponseGenerator {
    suspend fun generate(intent: IntentAnalysisResult): Response
}

/**
 * 意图分析结果
 */
data class IntentAnalysisResult(
    val normalizedInput: String,
    val isComplete: Boolean,
    val isOperation: Boolean,
    val confidence: Float,
    val hint: String? = null
)
