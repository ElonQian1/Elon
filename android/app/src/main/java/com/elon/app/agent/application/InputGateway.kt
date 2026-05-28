// application/InputGateway.kt
// module: application | layer: application | role: input-gateway
// summary: 统一输入入口网关 - 所有用户输入必须经过此处，包含防抖、意图分析、路由

package com.elon.app.agent.application

import android.util.Log
import kotlinx.coroutines.*
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/**
 * 🚪 统一输入入口网关
 * 
 * 所有用户输入（悬浮球、UI按钮、语音、Socket）都必须经过此网关。
 * 
 * 职责：
 * 1. 防抖去重 - 防止同一输入重复触发
 * 2. 意图分析 - 判断是聊天还是操作
 * 3. 路由分发 - 根据意图类型分发到不同处理器
 * 
 * 设计原则：
 * - 意图分析不可绕过
 * - 失败时安全降级（默认当聊天处理）
 * - 单例模式，全局唯一入口
 */
object InputGateway {
    
    private const val TAG = "InputGateway"
    
    // 防抖配置
    private const val DEBOUNCE_MS = 800L  // 800ms 内的重复输入会被忽略
    private val lastInputTime = AtomicLong(0L)
    private var lastInputText: String = ""
    
    // 执行锁 - 防止并发执行
    private val isProcessing = AtomicBoolean(false)
    
    // 协程作用域
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    
    // 回调接口
    interface InputCallback {
        fun onChatResponse(response: String)
        fun onOperationStart(goal: String)
        fun onError(error: String)
        fun onLog(message: String) {}  // 默认空实现
    }
    
    /**
     * 输入来源枚举
     */
    enum class InputSource {
        FLOATING_BALL,      // 悬浮球
        VOICE,              // 语音输入
        UI_BUTTON,          // UI按钮
        SOCKET,             // Socket命令
        NOTIFICATION,       // 通知栏
        QUICK_TASK          // 快捷任务
    }
    
    /**
     * 🚀 提交用户输入（所有入口的统一方法）
     * 
     * @param input 用户输入文本
     * @param source 输入来源
     * @param callback 结果回调
     * @param scriptEngine 脚本引擎实例（可选，如果不传会尝试自动获取）
     */
    fun submit(
        input: String,
        source: InputSource,
        callback: InputCallback,
        scriptEngine: ScriptEngine? = null
    ) {
        val trimmedInput = input.trim()
        
        // 1. 空输入检查
        if (trimmedInput.isBlank()) {
            callback.onError("输入不能为空")
            return
        }
        
        // 2. 防抖检查
        val now = System.currentTimeMillis()
        val timeSinceLastInput = now - lastInputTime.get()
        
        if (timeSinceLastInput < DEBOUNCE_MS && trimmedInput == lastInputText) {
            Log.w(TAG, "🚫 防抖拦截: 重复输入 '$trimmedInput' (间隔 ${timeSinceLastInput}ms)")
            return
        }
        
        // 3. 并发执行检查
        if (!isProcessing.compareAndSet(false, true)) {
            Log.w(TAG, "🚫 并发拦截: 已有任务在执行中")
            callback.onError("请等待当前任务完成")
            return
        }
        
        // 更新防抖状态
        lastInputTime.set(now)
        lastInputText = trimmedInput
        
        Log.i(TAG, "📥 收到输入: '$trimmedInput' [来源: $source]")
        callback.onLog("📥 收到输入: $trimmedInput")
        
        // 4. 启动处理流程
        scope.launch {
            try {
                processInput(trimmedInput, source, scriptEngine, callback)
            } finally {
                isProcessing.set(false)
            }
        }
    }
    
    /**
     * 处理输入（内部方法）
     */
    private suspend fun processInput(
        input: String,
        source: InputSource,
        scriptEngine: ScriptEngine?,
        callback: InputCallback
    ) {
        // 🔧 如果没有 scriptEngine，使用内置的快速规则匹配
        if (scriptEngine == null) {
            Log.w(TAG, "⚠️ ScriptEngine 未初始化，使用内置快速匹配")
            handleWithQuickRules(input, callback)
            return
        }
        
        // 🧠 意图分析（核心步骤，不可跳过）
        withContext(Dispatchers.Main) {
            callback.onLog("🧠 分析用户意图...")
        }
        
        val intentResult = try {
            scriptEngine.analyzeIntent(input)
        } catch (e: Exception) {
            Log.e(TAG, "❌ 意图分析异常: ${e.message}", e)
            // 失败时安全降级：当作聊天处理
            ScriptEngine.IntentResult(
                intent = ScriptEngine.UserIntent.CHAT,
                confidence = 0.0f,
                chatResponse = "抱歉，我没太理解你的意思。你可以试着说得更具体一些，比如【打开微信】或【搜索今日新闻】。"
            )
        }
        
        Log.i(TAG, "🎯 意图分析结果: ${intentResult.intent} (置信度: ${intentResult.confidence})")
        
        // 根据意图类型分发处理
        when (intentResult.intent) {
            ScriptEngine.UserIntent.CHAT -> {
                // 聊天意图 - 直接回复
                val response = intentResult.chatResponse 
                    ?: "你好！我是手机自动化助手，可以帮你操作手机。试试说【打开微信】或【搜索附近美食】。"
                
                Log.i(TAG, "💬 聊天回复: $response")
                withContext(Dispatchers.Main) {
                    callback.onLog("💬 这是日常对话")
                    callback.onChatResponse(response)
                }
            }
            
            ScriptEngine.UserIntent.PHONE_OPERATION -> {
                // 操作意图 - 检查完整性后执行
                val goal = intentResult.operationGoal ?: input
                
                if (intentResult.isComplete == false) {
                    // 表述不完整，请求补充
                    withContext(Dispatchers.Main) {
                        callback.onLog("⚠️ 指令不完整，请补充")
                        callback.onChatResponse("你想${goal}什么呢？请说得更具体一些。")
                    }
                } else {
                    // 完整指令，执行操作
                    Log.i(TAG, "🎯 执行操作: $goal")
                    withContext(Dispatchers.Main) {
                        callback.onLog("🎯 识别为手机操作: $goal")
                        callback.onOperationStart(goal)
                    }
                }
            }
            
            ScriptEngine.UserIntent.UNKNOWN -> {
                // 不确定 - 安全降级为聊天
                val response = intentResult.chatResponse 
                    ?: "我不太确定你想做什么。你可以直接告诉我，比如【打开淘宝】或【帮我搜索天气】。"
                
                Log.i(TAG, "❓ 意图不明，降级为聊天")
                withContext(Dispatchers.Main) {
                    callback.onLog("❓ 意图不明确")
                    callback.onChatResponse(response)
                }
            }
        }
    }
    
    /**
     * 🔧 内置快速规则匹配（当没有 ScriptEngine 时使用）
     * 
     * 这是一个简化版的意图匹配，不调用 AI
     */
    private suspend fun handleWithQuickRules(input: String, callback: InputCallback) {
        val normalized = input.trim().lowercase()
        
        // 1. 简单问候语直接响应
        val greetingResponses = mapOf(
            "你好" to "你好！我是手机自动化助手。",
            "您好" to "您好！有什么可以帮您的吗？",
            "嗨" to "嗨！我可以帮你操作手机。",
            "hi" to "Hi! 有什么需要帮忙的吗？",
            "hello" to "Hello! 试试说【打开微信】。",
            "谢谢" to "不客气！",
            "再见" to "再见！",
            "拜拜" to "拜拜！"
        )
        
        greetingResponses[normalized]?.let { response ->
            withContext(Dispatchers.Main) {
                callback.onLog("💬 问候语快速响应")
                callback.onChatResponse(response)
            }
            return
        }
        
        // 2. 短输入（<=4字符）且无明显操作关键词，当作聊天
        val operationKeywords = listOf("打开", "启动", "搜索", "点击", "滑动", "获取", "发送")
        val hasOperationKeyword = operationKeywords.any { normalized.contains(it) }
        
        if (input.length <= 4 && !hasOperationKeyword) {
            withContext(Dispatchers.Main) {
                callback.onLog("💬 短输入当作聊天")
                callback.onChatResponse("你好！请告诉我你想做什么，比如【打开微信】。")
            }
            return
        }
        
        // 3. 有操作关键词，尝试执行
        if (hasOperationKeyword) {
            Log.i(TAG, "🎯 检测到操作关键词，执行: $input")
            withContext(Dispatchers.Main) {
                callback.onLog("🎯 识别为操作指令")
                callback.onOperationStart(input)
            }
            return
        }
        
        // 4. 默认当作聊天
        withContext(Dispatchers.Main) {
            callback.onLog("💬 默认当作聊天")
            callback.onChatResponse("我不太确定你想做什么。试试说【打开XX应用】或【搜索XX】。")
        }
    }
    
    /**
     * 重置状态（用于测试或强制重置）
     */
    fun reset() {
        isProcessing.set(false)
        lastInputTime.set(0L)
        lastInputText = ""
        Log.i(TAG, "🔄 InputGateway 状态已重置")
    }
    
    /**
     * 检查是否正在处理
     */
    fun isBusy(): Boolean = isProcessing.get()
    
    /**
     * 取消当前处理
     */
    fun cancel() {
        scope.coroutineContext.cancelChildren()
        isProcessing.set(false)
        Log.i(TAG, "⏹️ 处理已取消")
    }
}
