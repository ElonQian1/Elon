// interface/AgentExecuteActivity.kt
package com.elon.app.agent

import android.Manifest
import android.app.Activity
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.method.ScrollingMovementMethod
import android.view.Gravity
import android.view.View
import android.widget.*
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import com.elon.app.agent.infrastructure.debug.DebugInterface
import com.elon.app.agent.infrastructure.floating.FloatingBallService
import com.elon.app.agent.infrastructure.floating.ReportSubmitDialog
import com.elon.app.agent.infrastructure.floating.ConversationalVoiceActivity
import com.elon.app.agent.infrastructure.voice.VoiceRecognitionHelper

private const val APP_BG = "#0B1118"
private const val CARD_BG = "#0E1116"
private const val SUBTLE_BG = "#080B0F"
private const val TEXT_PRIMARY = "#F8F7F4"
private const val TEXT_SECONDARY = "#B3DDDBD5"
private const val TEXT_TERTIARY = "#80BEBEBA"
private const val BUTTON_SECONDARY_BG = "#20262E"
private const val BUTTON_SECONDARY_TEXT = "#F8F7F4"
private const val BUTTON_PRIMARY_BG = "#F8F7F4"
private const val BUTTON_PRIMARY_TEXT = "#0B1118"
private const val LINK_PRIMARY = "#67BEA0"
private const val BADGE_INFO_BG = "#151A20"
private const val BADGE_INFO_TEXT = "#67BEA0"
private const val DANGER_TEXT = "#E07B84"

/**
 * 🎯 Agent 独立执行界面
 * 
 * 功能：
 * - 输入目标，直接让 AI 生成并执行脚本
 * - 🎤 语音输入任务目标
 * - 显示执行日志和进度
 * - 完全独立，不依赖 PC 端
 */
class AgentExecuteActivity : Activity() {
    
    private lateinit var goalInput: EditText
    private lateinit var executeButton: Button
    private lateinit var stopButton: Button
    private lateinit var voiceButton: Button
    private lateinit var presetButtonsLayout: LinearLayout
    private lateinit var progressBar: ProgressBar
    private lateinit var progressText: TextView
    private lateinit var logOutput: TextView
    private lateinit var statusIndicator: View
    private lateinit var voiceStatusText: TextView
    private lateinit var viewReportButton: Button  // 📊 查看报告按钮
    
    private var isExecuting = false
    private val handler = Handler(Looper.getMainLooper())
    
    // 语音识别
    private var voiceHelper: VoiceRecognitionHelper? = null
    
    // 接收执行日志的广播
    private val logReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val log = intent?.getStringExtra("log") ?: return
            appendLog(log)
        }
    }
    
    // 接收进度更新的广播
    private val progressReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val current = intent?.getIntExtra("current", 0) ?: 0
            val total = intent?.getIntExtra("total", 0) ?: 0
            val stepName = intent?.getStringExtra("step_name") ?: ""
            updateProgress(current, total, stepName)
        }
    }
    
    // 接收执行完成的广播
    private val completeReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val success = intent?.getBooleanExtra("success", false) ?: false
            val result = intent?.getStringExtra("result") ?: ""
            onExecutionComplete(success, result)
        }
    }
    
    // 接收聊天回复的广播（当用户意图是日常聊天而非手机操作时）
    private val chatResponseReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val response = intent?.getStringExtra("response") ?: return
            showChatResponse(response)
        }
    }
    
    // 📊 接收报告可用的广播
    private val reportAvailableReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val shouldReport = intent?.getBooleanExtra("should_report", false) ?: false
            val perfScore = intent?.getStringExtra("performance_score") ?: "UNKNOWN"
            if (shouldReport) {
                showReportButton(perfScore)
            }
        }
    }
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(createLayout())
        
        // 初始化语音识别
        initVoiceRecognition()
        
        // 注册广播接收器
        LocalBroadcastManager.getInstance(this).apply {
            registerReceiver(logReceiver, IntentFilter("agent.log"))
            registerReceiver(progressReceiver, IntentFilter("agent.progress"))
            registerReceiver(completeReceiver, IntentFilter("agent.complete"))
            registerReceiver(chatResponseReceiver, IntentFilter("agent.chat_response"))
            registerReceiver(reportAvailableReceiver, IntentFilter("agent.report_available"))
        }
        
        // 检查是否有预设任务要执行
        handlePresetIntent(intent)
    }
    
    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        intent?.let { handlePresetIntent(it) }
    }
    
    override fun onDestroy() {
        super.onDestroy()
        voiceHelper?.destroy()
        LocalBroadcastManager.getInstance(this).apply {
            unregisterReceiver(logReceiver)
            unregisterReceiver(progressReceiver)
            unregisterReceiver(completeReceiver)
            unregisterReceiver(chatResponseReceiver)
            unregisterReceiver(reportAvailableReceiver)
        }
    }
    
    private fun handlePresetIntent(intent: Intent) {
        val presetGoal = intent.getStringExtra("preset_goal")
        val autoExecute = intent.getBooleanExtra("auto_execute", false)
        
        if (!presetGoal.isNullOrEmpty()) {
            goalInput.setText(presetGoal)
            if (autoExecute) {
                handler.postDelayed({ executeGoal() }, 500)
            }
        }
    }
    
    private fun createLayout(): View {
        val scrollView = ScrollView(this)
        
        val mainLayout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
            setBackgroundColor(Color.parseColor(APP_BG))
        }
        
        // === 标题区 ===
        mainLayout.addView(createHeader())
        
        // === 状态指示器 ===
        statusIndicator = View(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 8
            ).apply { setMargins(0, 0, 0, 16) }
            setBackgroundColor(Color.parseColor(TEXT_TERTIARY))
        }
        mainLayout.addView(statusIndicator)
        
        // === 输入区 ===
        mainLayout.addView(createInputSection())
        
        // === 预设快捷按钮 ===
        mainLayout.addView(createPresetSection())
        
        // === 进度区 ===
        mainLayout.addView(createProgressSection())
        
        // === 日志区 ===
        mainLayout.addView(createLogSection())
        
        // === 悬浮球开关 ===
        mainLayout.addView(createFloatingBallSection())
        
        scrollView.addView(mainLayout)
        return scrollView
    }
    
    private fun createHeader(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, 0, 0, 16)
            
            addView(TextView(this@AgentExecuteActivity).apply {
                text = "🤖"
                textSize = 32f
            })
            
            addView(LinearLayout(this@AgentExecuteActivity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(16, 0, 0, 0)
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                
                addView(TextView(this@AgentExecuteActivity).apply {
                text = "AI Agent"
                textSize = 20f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(TEXT_PRIMARY))
            })
            addView(TextView(this@AgentExecuteActivity).apply {
                text = "语音/文字输入，自动执行任务"
                textSize = 12f
                setTextColor(Color.parseColor(TEXT_SECONDARY))
            })
            })
            
            // ⚙️ 设置按钮
            addView(Button(this@AgentExecuteActivity).apply {
                text = "⚙️"
            textSize = 18f
            setBackgroundColor(Color.TRANSPARENT)
            setTextColor(Color.parseColor(LINK_PRIMARY))
            setOnClickListener {
                startActivity(Intent(this@AgentExecuteActivity, AgentConfigActivity::class.java))
            }
            })
        }
    }
    
    private fun createInputSection(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD_BG))
            setPadding(24, 24, 24, 24)
            
            // 标题行（带语音按钮）
            val titleRow = LinearLayout(this@AgentExecuteActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                
                addView(TextView(this@AgentExecuteActivity).apply {
                    text = "🎯 输入任务目标"
                    textSize = 16f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(TEXT_PRIMARY))
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                
                // 语音输入按钮
                voiceButton = Button(this@AgentExecuteActivity).apply {
                    text = "🎤 语音"
                    textSize = 12f
                    setBackgroundColor(Color.parseColor(BUTTON_SECONDARY_BG))
                    setTextColor(Color.parseColor(BUTTON_SECONDARY_TEXT))
                    setOnClickListener { toggleVoiceInput() }
                }
                addView(voiceButton)
            }
            addView(titleRow)
            
            // 语音状态提示
            voiceStatusText = TextView(this@AgentExecuteActivity).apply {
                text = ""
                textSize = 12f
                setTextColor(Color.parseColor(LINK_PRIMARY))
                visibility = View.GONE
                gravity = Gravity.CENTER
                setPadding(0, 8, 0, 0)
            }
            addView(voiceStatusText)
            
            goalInput = EditText(this@AgentExecuteActivity).apply {
                hint = "输入任务或点击🎤语音输入\n例如：打开微信，给张三发消息"
                minLines = 3
                maxLines = 5
                gravity = Gravity.TOP
                setTextColor(Color.parseColor(TEXT_PRIMARY))
                setHintTextColor(Color.parseColor(TEXT_TERTIARY))
                setBackgroundColor(Color.parseColor(SUBTLE_BG))
                setPadding(16, 16, 16, 16)
            }
            addView(goalInput, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(0, 12, 0, 16) })
            
            val buttonLayout = LinearLayout(this@AgentExecuteActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER
            }
            
            executeButton = Button(this@AgentExecuteActivity).apply {
                text = "▶️ 执行任务"
                setBackgroundColor(Color.parseColor(BUTTON_PRIMARY_BG))
                setTextColor(Color.parseColor(BUTTON_PRIMARY_TEXT))
                setOnClickListener { executeGoal() }
            }
            buttonLayout.addView(executeButton, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            
            stopButton = Button(this@AgentExecuteActivity).apply {
                text = "⏹️ 停止"
                setBackgroundColor(Color.parseColor(BUTTON_SECONDARY_BG))
                setTextColor(Color.parseColor(BUTTON_SECONDARY_TEXT))
                isEnabled = false
                setOnClickListener { stopExecution() }
            }
            buttonLayout.addView(stopButton, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                setMargins(16, 0, 0, 0)
            })
            
            addView(buttonLayout)
        }.also { layout ->
            layout.layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(0, 0, 0, 16) }
        }
    }
    
    private fun createPresetSection(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD_BG))
            setPadding(24, 24, 24, 24)
            
            addView(TextView(this@AgentExecuteActivity).apply {
                text = "⚡ 常用任务"
                textSize = 14f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(TEXT_PRIMARY))
            })
            
            presetButtonsLayout = LinearLayout(this@AgentExecuteActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                setPadding(0, 12, 0, 0)
            }
            
            // 添加预设按钮
            val presets = listOf(
                "打开小红书" to "打开小红书",
                "热门笔记" to "打开小红书，找到点赞过万的笔记",
                "查看评论" to "打开小红书，进入第一个笔记，查看评论"
            )
            
            presets.forEach { (label, goal) ->
                presetButtonsLayout.addView(Button(this@AgentExecuteActivity).apply {
                    text = label
                    textSize = 12f
                    setBackgroundColor(Color.parseColor(BUTTON_SECONDARY_BG))
                    setTextColor(Color.parseColor(BUTTON_SECONDARY_TEXT))
                    setOnClickListener {
                        goalInput.setText(goal)
                    }
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                    setMargins(4, 0, 4, 0)
                })
            }
            
            addView(presetButtonsLayout)
        }.also { layout ->
            layout.layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(0, 0, 0, 16) }
        }
    }
    
    private fun createProgressSection(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD_BG))
            setPadding(24, 24, 24, 24)
            visibility = View.GONE
            tag = "progress_section"
            
            progressText = TextView(this@AgentExecuteActivity).apply {
                text = "准备中..."
                textSize = 14f
                setTextColor(Color.parseColor(TEXT_SECONDARY))
            }
            addView(progressText)
            
            progressBar = ProgressBar(this@AgentExecuteActivity, null, android.R.attr.progressBarStyleHorizontal).apply {
                max = 100
                progress = 0
            }
            addView(progressBar, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(0, 8, 0, 0) })
            
            // 📊 查看报告按钮（默认隐藏）
            viewReportButton = Button(this@AgentExecuteActivity).apply {
                text = "📋 查看执行报告"
                textSize = 14f
                setTextColor(Color.parseColor(BUTTON_SECONDARY_TEXT))
                setBackgroundColor(Color.parseColor(BUTTON_SECONDARY_BG))
                visibility = View.GONE
                setOnClickListener { showExecutionReport() }
            }
            addView(viewReportButton, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(0, 16, 0, 0) })
        }.also { layout ->
            layout.layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(0, 0, 0, 16) }
        }
    }
    
    private fun createLogSection(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD_BG))
            setPadding(24, 24, 24, 24)
            
            addView(LinearLayout(this@AgentExecuteActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                
                addView(TextView(this@AgentExecuteActivity).apply {
                    text = "📋 执行日志"
                    textSize = 14f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(TEXT_PRIMARY))
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                
                addView(Button(this@AgentExecuteActivity).apply {
                    text = "清空"
                    textSize = 12f
                    setBackgroundColor(Color.parseColor(BUTTON_SECONDARY_BG))
                    setTextColor(Color.parseColor(BUTTON_SECONDARY_TEXT))
                    setOnClickListener { logOutput.text = "" }
                })
            })
            
            logOutput = TextView(this@AgentExecuteActivity).apply {
                text = "等待执行...\n"
                textSize = 11f
                setTextColor(Color.parseColor(TEXT_SECONDARY))
                setBackgroundColor(Color.parseColor(SUBTLE_BG))
                setPadding(12, 12, 12, 12)
                maxLines = 50
                movementMethod = ScrollingMovementMethod()
            }
            addView(logOutput, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 300
            ).apply { setMargins(0, 8, 0, 0) })
        }.also { layout ->
            layout.layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
    }
    
    // === 执行控制 ===
    
    private fun executeGoal() {
        val goal = goalInput.text.toString().trim()
        if (goal.isEmpty()) {
            Toast.makeText(this, "请输入任务目标", Toast.LENGTH_SHORT).show()
            return
        }
        
        if (isExecuting) {
            Toast.makeText(this, "正在执行中...", Toast.LENGTH_SHORT).show()
            return
        }
        
        isExecuting = true
        updateUI(executing = true)
        appendLog("🧠 分析用户意图: $goal")
        
        // 使用智能执行：先分析意图，再决定流程
        val intent = Intent("agent.smart_execute").apply {
            putExtra("goal", goal)
        }
        LocalBroadcastManager.getInstance(this).sendBroadcast(intent)
    }
    
    /**
     * 处理聊天回复（非操作命令时显示）
     */
    private fun showChatResponse(response: String) {
        handler.post {
            isExecuting = false
            updateUI(executing = false)
            appendLog("💬 AI 回复: $response")
            
            // 可选：用 Toast 或对话框显示
            Toast.makeText(this, response, Toast.LENGTH_LONG).show()
        }
    }
    
    private fun stopExecution() {
        appendLog("⏹️ 用户请求停止...")
        
        val intent = Intent("agent.stop")
        LocalBroadcastManager.getInstance(this).sendBroadcast(intent)
        
        isExecuting = false
        updateUI(executing = false)
    }
    
    private fun updateUI(executing: Boolean) {
        executeButton.isEnabled = !executing
        stopButton.isEnabled = executing
        goalInput.isEnabled = !executing
        
        statusIndicator.setBackgroundColor(
            Color.parseColor(if (executing) LINK_PRIMARY else TEXT_TERTIARY)
        )
        
        // 手动查找进度区
        (window.decorView as? android.view.ViewGroup)?.let { root ->
            findViewByTag(root, "progress_section")?.visibility = 
                if (executing) View.VISIBLE else View.GONE
        }
    }
    
    private fun findViewByTag(parent: android.view.ViewGroup, tag: String): View? {
        for (i in 0 until parent.childCount) {
            val child = parent.getChildAt(i)
            if (child.tag == tag) return child
            if (child is android.view.ViewGroup) {
                findViewByTag(child, tag)?.let { return it }
            }
        }
        return null
    }
    
    private fun updateProgress(current: Int, total: Int, stepName: String) {
        handler.post {
            if (total > 0) {
                progressBar.max = total
                progressBar.progress = current
                progressText.text = "步骤 $current/$total: $stepName"
            }
        }
    }
    
    private fun appendLog(log: String) {
        handler.post {
            val timestamp = java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.getDefault())
                .format(java.util.Date())
            logOutput.append("[$timestamp] $log\n")
            
            // 自动滚动到底部
            val scrollAmount = logOutput.layout?.getLineTop(logOutput.lineCount) ?: 0
            logOutput.scrollTo(0, scrollAmount - logOutput.height)
        }
    }
    
    private fun onExecutionComplete(success: Boolean, result: String) {
        handler.post {
            isExecuting = false
            updateUI(executing = false)
            
            if (success) {
                statusIndicator.setBackgroundColor(Color.parseColor(LINK_PRIMARY))
                appendLog("✅ 执行成功!")
                if (result.isNotEmpty()) {
                    appendLog("📊 结果: $result")
                }
                Toast.makeText(this, "✅ 任务完成!", Toast.LENGTH_SHORT).show()
            } else {
                statusIndicator.setBackgroundColor(Color.parseColor(DANGER_TEXT))
                appendLog("❌ 执行失败: $result")
                Toast.makeText(this, "❌ 任务失败", Toast.LENGTH_SHORT).show()
            }
        }
    }
    
    // ==================== 语音识别相关 ====================
    
    private fun initVoiceRecognition() {
        // 检查权限
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) 
            != PackageManager.PERMISSION_GRANTED) {
            // 稍后在用户点击语音按钮时请求权限
            return
        }
        
        initializeVoiceHelper()
    }
    
    private fun initializeVoiceHelper() {
        voiceHelper = VoiceRecognitionHelper(this).apply {
            onResult = { result: String ->
                handler.post {
                    goalInput.setText(result as CharSequence)
                    goalInput.setSelection(result.length)
                    voiceStatusText.text = "✅ 识别完成，即将执行..."
                    appendLog("🎤 语音识别: $result")
                    
                    // 🆕 语音识别完成后自动执行（延迟 800ms 让用户看到识别结果）
                    if (result.isNotBlank() && !isExecuting) {
                        handler.postDelayed({
                            voiceStatusText.visibility = View.GONE
                            executeGoal()
                        }, 800)
                    } else {
                        voiceStatusText.visibility = View.GONE
                    }
                }
            }
            onPartialResult = { partial: String ->
                handler.post {
                    voiceStatusText.text = "🎤 正在听: $partial"
                }
            }
            onError = { error: String ->
                handler.post {
                    voiceStatusText.text = "❌ $error"
                    voiceButton.text = "🎤 语音"
                    voiceButton.setBackgroundColor(Color.parseColor(BUTTON_SECONDARY_BG))
                    voiceButton.setTextColor(Color.parseColor(BUTTON_SECONDARY_TEXT))
                    Toast.makeText(this@AgentExecuteActivity, error as CharSequence, Toast.LENGTH_SHORT).show()
                }
            }
            onListeningStateChanged = { listening: Boolean ->
                handler.post {
                    if (listening) {
                        voiceButton.text = "🔴 停止"
                        voiceButton.setBackgroundColor(Color.parseColor(BADGE_INFO_BG))
                        voiceButton.setTextColor(Color.parseColor(BADGE_INFO_TEXT))
                        voiceStatusText.text = "🎤 正在聆听..."
                        voiceStatusText.visibility = View.VISIBLE
                    } else {
                        voiceButton.text = "🎤 语音"
                        voiceButton.setBackgroundColor(Color.parseColor(BUTTON_SECONDARY_BG))
                        voiceButton.setTextColor(Color.parseColor(BUTTON_SECONDARY_TEXT))
                        // 不要在这里隐藏 voiceStatusText，让它显示识别结果
                    }
                }
            }
        }
        voiceHelper?.initialize()
    }
    
    private fun toggleVoiceInput() {
        // 🆕 使用新的智能对话 Activity（带 TTS 回应）
        val intent = Intent(this, ConversationalVoiceActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
        }
        startActivity(intent)
    }
    
    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        
        when (requestCode) {
            REQUEST_RECORD_AUDIO -> {
                if (grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                    Toast.makeText(this, "✅ 录音权限已授予", Toast.LENGTH_SHORT).show()
                    initializeVoiceHelper()
                    // 自动开始语音识别
                    voiceHelper?.startListening()
                } else {
                    Toast.makeText(this, "❌ 需要录音权限才能使用语音输入", Toast.LENGTH_LONG).show()
                }
            }
        }
    }
    
    companion object {
        private const val REQUEST_RECORD_AUDIO = 1001
        
        /**
         * 启动执行界面
         */
        fun start(context: Context, goal: String? = null, autoExecute: Boolean = false) {
            val intent = Intent(context, AgentExecuteActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                goal?.let { putExtra("preset_goal", it) }
                putExtra("auto_execute", autoExecute)
            }
            context.startActivity(intent)
        }
    }
    
    // ==================== 悬浮球相关 ====================
    
    private lateinit var floatingBallSwitch: Switch
    
    private fun createFloatingBallSection(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD_BG))
            setPadding(24, 24, 24, 24)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(0, 16, 0, 0) }
            
            // 标题行
            val titleRow = LinearLayout(this@AgentExecuteActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
            }
            
            // 左侧图标和文字
            val leftSection = LinearLayout(this@AgentExecuteActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                
                addView(TextView(this@AgentExecuteActivity).apply {
                    text = "🎈"
                    textSize = 24f
                })
                
                addView(LinearLayout(this@AgentExecuteActivity).apply {
                    orientation = LinearLayout.VERTICAL
                    setPadding(16, 0, 0, 0)
                    
                    addView(TextView(this@AgentExecuteActivity).apply {
                        text = "悬浮球"
                        textSize = 16f
                        setTypeface(null, Typeface.BOLD)
                        setTextColor(Color.parseColor(TEXT_PRIMARY))
                    })
                    addView(TextView(this@AgentExecuteActivity).apply {
                        text = "单击语音 | 双击文字"
                        textSize = 12f
                        setTextColor(Color.parseColor(TEXT_SECONDARY))
                    })
                })
            }
            titleRow.addView(leftSection)
            
            // 开关
            floatingBallSwitch = Switch(this@AgentExecuteActivity).apply {
                isChecked = FloatingBallService.isRunning
                setOnCheckedChangeListener { _, isChecked ->
                    android.util.Log.i("FloatingBallSwitch", "开关状态变化: $isChecked")
                    appendLog("🎈 悬浮球开关: $isChecked")
                    if (isChecked) {
                        startFloatingBall()
                    } else {
                        stopFloatingBall()
                    }
                }
            }
            titleRow.addView(floatingBallSwitch)
            
            addView(titleRow)
            
            // 说明文字
            addView(TextView(this@AgentExecuteActivity).apply {
                text = "开启后，悬浮球将显示在所有界面上方。\n• 单击悬浮球 → 语音输入任务\n• 双击悬浮球 → 文字输入任务\n• 长按拖动 → 移动位置"
                textSize = 12f
                setTextColor(Color.parseColor(TEXT_TERTIARY))
                setPadding(0, 16, 0, 0)
            })
            
            // 权限提示
            if (!FloatingBallService.canDrawOverlays(this@AgentExecuteActivity)) {
                addView(Button(this@AgentExecuteActivity).apply {
                    text = "⚠️ 需要悬浮窗权限，点击授权"
                    textSize = 12f
                    setBackgroundColor(Color.parseColor(BADGE_INFO_BG))
                    setTextColor(Color.parseColor(BADGE_INFO_TEXT))
                    setOnClickListener {
                        FloatingBallService.requestOverlayPermission(this@AgentExecuteActivity)
                    }
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { setMargins(0, 12, 0, 0) }
                })
            }
        }
    }
    
    private fun startFloatingBall() {
        android.util.Log.i("FloatingBall", "=== startFloatingBall() 被调用 ===")
        
        if (!FloatingBallService.canDrawOverlays(this)) {
            android.util.Log.w("FloatingBall", "没有悬浮窗权限")
            floatingBallSwitch.isChecked = false
            FloatingBallService.requestOverlayPermission(this)
            Toast.makeText(this, "请先授予悬浮窗权限", Toast.LENGTH_LONG).show()
            return
        }
        
        android.util.Log.i("FloatingBall", "权限检查通过，准备启动服务")
        
        // 设置任务提交回调
        FloatingBallService.onTaskSubmit = { goal ->
            android.util.Log.i("FloatingBall", "收到任务: $goal")
            // 直接调用 AgentService 执行任务
            if (AgentService.isRunning()) {
                android.util.Log.i("FloatingBall", "AgentService 正在运行，执行任务")
                AgentService.executeTask(goal)
                handler.post {
                    appendLog("📱 悬浮球任务: $goal")
                }
            } else {
                android.util.Log.w("FloatingBall", "AgentService 未运行")
                handler.post {
                    Toast.makeText(this, "请先开启无障碍服务", Toast.LENGTH_SHORT).show()
                }
            }
        }
        
        FloatingBallService.start(this)
        android.util.Log.i("FloatingBall", "FloatingBallService.start() 已调用")
        Toast.makeText(this, "🎈 悬浮球已开启", Toast.LENGTH_SHORT).show()
    }
    
    private fun stopFloatingBall() {
        FloatingBallService.stop(this)
        FloatingBallService.onTaskSubmit = null
        Toast.makeText(this, "悬浮球已关闭", Toast.LENGTH_SHORT).show()
    }
    
    // ============================================
    // 📊 执行报告相关方法
    // ============================================
    
    /**
     * 显示报告按钮（执行完成后调用）
     */
    private fun showReportButton(performanceScore: String) {
        handler.post {
            viewReportButton.visibility = View.VISIBLE
            viewReportButton.text = when (performanceScore) {
                "POOR" -> "⚠️ 执行存在问题，点击查看报告"
                "FAIR" -> "📊 执行完成，点击查看详细报告"
                else -> "📋 查看执行报告"
            }
            viewReportButton.setBackgroundColor(when (performanceScore) {
                "POOR" -> Color.parseColor(DANGER_TEXT)
                "FAIR" -> Color.parseColor(LINK_PRIMARY)
                else -> Color.parseColor(LINK_PRIMARY)
            })
            viewReportButton.setTextColor(Color.parseColor(
                if (performanceScore == "GOOD") BUTTON_PRIMARY_TEXT else BUTTON_SECONDARY_TEXT
            ))
        }
    }
    
    /**
     * 显示执行报告弹窗
     */
    private fun showExecutionReport() {
        val report = DebugInterface.getLastExecutionReport()
        if (report == null) {
            Toast.makeText(this, "暂无执行报告", Toast.LENGTH_SHORT).show()
            return
        }
        
        try {
            // ReportSubmitDialog 是实例类，需要传入 Context 创建实例
            ReportSubmitDialog(this).show()
        } catch (e: Exception) {
            android.util.Log.e("ExecutionReport", "显示报告弹窗失败: ${e.message}")
            Toast.makeText(this, "无法显示报告: ${e.message}", Toast.LENGTH_LONG).show()
        }
    }
    
    /**
     * 隐藏报告按钮（新任务开始时调用）
     */
    private fun hideReportButton() {
        handler.post {
            viewReportButton.visibility = View.GONE
        }
    }
    
    override fun onResume() {
        super.onResume()
        // 刷新悬浮球开关状态
        if (::floatingBallSwitch.isInitialized) {
            floatingBallSwitch.isChecked = FloatingBallService.isRunning
        }
    }
}
