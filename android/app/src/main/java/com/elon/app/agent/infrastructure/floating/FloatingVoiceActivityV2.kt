// infrastructure/floating/FloatingVoiceActivityV2.kt
// module: infrastructure/floating | layer: infrastructure | role: voice-input-activity-v2
// summary: 语音输入透明Activity V2 - 使用 ConversationalVoiceAdapter 实现智能对话

package com.elon.app.agent.infrastructure.floating

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.WindowManager
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.elon.app.agent.AgentConfigActivity
import com.elon.app.agent.application.IntentAnalyzer
import com.elon.app.agent.domain.conversation.ConversationState
import com.elon.app.agent.infrastructure.voice.*import kotlinx.coroutines.*
import kotlinx.coroutines.flow.collectLatest

/**
 * 🎤 语音输入透明Activity V2
 * 
 * 架构说明：
 * - 使用 ConversationalVoiceAdapter 管理对话流程
 * - 支持"边听边回应"的智能对话体验
 * - 状态驱动 UI 更新
 * - 接入 AI 实现智能意图分析和对话回复
 * 
 * 状态流转：
 * IDLE → LISTENING → THINKING → SPEAKING/EXECUTING → IDLE
 *                  ↖________↙ (被打断时)
 */
class ConversationalVoiceActivity : AppCompatActivity() {
    
    companion object {
        private const val TAG = "ConversationalVoice"
    }
    
    // ==================== UI 组件 ====================
    
    private lateinit var statusText: TextView
    private lateinit var resultText: TextView
    private lateinit var responseText: TextView
    private lateinit var voiceIndicator: TextView
    private lateinit var cancelButton: Button
    
    // ==================== 对话核心 ====================
    
    private lateinit var voiceAdapter: ConversationalVoiceAdapter
    private var ttsService: TextToSpeechService? = null
    
    private val handler = Handler(Looper.getMainLooper())

    /** Activity 是否处于前台（resumed）。后台时不自动重启 ASR，避免拿不到麦克风。 */
    @Volatile
    private var isActivityResumed = false

    /** 是否经历过至少一次 onPause（区分首次启动 vs 从后台回来）。*/
    private var hasEverPaused = false

    // ==================== 权限请求 ====================
    
    private val requestPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        if (isGranted) {
            startConversation()
        } else {
            Toast.makeText(this, "需要麦克风权限", Toast.LENGTH_SHORT).show()
            finish()
        }
    }
    
    // ==================== 生命周期 ====================
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // 设置透明窗口
        setupTransparentWindow()
        
        // 创建UI
        createUI()
        
        // 初始化对话系统
        initializeConversation()
        
        // 检查权限并开始
        checkPermissionAndStart()
    }
    
    /**
     * 设置透明窗口属性
     */
    private fun setupTransparentWindow() {
        window.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND)
            setDimAmount(0.6f)
        }
    }
    
    /**
     * 初始化对话系统
     */
    private fun initializeConversation() {
        val config = AgentConfigActivity.getAgentConfig(this)
        val orderLog = config.voiceModeOrder.joinToString(" -> ")
        Log.i(TAG, "📱 语音回退顺序：$orderLog")

        // 初始化 TTS
        ttsService = AndroidTTSService(this)

        // 初始化对话适配器（传 serverUrl 以支持云端 ASR 兜底）
        voiceAdapter = ConversationalVoiceAdapter(
            this,
            config.cliServerUrl.ifBlank { "http://43.139.149.158:8080" }
        ).apply {
            setTTSService(ttsService)

            // 按回退顺序依次尝试，使用第一个有配置的模式
            var activatedMode = AgentConfigActivity.VOICE_MODE_SIMPLE
            val skipped = mutableListOf<String>()

            // 🔑 重构核心：API Key 模式和 CLI 模式共用同一条「智能管线」
            //   意图分析：SmartIntentAnalyzerAdapter（本地规则 + AI 判断完整性/意图）
            //   响应生成：SmartResponseGenerator（闲聊→AI对话；操作→生成脚本并执行）
            // 二者唯一区别只是底层 AIClient 由 AIClientFactory 自动选：
            //   有 Key → 用自带 Key；没 Key 但已登录选了项目 → 自动走服务器 CLI。
            // 因此不再需要 CLI 专属的「透传 + 全发服务器」管线。
            fun activateSmartPipeline() {
                val intentAnalyzer = IntentAnalyzer(this@ConversationalVoiceActivity)
                setIntentAnalyzer(SmartIntentAnalyzerAdapter(intentAnalyzer))
                setResponseGenerator(SmartResponseGenerator(this@ConversationalVoiceActivity))
            }

            run cascade@{
                for (mode in config.voiceModeOrder) {
                    when (mode) {
                        AgentConfigActivity.VOICE_MODE_APIKEY -> {
                            if (config.hunyuanApiKey.isNotEmpty() || config.openaiApiKey.isNotEmpty()) {
                                activateSmartPipeline()
                                activatedMode = mode
                                Log.i(TAG, "✅ 智能管线已激活（自带 Key，优先级 #${config.voiceModeOrder.indexOf(mode) + 1}）")
                                return@cascade
                            } else {
                                skipped.add("混元/OpenAI API Key（未填写）")
                                Log.i(TAG, "⏭️ API Key 未配置，跳过")
                            }
                        }
                        AgentConfigActivity.VOICE_MODE_CLI -> {
                            // 优先读主 UI 当前项目，没有才退到 AgentConfig.cliProjectId
                            val effectiveProjectId = com.elon.app.agent.infrastructure.auth.MainAppBridge
                                .effectiveCliProjectId(this@ConversationalVoiceActivity)
                                ?: ""
                            if (effectiveProjectId.isNotEmpty()) {
                                // 同一条智能管线：意图分析在本地/AI 做，闲聊与脚本都通过
                                // AIClientFactory 自动走服务器 CLI（用户已登录且选了项目）。
                                activateSmartPipeline()
                                activatedMode = mode
                                Log.i(TAG, "✅ 智能管线已激活（服务器 CLI，project=$effectiveProjectId）")
                                return@cascade
                            } else {
                                skipped.add("服务器 CLI（未登录主 UI 或未选项目）")
                                Log.i(TAG, "⏭️ CLI Project ID 未配置，跳过")
                            }
                        }
                        AgentConfigActivity.VOICE_MODE_SIMPLE -> {
                            activatedMode = mode
                            Log.i(TAG, "ℹ️ 简单模式（关键词匹配）")
                            return@cascade
                        }
                    }
                }
                // voiceModeOrder 里没有 simple 时的兜底
                Log.i(TAG, "ℹ️ 兜底：简单模式")
            }

            // Toast 汇报激活的模式（+ 跳过了哪些）
            val modeLabel = when (activatedMode) {
                AgentConfigActivity.VOICE_MODE_APIKEY -> "智能模式（自带 AI Key）"
                AgentConfigActivity.VOICE_MODE_CLI    -> "智能模式（服务器 AI）"
                else                                  -> "简单模式（关键词匹配）"
            }
            val skippedDesc = skipped.joinToString("、")
            val msg = if (skipped.isEmpty()) modeLabel else "$modeLabel\n（已跳过：$skippedDesc）"
            Toast.makeText(this@ConversationalVoiceActivity, msg, Toast.LENGTH_SHORT).show()

            // 设置 UI 监听器
            listener = createConversationListener()

            // 设置任务执行回调
            onTaskExecute = { goal ->
                executeTask(goal)
            }
        }
        
        // 监听状态变化，更新 UI
        lifecycleScope.launch {
            voiceAdapter.currentState.collectLatest { state ->
                updateUIForState(state)
            }
        }
    }
    
    /**
     * 创建对话监听器
     */
    private fun createConversationListener() = object : VoiceConversationListener {
        
        override fun onListeningStarted() {
            runOnUiThread {
                statusText.text = "正在聆听..."
                voiceIndicator.text = "🔴"
            }
        }
        
        override fun onUserSpeaking(text: String) {
            runOnUiThread {
                resultText.text = text
            }
        }
        
        override fun onUserInputComplete(text: String) {
            runOnUiThread {
                resultText.text = text
            }
        }
        
        override fun onThinking() {
            runOnUiThread {
                statusText.text = "思考中..."
                voiceIndicator.text = "🧠"
            }
        }

        override fun onProgress(step: String) {
            runOnUiThread {
                // 在思考阶段实时展示 AI 处理步骤（截取前 30 字，避免截断 UI）
                statusText.text = step.take(30)
            }
        }
        
        override fun onAssistantResponse(text: String) {
            runOnUiThread {
                responseText.text = text
                responseText.visibility = android.view.View.VISIBLE
            }
        }
        
        override fun onAssistantSpeaking(text: String) {
            runOnUiThread {
                statusText.text = "正在回应..."
                voiceIndicator.text = "🗣️"
            }
        }
        
        override fun onExecuting(goal: String) {
            runOnUiThread {
                statusText.text = "正在执行..."
                voiceIndicator.text = "🚀"
            }
        }
        
        override fun onExecutionComplete(success: Boolean, result: String) {
            runOnUiThread {
                if (success) {
                    Toast.makeText(this@ConversationalVoiceActivity, "✅ $result", Toast.LENGTH_SHORT).show()
                } else {
                    Toast.makeText(this@ConversationalVoiceActivity, "❌ $result", Toast.LENGTH_SHORT).show()
                }
                handler.postDelayed({ finish() }, 500)
            }
        }
        
        override fun onError(error: String) {
            runOnUiThread {
                Log.w(TAG, "⚠️ 收到错误: $error")
                
                // 可恢复错误列表（不关闭窗口，等待用户重试）
                val isRecoverableError = error.contains("客户端") || 
                                         error.contains("client") ||
                                         error.contains("重试") ||
                                         error.contains("超时") ||  // 网络超时也是可恢复的
                                         error.contains("timeout") ||
                                         error.contains("网络")  // 网络问题
                
                if (isRecoverableError) {
                    // 可恢复错误，显示提示并等待用户重新点击
                    Log.i(TAG, "📍 可恢复错误，等待用户重试")
                    statusText.text = "🎤 点击重试"
                    voiceIndicator.text = "🎤"
                    // 不关闭窗口，让用户可以点击重试
                } else {
                    // 严重错误（如权限问题），显示后关闭
                    statusText.text = "错误: $error"
                    voiceIndicator.text = "❌"
                    handler.postDelayed({ finish() }, 2000)
                }
            }
        }
        
        override fun onIdle() {
            // 对话结束，自动开启下一轮监听
            runOnUiThread {
                statusText.text = "\uD83C\uDFA4 继续说\u2026"
                voiceIndicator.text = "\uD83C\uDFA4"

                // 回复后 0.8s 自动重启监听：用户说完一句、听到回复，再说下一句，中间不需要点击任何东西
                handler.postDelayed({
                    if (voiceAdapter.currentState.value == ConversationState.IDLE && !isFinishing) {
                        Log.i(TAG, "\uD83D\uDD04 \u81ea\u52a8\u5f00\u59cb\u4e0b\u4e00\u8f6e\u5bf9\u8bdd")
                        resultText.text = ""
                        responseText.text = ""
                        responseText.visibility = android.view.View.GONE
                        voiceAdapter.restartListening()
                    }
                }, 800) // 从 3000ms 缩短到 800ms，回复后迅速重启等待用户继续说话
            }
        }
    }
    
    /**
     * 根据状态更新 UI
     */
    private fun updateUIForState(state: ConversationState) {
        runOnUiThread {
            when (state) {
                ConversationState.IDLE -> {
                    voiceIndicator.text = "🎤"
                    statusText.text = "🎤 点击或等待自动开始"
                }
                ConversationState.LISTENING -> {
                    voiceIndicator.text = "🔴"
                    statusText.text = "正在聆听..."
                }
                ConversationState.THINKING -> {
                    voiceIndicator.text = "🧠"
                    statusText.text = "思考中..."
                }
                ConversationState.SPEAKING -> {
                    voiceIndicator.text = "🗣️"
                    statusText.text = "正在回应..."
                }
                ConversationState.EXECUTING -> {
                    voiceIndicator.text = "🚀"
                    statusText.text = "正在执行..."
                }
                ConversationState.INTERRUPTED -> {
                    voiceIndicator.text = "⚡"
                    statusText.text = "已打断"
                }
                ConversationState.AWAITING_MORE -> {
                    voiceIndicator.text = "🟡"
                    statusText.text = "继续说..."
                }
            }
        }
    }
    
    /**
     * 创建 UI 布局
     */
    private fun createUI() {
        val density = resources.displayMetrics.density
        
        val rootLayout = FrameLayout(this).apply {
            setBackgroundColor(Color.TRANSPARENT)
            setOnClickListener { cancelAndClose() }
        }
        
        // 中央卡片
        val card = createCard(density)
        
        // 语音指示器 - 可点击重新开始
        voiceIndicator = TextView(this).apply {
            text = "🎤"
            textSize = 56f
            gravity = Gravity.CENTER
            isClickable = true  // 确保可点击
            isFocusable = true  // 确保可聚焦
            // 👇 点击指示器重新开始对话
            setOnClickListener { v ->
                // 阻止事件继续传播
                v.parent?.requestDisallowInterceptTouchEvent(true)
                
                val currentState = voiceAdapter.currentState.value
                Log.i(TAG, "🔄 点击麦克风，当前状态: $currentState")
                
                when (currentState) {
                    ConversationState.IDLE -> {
                        Log.i(TAG, "🔄 用户点击重新开始对话")
                        resultText.text = ""
                        responseText.text = ""
                        responseText.visibility = android.view.View.GONE
                        voiceAdapter.restartListening()
                    }
                    ConversationState.SPEAKING -> {
                        // 正在说话时点击 = 打断
                        Log.i(TAG, "⚡ 用户点击打断")
                        voiceAdapter.interrupt()
                    }
                    else -> {
                        // 其他状态不处理，但也不关闭窗口
                        Log.d(TAG, "📍 当前状态 $currentState，忽略点击")
                    }
                }
            }
        }
        card.addView(voiceIndicator, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { gravity = Gravity.CENTER })
        
        // 状态文字
        statusText = TextView(this).apply {
            text = "正在准备..."
            textSize = 16f
            setTextColor(Color.parseColor("#F2F5FA"))
            gravity = Gravity.CENTER
        }
        card.addView(statusText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (12 * density).toInt()
        })
        
        // 用户输入文字
        resultText = TextView(this).apply {
            text = ""
            textSize = 18f
            setTextColor(Color.parseColor("#6091CF"))
            gravity = Gravity.CENTER
            maxLines = 5
            minHeight = (60 * density).toInt()
        }
        card.addView(resultText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (16 * density).toInt()
        })
        
        // 助手响应文字
        responseText = TextView(this).apply {
            text = ""
            textSize = 14f
            setTextColor(Color.parseColor("#58BE6A"))
            gravity = Gravity.CENTER
            maxLines = 3
            visibility = android.view.View.GONE
        }
        card.addView(responseText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (8 * density).toInt()
        })
        
        // 取消按钮
        cancelButton = Button(this).apply {
            text = "❌ 取消"
            textSize = 14f
            setTextColor(Color.parseColor("#F2F5FA"))
            val bg = GradientDrawable().apply {
                setColor(Color.parseColor("#283140"))
                cornerRadius = 20 * density
            }
            background = bg
            setPadding((24 * density).toInt(), (10 * density).toInt(), (24 * density).toInt(), (10 * density).toInt())
            setOnClickListener { cancelAndClose() }
        }
        card.addView(cancelButton, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (20 * density).toInt()
        })
        
        // 提示
        val tipText = TextView(this).apply {
            text = "说完会自动执行，点击空白处取消"
            textSize = 11f
            setTextColor(Color.parseColor("#6F7785"))
            gravity = Gravity.CENTER
        }
        card.addView(tipText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (12 * density).toInt()
        })
        
        rootLayout.addView(card, FrameLayout.LayoutParams(
            (320 * density).toInt(),
            FrameLayout.LayoutParams.WRAP_CONTENT
        ).apply { gravity = Gravity.CENTER })
        
        setContentView(rootLayout)
    }
    
    /**
     * 创建卡片容器
     */
    private fun createCard(density: Float): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(
                (32 * density).toInt(),
                (32 * density).toInt(),
                (32 * density).toInt(),
                (24 * density).toInt()
            )
            
            val bg = GradientDrawable().apply {
                setColor(Color.parseColor("#181B20"))
                cornerRadius = 24 * density
            }
            background = bg
            elevation = 16 * density
            isClickable = true
            isFocusable = true
            // 点击卡片区域 = 重新开始对话（IDLE状态时）
            setOnClickListener { 
                val currentState = voiceAdapter.currentState.value
                Log.d(TAG, "📍 点击卡片，当前状态: $currentState")
                
                if (currentState == ConversationState.IDLE) {
                    Log.i(TAG, "🔄 用户点击卡片重新开始对话")
                    resultText.text = ""
                    responseText.text = ""
                    responseText.visibility = android.view.View.GONE
                    voiceAdapter.restartListening()
                }
                // 其他状态不做任何事，也不关闭窗口
            }
        }
    }
    
    /**
     * 检查权限并开始
     */
    private fun checkPermissionAndStart() {
        when {
            ContextCompat.checkSelfPermission(
                this, Manifest.permission.RECORD_AUDIO
            ) == PackageManager.PERMISSION_GRANTED -> {
                startConversation()
            }
            else -> {
                requestPermissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
            }
        }
    }
    
    /**
     * 开始对话
     */
    private fun startConversation() {
        statusText.text = "请说话..."
        voiceIndicator.text = "🔴"
        voiceAdapter.start()
    }
    
    /**
     * 执行任务
     */
    private fun executeTask(goal: String) {
        if (!com.elon.app.agent.AgentService.isRunning()) {
            Toast.makeText(this, "❌ 请先开启无障碍服务", Toast.LENGTH_LONG).show()
            finish()
            return
        }
        
        com.elon.app.agent.AgentService.executeTask(goal)
        Toast.makeText(this, "🚀 $goal", Toast.LENGTH_SHORT).show()
        finish()
    }
    
    /**
     * 取消并关闭
     */
    private fun cancelAndClose() {
        voiceAdapter.stop()
        handler.removeCallbacksAndMessages(null)
        finish()
    }
    
    // ==================== 生命周期管理 ====================

    /**
     * 🔁 再次点悬浮球时（singleInstance 实例已存在）走这里，而不是 onCreate。
     *
     * 修复：之前未重写 onNewIntent，导致完成一次语音命令（如打开微信）后
     * 再点悬浮球，窗口出来了却不重启 ASR，表现为“听不到说话”。
     */
    override fun onNewIntent(intent: android.content.Intent?) {
        super.onNewIntent(intent)
        setIntent(intent)
        Log.i(TAG, "🔁 onNewIntent：重新唤起语音对话，重启聚听")
        // 清理上一轮文字，重新开始聚听
        resultText.text = ""
        responseText.text = ""
        responseText.visibility = android.view.View.GONE
        statusText.text = "请说话..."
        voiceIndicator.text = "🔴"
        // 延迟到 onResume 之后再重启，确保窗口已在前台能拿到麦克风
        handler.postDelayed({
            if (!isFinishing) voiceAdapter.restartListening()
        }, 300)
    }

    override fun onResume() {
        super.onResume()
        isActivityResumed = true
        // 只在「从后台回来」时才补重启，首次启动由 startConversation 负责，不在这里干预。
        if (hasEverPaused) {
            handler.postDelayed({
                if (!isFinishing && voiceAdapter.currentState.value == ConversationState.IDLE) {
                    voiceAdapter.restartListening()
                }
            }, 500)
        }
    }

    override fun onPause() {
        super.onPause()
        isActivityResumed = false
        hasEverPaused = true
        // 退到后台时停采收 ASR，避免后台空跑麦克风。
        voiceAdapter.stop()
    }

    override fun onDestroy() {
        super.onDestroy()
        handler.removeCallbacksAndMessages(null)
        voiceAdapter.destroy()
        ttsService?.destroy()
    }
}
