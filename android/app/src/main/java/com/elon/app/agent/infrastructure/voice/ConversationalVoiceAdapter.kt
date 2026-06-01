// infrastructure/voice/ConversationalVoiceAdapter.kt
// module: infrastructure/voice | layer: infrastructure | role: voice-conversation-adapter
// summary: 语音对话适配器 - 连接 UI层、对话管理器、语音识别、TTS 的核心桥梁

package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.util.Log
import com.elon.app.AgentVoiceBridge
import com.elon.app.agent.application.conversation.*
import com.elon.app.agent.domain.conversation.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.StateFlow

/**
 * 🎙️ 语音对话适配器
 * 
 * 职责：
 * 1. 连接 StreamingASR（语音输入）和 ConversationManager（对话控制）
 * 2. 连接 TTS（语音输出）和 ConversationManager
 * 3. 提供简洁的 UI 层接口
 * 
 * 设计原则：
 * - 适配器模式：隔离 UI 层与复杂的对话逻辑
 * - 单一职责：只负责连接，不包含业务逻辑
 * - 生命周期感知：自动管理资源
 * 
 * 使用示例：
 * ```kotlin
 * val adapter = ConversationalVoiceAdapter(context)
 * adapter.listener = object : VoiceConversationListener { ... }
 * adapter.start()  // 开始对话
 * adapter.stop()   // 结束对话
 * ```
 */
class ConversationalVoiceAdapter(private val context: Context) {
    
    companion object {
        private const val TAG = "ConversationAdapter"
    }
    
    // ==================== 核心组件 ====================
    
    /** 对话管理器 - 控制对话状态和流程 */
    private val conversationManager = ConversationManager()
    
    /** 多引擎语音识别（含自动回退：系统默认 → 品牌引擎 → Google，任意引擎失败自动切换下一个）*/
    private val agentVoiceBridge = AgentVoiceBridge(context)
    
    /** 快速响应缓存 - 即时响应常见问候 */
    private val quickResponseCache = QuickResponseCache
    
    /** TTS 服务 - 语音合成输出 */
    private var ttsService: TextToSpeechService? = null
    
    // ==================== 协程管理 ====================
    
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    
    // ==================== UI层回调 ====================
    
    /** UI层监听器 */
    var listener: VoiceConversationListener? = null
    
    /** 任务执行回调 */
    var onTaskExecute: ((String) -> Unit)? = null
    
    // ==================== 状态访问 ====================
    
    /** 当前对话状态（可观察） */
    val currentState: StateFlow<ConversationState>
        get() = conversationManager.currentState
    
    /** 是否正在运行 */
    val isRunning: Boolean
        get() = conversationManager.currentState.value != ConversationState.IDLE
    
    // ==================== 初始化 ====================
    
    init {
        setupConversationManager()
        setupASR()
        Log.i(TAG, "✅ ConversationalVoiceAdapter 初始化完成")
    }
    
    /**
     * 配置对话管理器
     */
    private fun setupConversationManager() {
        // 设置快速响应提供者
        conversationManager.quickResponseProvider = object : QuickResponseProvider {
            override fun tryGetQuickResponse(input: String): Response? {
                return quickResponseCache.tryGetQuickResponse(input)
            }
        }
        
        // 设置对话回调
        conversationManager.listener = object : ConversationListener {
            
            override fun onStateChanged(from: ConversationState, to: ConversationState) {
                Log.d(TAG, "🔄 状态: $from → $to")
                listener?.onStateChanged(from, to)
            }
            
            override fun onListeningStarted() {
                Log.d(TAG, "🎤 开始聆听")
                listener?.onListeningStarted()
            }
            
            override fun onPartialText(text: String) {
                listener?.onUserSpeaking(text)
            }
            
            override fun onUserInputComplete(fullText: String) {
                Log.i(TAG, "📝 用户输入完成: $fullText")
                listener?.onUserInputComplete(fullText)
            }
            
            override fun onThinkingStarted(input: String) {
                Log.d(TAG, "🧠 开始思考: $input")
                listener?.onThinking()
            }
            
            override fun onResponseReady(response: Response) {
                Log.i(TAG, "💬 响应就绪: ${response.text}")
                handleResponse(response)
            }
            
            override fun onSpeakingStarted(text: String) {
                listener?.onAssistantSpeaking(text)
            }
            
            override fun onSpeakingFinished() {
                listener?.onAssistantFinished()
            }
            
            override fun onExecutionStarted(goal: String) {
                Log.i(TAG, "🚀 开始执行: $goal")
                listener?.onExecuting(goal)
                onTaskExecute?.invoke(goal)
            }
            
            override fun onExecutionCompleted(success: Boolean, result: String) {
                listener?.onExecutionComplete(success, result)
            }
            
            override fun onInterrupted() {
                Log.d(TAG, "⚡ 被打断")
                ttsService?.stop()
                listener?.onInterrupted()
            }
            
            override fun onError(error: String) {
                Log.e(TAG, "❌ 错误: $error")
                listener?.onError(error)
            }
            
            override fun onIdleReturned() {
                Log.d(TAG, "😴 回到空闲")
                listener?.onIdle()
            }
        }
    }
    
    /**
     * 配置语音识别（通过 AgentVoiceBridge 获得多引擎自动回退）
     */
    private fun setupASR() {
        agentVoiceBridge.onStart = { conversationManager.onSpeechStart() }
        agentVoiceBridge.onPartial = { text -> conversationManager.onPartialResult(text, 1.0f) }
        agentVoiceBridge.onFinal = { text -> conversationManager.onSpeechEnd(text) }
        agentVoiceBridge.onEnd = {
            // AgentVoiceBridge 对空结果不会触发 onFinal，直接触发 onEnd；
            // 若 conversationManager 仍在监听中（未拿到文字），通知其以空结果结束，避免 UI 卡住。
            if (conversationManager.currentState.value == com.elon.app.agent.domain.conversation.ConversationState.LISTENING) {
                conversationManager.onSpeechEnd("")
            }
        }
        agentVoiceBridge.onError = { error ->
            Log.e(TAG, "🎤 ASR 错误: $error")
            conversationManager.onError(error)
        }
    }
    
    // ==================== 公开方法 ====================
    
    /**
     * 🎬 开始语音对话
     * 
     * 调用后进入聆听状态，等待用户说话
     */
    fun start() {
        Log.i(TAG, "▶️ 开始语音对话")
        agentVoiceBridge.start()
    }
    
    /**
     * ⏹️ 停止语音对话
     * 
     * 停止所有输入输出，回到空闲状态
     */
    fun stop() {
        Log.i(TAG, "⏹️ 停止语音对话")
        agentVoiceBridge.stop()
        ttsService?.stop()
        conversationManager.reset()
    }
    
    /**
     * 🔄 重新开始聆听
     * 
     * 用于用户主动触发新一轮对话
     */
    fun restartListening() {
        Log.d(TAG, "🔄 重新开始聆听")
        
        // 先重置对话状态，避免旧状态影响
        conversationManager.reset()
        
        // 停止当前的 ASR
        agentVoiceBridge.stop()
        
        scope.launch {
            delay(300) // 增加延迟，确保 ASR 完全停止
            Log.d(TAG, "🔄 延迟后启动 ASR")
            agentVoiceBridge.start()
        }
    }
    
    /**
     * ⚡ 用户打断
     * 
     * 当用户在助手说话时开始说话，调用此方法
     */
    fun interrupt() {
        conversationManager.interrupt()
        ttsService?.stop()
    }
    
    /**
     * 📝 文字输入（非语音）
     * 
     * 支持文字输入模式
     */
    fun submitText(text: String) {
        Log.i(TAG, "📝 文字输入: $text")
        conversationManager.onTextInput(text)
    }
    
    /**
     * 🧹 释放资源
     */
    fun destroy() {
        Log.i(TAG, "🧹 释放资源")
        scope.cancel()
        agentVoiceBridge.destroy()
        ttsService?.destroy()
        conversationManager.destroy()
    }
    
    // ==================== 内部方法 ====================
    
    /**
     * 处理响应
     */
    private fun handleResponse(response: Response) {
        // 更新 UI
        listener?.onAssistantResponse(response.text)
        
        // 如果需要执行任务
        if (response.shouldExecute && response.actionGoal != null) {
            Log.i(TAG, "🎯 需要执行任务: ${response.actionGoal}")
            
            // 先播放 TTS 确认，然后执行任务
            ttsService?.speak(response.text, onComplete = {
                Log.i(TAG, "✅ TTS 播放完成，开始执行任务")
                scope.launch {
                    conversationManager.startExecution(response.actionGoal)
                }
            }) ?: run {
                // 没有 TTS，短暂延迟后直接执行
                scope.launch {
                    delay(300)
                    conversationManager.startExecution(response.actionGoal)
                }
            }
        } else {
            // 普通对话，可以播放 TTS
            speakResponse(response.text)
        }
    }
    
    /**
     * TTS 播放响应
     */
    private fun speakResponse(text: String) {
        ttsService?.speak(text, onComplete = {
            conversationManager.onSpeakingFinished()
        }) ?: run {
            // 没有 TTS，直接标记完成
            scope.launch {
                delay(500) // 模拟说话时间
                conversationManager.onSpeakingFinished()
            }
        }
    }
    
    // ==================== TTS 配置 ====================
    
    /**
     * 设置 TTS 服务
     */
    fun setTTSService(tts: TextToSpeechService?) {
        this.ttsService = tts
    }
    
    /**
     * 设置意图分析器（智能模式）
     * 
     * 如果不设置，会使用简单的关键词匹配（不智能）
     * 设置后会调用 AI 进行意图分析
     */
    fun setIntentAnalyzer(analyzer: StreamingIntentAnalyzer?) {
        conversationManager.intentAnalyzer = analyzer
        Log.i(TAG, if (analyzer != null) "✅ 已设置智能意图分析器" else "⚠️ 未设置意图分析器，使用关键词匹配")
    }
    
    /**
     * 设置响应生成器（智能对话模式）
     * 
     * 如果设置，会使用 AI 生成自然对话回复
     */
    fun setResponseGenerator(generator: ResponseGenerator?) {
        conversationManager.responseGenerator = generator
        Log.i(TAG, if (generator != null) "✅ 已设置智能响应生成器" else "⚠️ 未设置响应生成器，使用模板回复")
    }
}

// ==================== UI层简化接口 ====================

/**
 * 🎯 语音对话监听器（UI层使用）
 * 
 * 设计原则：
 * - 只包含 UI 需要的回调
 * - 方法名直观，无需阅读文档即可理解
 * - 提供默认空实现，UI 只需覆盖关心的方法
 */
interface VoiceConversationListener {
    
    /** 状态变化（用于 UI 状态同步） */
    fun onStateChanged(from: ConversationState, to: ConversationState) {}
    
    /** 开始聆听 - 可显示录音动画 */
    fun onListeningStarted() {}
    
    /** 用户正在说话 - 实时显示识别文字 */
    fun onUserSpeaking(text: String) {}
    
    /** 用户输入完成 - 显示完整文字 */
    fun onUserInputComplete(text: String) {}
    
    /** 正在思考 - 可显示思考动画 */
    fun onThinking() {}
    
    /** 助手响应文字 - 显示响应内容 */
    fun onAssistantResponse(text: String) {}
    
    /** 助手开始说话 - 可显示说话动画 */
    fun onAssistantSpeaking(text: String) {}
    
    /** 助手说完了 */
    fun onAssistantFinished() {}
    
    /** 正在执行任务 */
    fun onExecuting(goal: String) {}
    
    /** 任务执行完成 */
    fun onExecutionComplete(success: Boolean, result: String) {}
    
    /** 被用户打断 */
    fun onInterrupted() {}
    
    /** 发生错误 */
    fun onError(error: String) {}
    
    /** 回到空闲状态 */
    fun onIdle() {}
}

// ==================== TTS 服务接口 ====================

/**
 * 🔊 TTS 服务接口
 * 
 * 抽象接口，支持多种 TTS 实现：
 * - Android 内置 TTS
 * - 云端 TTS（阿里云、讯飞等）
 * - 流式 TTS
 */
interface TextToSpeechService {
    
    /** 播放文字 */
    fun speak(text: String, onComplete: (() -> Unit)? = null)
    
    /** 停止播放 */
    fun stop()
    
    /** 释放资源 */
    fun destroy()
    
    /** 是否正在播放 */
    val isSpeaking: Boolean
}
