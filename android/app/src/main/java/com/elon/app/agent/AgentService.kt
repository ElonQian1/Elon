package com.elon.app.agent

import android.accessibilityservice.AccessibilityButtonController
import android.accessibilityservice.AccessibilityService
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.util.Log
import android.view.accessibility.AccessibilityEvent
import android.widget.Toast
import androidx.core.app.NotificationCompat
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import com.elon.app.agent.application.LocalBasicActionExecutor
import com.elon.app.agent.application.ScriptEngine
import com.elon.app.agent.infrastructure.NotificationActionReceiver
import com.elon.app.agent.infrastructure.accessibility.*
import com.elon.app.agent.infrastructure.floating.ConversationalVoiceActivity
import com.elon.app.agent.infrastructure.floating.FloatingInputActivity
import kotlinx.coroutines.*

/**
 * AI Agent 无障碍服务
 * 
 * 负责：
 * - 提供手机控制能力（点击、滑动、按键等）
 * - 提供屏幕读取能力（UI树解析）
 * - 接收 PC 端的命令
 */
class AgentService : AccessibilityService() {
    private var socketServer: SocketServer? = null
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    
    // 核心组件
    private lateinit var gestureExecutor: AccessibilityGestureExecutor
    private lateinit var uiParser: UITreeParser
    private lateinit var screenReader: AccessibilityScreenReader
    
    // 🆕 智能屏幕读取器（支持三种模式）
    var smartScreenReader: SmartScreenReader? = null
        private set
    
    // 🆕 系统无障碍按钮控制器
    private var accessibilityButtonController: AccessibilityButtonController? = null
    private var accessibilityButtonCallback: AccessibilityButtonController.AccessibilityButtonCallback? = null
    
    companion object {
        const val NOTIFICATION_ID = 1001
        const val CHANNEL_ID = "agent_service_channel"
        private const val TAG = "AgentService"
        
        // 🆕 配置开关：是否启用自定义悬浮窗（默认不启用，使用系统无障碍按钮）
        var useCustomFloatingWindow: Boolean = false
        
        // 静态实例，供悬浮球等组件调用
        @Volatile
        private var instance: AgentService? = null
        
        /**
         * 获取服务实例
         */
        fun getInstance(): AgentService? = instance
        
        /**
         * 🔧 修复：从外部执行任务必须经过 InputGateway（统一入口）
         * 
         * 之前的问题：直接调用 executeGoalIndependently() 绕过了意图分析，
         * 导致说"你好"会错误地执行任务
         */
        fun executeTask(goal: String) {
            instance?.let { service ->
                // 使用 InputGateway 统一入口（包含意图分析 + 防抖）
                com.elon.app.agent.application.InputGateway.submit(
                    input = goal,
                    source = com.elon.app.agent.application.InputGateway.InputSource.FLOATING_BALL,
                    callback = object : com.elon.app.agent.application.InputGateway.InputCallback {
                        override fun onChatResponse(response: String) {
                            // 聊天响应 - 用 Toast 显示
                            android.os.Handler(android.os.Looper.getMainLooper()).post {
                                Toast.makeText(service.applicationContext, response, Toast.LENGTH_LONG).show()
                            }
                        }
                        
                        override fun onOperationStart(goal: String) {
                            // 操作开始 - 执行任务
                            service.executeGoalIndependently(goal)
                        }
                        
                        override fun onError(error: String) {
                            Log.e(TAG, "❌ InputGateway 错误: $error")
                            android.os.Handler(android.os.Looper.getMainLooper()).post {
                                Toast.makeText(service.applicationContext, error, Toast.LENGTH_SHORT).show()
                            }
                        }
                    }
                )
            } ?: run {
                Log.e(TAG, "❌ 无障碍服务未运行，无法执行任务")
            }
        }
        
        /**
         * 检查服务是否运行
         */
        fun isRunning(): Boolean = instance != null

        /**
         * 停止当前正在运行的 Agent 任务。
         * AgentActivity / 主 UI 可调用。服务未运行时为空操作。
         */
        fun stop() {
            instance?.stopCurrentExecution()
        }
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this  // 保存实例
        Log.i(TAG, "🚀 无障碍服务已连接")
        
        // 启动前台服务（防止被杀）
        startForegroundService()
        
        // 初始化核心组件
        initializeCoreComponents()
        
        // 🆕 初始化智能屏幕读取器（支持三种模式）
        smartScreenReader = SmartScreenReader(this)
        Log.i(TAG, "✅ SmartScreenReader 初始化完成")
        
        // 🆕 初始化系统无障碍按钮（Android 8.0+）
        setupSystemAccessibilityButton()
        
        // 启动 Socket 服务器（PC 通信）
        socketServer = SocketServer(this)
        socketServer?.loadSavedApiKey()  // 🆕 自动加载保存的 API Key
        socketServer?.start(11451)
        
        Log.i(TAG, "✅ Agent 服务已启动，等待 PC 端连接")
    }
    
    /**
     * 🆕 设置系统无障碍按钮（悬浮快捷方式）
     * 
     * 这是 Android 8.0+ 原生功能，无需悬浮窗权限
     * 用户可在 设置 → 无障碍 → 快捷方式 中配置触发方式
     */
    private fun setupSystemAccessibilityButton() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            try {
                // 从父类 AccessibilityService 获取控制器
                val controller = accessibilityButtonController
                accessibilityButtonController = controller
                
                if (controller == null) {
                    Log.w(TAG, "⚠️ 无障碍按钮控制器不可用（可能未在快捷方式中选择本服务）")
                    return
                }
                
                // 检查按钮是否可用
                val isAvailable = controller.isAccessibilityButtonAvailable
                Log.i(TAG, "🔘 无障碍按钮可用性: $isAvailable")
                
                accessibilityButtonCallback = object : AccessibilityButtonController.AccessibilityButtonCallback() {
                    override fun onClicked(controller: AccessibilityButtonController) {
                        Log.i(TAG, "🔘 系统无障碍按钮被点击")
                        onSystemAccessibilityButtonClicked()
                    }
                    
                    override fun onAvailabilityChanged(controller: AccessibilityButtonController, available: Boolean) {
                        Log.i(TAG, "🔘 系统无障碍按钮可用性变化: $available")
                    }
                }
                
                controller.registerAccessibilityButtonCallback(accessibilityButtonCallback!!)
                Log.i(TAG, "✅ 系统无障碍按钮回调已注册")
                
            } catch (e: Exception) {
                Log.e(TAG, "❌ 注册系统无障碍按钮失败", e)
            }
        } else {
            Log.w(TAG, "⚠️ Android 8.0 以下不支持系统无障碍按钮")
        }
    }
    
    /**
     * 显示语音输入界面（使用智能对话系统 V2）
     */
    private fun showVoiceInput() {
        try {
            val intent = Intent(this, ConversationalVoiceActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
            }
            startActivity(intent)
        } catch (e: Exception) {
            Log.e(TAG, "打开语音输入失败", e)
        }
    }
    
    /**
     * 显示文字输入界面
     */
    private fun showTextInput() {
        try {
            val intent = Intent(this, FloatingInputActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
            }
            startActivity(intent)
        } catch (e: Exception) {
            Log.e(TAG, "打开文字输入失败", e)
        }
    }
    
    /**
     * 打开主界面
     */
    private fun openMainActivity() {
        try {
            val intent = Intent(this, AgentExecuteActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            }
            startActivity(intent)
        } catch (e: Exception) {
            Log.e(TAG, "打开主界面失败", e)
        }
    }
    
    /**
     * 🆕 系统无障碍按钮点击回调
     * 
     * 点击后打开任务输入界面
     */
    private fun onSystemAccessibilityButtonClicked() {
        try {
            // 打开任务执行界面
            val intent = Intent(this, AgentExecuteActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            }
            startActivity(intent)
            
            // 震动反馈
            val vibrator = getSystemService(Context.VIBRATOR_SERVICE) as? android.os.Vibrator
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                vibrator?.vibrate(android.os.VibrationEffect.createOneShot(50, android.os.VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                @Suppress("DEPRECATION")
                vibrator?.vibrate(50)
            }
            
        } catch (e: Exception) {
            Log.e(TAG, "❌ 打开任务界面失败", e)
            Toast.makeText(this, "打开界面失败: ${e.message}", Toast.LENGTH_SHORT).show()
        }
    }
    
    /**
     * 初始化核心组件
     */
    private fun initializeCoreComponents() {
        try {
            gestureExecutor = AccessibilityGestureExecutor(this)
            uiParser = UITreeParser(this)
            screenReader = AccessibilityScreenReader(this)
            
            Log.i(TAG, "✅ 核心组件初始化完成")
            
        } catch (e: Exception) {
            Log.e(TAG, "❌ 核心组件初始化失败", e)
        }
    }
    
    /**
     * 获取手势执行器（供外部使用）
     */
    fun getGestureExecutor(): AccessibilityGestureExecutor = gestureExecutor
    
    /**
     * 获取 UI 解析器（供外部使用）
     */
    fun getUIParser(): UITreeParser = uiParser
    
    /**
     * 获取屏幕读取器（供外部使用）
     */
    fun getScreenReader(): AccessibilityScreenReader = screenReader
    
    private fun startForegroundService() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "AI Agent 服务",
                NotificationManager.IMPORTANCE_LOW
            )
            val manager = getSystemService(NotificationManager::class.java)
            manager?.createNotificationChannel(channel)
        }
        
        // 创建增强通知
        val notification = createEnhancedNotification()
        
        // Android 14+ 需要指定前台服务类型
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(NOTIFICATION_ID, notification, 
                android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        
        // 注册本地广播接收器（用于 Activity 通信）
        registerLocalBroadcastReceivers()
    }
    
    /**
     * 创建增强通知（带快捷操作按钮）
     */
    private fun createEnhancedNotification(): android.app.Notification {
        // 打开界面的 Intent
        val openIntent = Intent(this, NotificationActionReceiver::class.java).apply {
            action = NotificationActionReceiver.ACTION_OPEN_APP
        }
        val openPendingIntent = PendingIntent.getBroadcast(
            this, 0, openIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        
        // 快捷任务：打开小红书
        val xhsIntent = Intent(this, NotificationActionReceiver::class.java).apply {
            action = NotificationActionReceiver.ACTION_QUICK_TASK
            putExtra("task", NotificationActionReceiver.TASK_OPEN_XHS)
        }
        val xhsPendingIntent = PendingIntent.getBroadcast(
            this, 1, xhsIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        
        // 🆕 语音输入（最方便！）
        val voiceIntent = Intent(this, NotificationActionReceiver::class.java).apply {
            action = NotificationActionReceiver.ACTION_VOICE_INPUT
        }
        val voicePendingIntent = PendingIntent.getBroadcast(
            this, 3, voiceIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        
        // 🆕 文字输入
        val textIntent = Intent(this, NotificationActionReceiver::class.java).apply {
            action = NotificationActionReceiver.ACTION_TEXT_INPUT
        }
        val textPendingIntent = PendingIntent.getBroadcast(
            this, 4, textIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("🤖 AI Agent 运行中")
            .setContentText("点击打开 · 下拉使用语音输入")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .setContentIntent(openPendingIntent)
            // 🆕 语音输入放第一个（最方便！）
            .addAction(android.R.drawable.ic_btn_speak_now, "🎤 语音", voicePendingIntent)
            .addAction(android.R.drawable.ic_menu_edit, "⌨️ 文字", textPendingIntent)
            .addAction(android.R.drawable.ic_menu_send, "📱 小红书", xhsPendingIntent)
            .setStyle(NotificationCompat.BigTextStyle()
                .bigText("Agent 正在后台待命\n\n" +
                    "🎤 语音 - 说话即可下达任务（最方便）\n" +
                    "⌨️ 文字 - 输入文字任务\n" +
                    "📱 小红书 - 打开小红书应用"))
            .build()
    }
    
    // 脚本引擎实例（用于独立执行）
    private var scriptEngine: ScriptEngine? = null
    private var currentJob: Job? = null
    
    /**
     * 注册本地广播接收器
     */
    private fun registerLocalBroadcastReceivers() {
        val localBroadcastManager = LocalBroadcastManager.getInstance(this)
        
        // 🧠 智能执行广播（先分析意图）
        localBroadcastManager.registerReceiver(object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                val goal = intent?.getStringExtra("goal") ?: return
                executeSmartly(goal)
            }
        }, IntentFilter("agent.smart_execute"))
        
        // 执行任务广播（直接执行，跳过意图分析）
        localBroadcastManager.registerReceiver(object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                val goal = intent?.getStringExtra("goal") ?: return
                executeGoalIndependently(goal)
            }
        }, IntentFilter("agent.execute"))
        
        // 停止任务广播
        localBroadcastManager.registerReceiver(object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                stopCurrentExecution()
            }
        }, IntentFilter("agent.stop"))
    }
    
    /**
     * 🧠 智能执行（先分析意图）
     */
    private fun executeSmartly(userInput: String) {
        Log.i(TAG, "🧠 智能执行: $userInput")
        
        // 重构后：不再检查 apiKey，ScriptEngine 会通过 AIClientFactory 自动选择 AI 链路。
        // 如果用户既没填 Key 也没登录/选项目，chat() 调用时会抛可读异常。
        if (scriptEngine == null) {
            scriptEngine = ScriptEngine(this)
        }
        
        currentJob = scope.launch {
            try {
                // 第一步：分析意图
                sendLogBroadcast("🧠 分析用户意图...")
                val intentResult = scriptEngine?.analyzeIntent(userInput)
                
                if (intentResult == null) {
                    sendLogBroadcast("⚠️ 意图分析失败，尝试直接执行")
                    executeGoalIndependently(userInput)
                    return@launch
                }
                
                when (intentResult.intent) {
                    ScriptEngine.UserIntent.CHAT -> {
                        // 聊天意图 - 返回 AI 回复
                        val response = intentResult.chatResponse ?: "我是手机自动化助手，可以帮你操作手机。"
                        sendLogBroadcast("💬 这是日常对话，AI 回复:")
                        sendLogBroadcast("💬 $response")
                        
                        // 发送聊天回复广播
                        LocalBroadcastManager.getInstance(this@AgentService).sendBroadcast(
                            Intent("agent.chat_response")
                                .putExtra("response", response)
                        )
                        sendCompleteBroadcast(true, "")
                    }
                    
                    ScriptEngine.UserIntent.PHONE_OPERATION -> {
                        // 操作意图 - 执行脚本流程
                        val goal = intentResult.operationGoal ?: userInput
                        sendLogBroadcast("🎯 识别为手机操作: $goal")
                        executeGoalIndependently(goal)
                    }
                    
                    else -> {
                        // 不确定 - 默认执行
                        sendLogBroadcast("⚠️ 意图不明确，尝试执行")
                        executeGoalIndependently(userInput)
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "智能执行失败", e)
                sendLogBroadcast("❌ 智能执行失败: ${e.message}")
                sendCompleteBroadcast(false, e.message ?: "未知错误")
            }
        }
    }
    
    /**
     * 🎯 独立执行目标（不依赖 PC）
     */
    private fun executeGoalIndependently(goal: String) {
        Log.i(TAG, "🎯 开始独立执行: $goal")

        // ⚡ 基础动作本地直控：打开应用 / 返回 / 回桌面 / 最近任务
        // 这类动作无需 AI 生成脚本、也无需服务器，本地无障碍直接执行。
        when (val r = LocalBasicActionExecutor.tryExecute(this, goal)) {
            is LocalBasicActionExecutor.Result.Handled -> {
                sendLogBroadcast("⚡ 本地直控：${r.message}")
                sendCompleteBroadcast(true, r.message)
                return
            }
            LocalBasicActionExecutor.Result.NotHandled -> {
                // 不是纯基础动作，继续走 AI 脚本流程
            }
        }

        // 重构后：不再检查 apiKey。ScriptEngine 通过 AIClientFactory 选链路。
        if (scriptEngine == null) {
            scriptEngine = ScriptEngine(this)
        }
        
        currentJob = scope.launch {
            try {
                // 设置日志回调
                scriptEngine?.onLog = { log ->
                    sendLogBroadcast(log)
                }
                
                // 生成脚本
                sendLogBroadcast("📝 AI 正在生成脚本...")
                val generateResult = scriptEngine?.generateScript(goal)
                
                if (generateResult == null || generateResult.isFailure) {
                    sendLogBroadcast("❌ 脚本生成失败")
                    sendCompleteBroadcast(false, generateResult?.exceptionOrNull()?.message ?: "生成失败")
                    return@launch
                }
                
                val script = generateResult.getOrNull() ?: run {
                    sendCompleteBroadcast(false, "脚本为空")
                    return@launch
                }
                
                sendLogBroadcast("✅ 脚本生成成功: ${script.name} (${script.steps.size} 步)")
                
                // 执行脚本（带自动改进）
                sendLogBroadcast("▶️ 开始执行脚本...")
                val executeResult = scriptEngine?.executeWithAutoImprove(script.id) { current, total, stepName ->
                    // 发送进度广播
                    LocalBroadcastManager.getInstance(this@AgentService).sendBroadcast(
                        Intent("agent.progress")
                            .putExtra("current", current)
                            .putExtra("total", total)
                            .putExtra("step_name", stepName)
                    )
                }
                
                if (executeResult?.success == true) {
                    sendCompleteBroadcast(true, "")
                } else {
                    sendCompleteBroadcast(false, executeResult?.error ?: "执行失败")
                }
                
            } catch (e: Exception) {
                Log.e(TAG, "独立执行失败", e)
                sendLogBroadcast("❌ 错误: ${e.message}")
                sendCompleteBroadcast(false, e.message ?: "未知错误")
            }
        }
    }
    
    /**
     * 停止当前执行（供 companion AgentService.stop() 调用）
     */
    fun stopCurrentExecution() {
        currentJob?.cancel()
        currentJob = null
        sendLogBroadcast("⏹️ 已停止执行")
    }
    
    /**
     * 发送日志广播
     */
    private fun sendLogBroadcast(log: String) {
        LocalBroadcastManager.getInstance(this).sendBroadcast(
            Intent("agent.log").putExtra("log", log)
        )
    }
    
    /**
     * 发送完成广播
     */
    private fun sendCompleteBroadcast(success: Boolean, result: String) {
        LocalBroadcastManager.getInstance(this).sendBroadcast(
            Intent("agent.complete")
                .putExtra("success", success)
                .putExtra("result", result)
        )
        
        // 📊 检查是否建议显示报告
        checkAndPromptReport()
    }
    
    /**
     * 📊 检查是否需要提示用户查看/提交报告
     */
    private fun checkAndPromptReport() {
        val debugInterface = com.elon.app.agent.infrastructure.debug.DebugInterface.getInstance()
        val report = debugInterface.lastExecutionReport ?: return
        
        // 如果建议上报（有性能问题），显示提示
        if (report.shouldReport) {
            scope.launch(Dispatchers.Main) {
                try {
                    Toast.makeText(
                        this@AgentService,
                        "💡 检测到执行问题，点击悬浮球查看详细报告",
                        Toast.LENGTH_LONG
                    ).show()
                    
                    // 发送广播通知界面显示报告按钮
                    LocalBroadcastManager.getInstance(this@AgentService).sendBroadcast(
                        Intent("agent.report_available")
                            .putExtra("should_report", true)
                            .putExtra("performance_score", report.summary.performanceScore)
                    )
                } catch (e: Exception) {
                    Log.w(TAG, "显示报告提示失败", e)
                }
            }
        }
    }
    
    /**
     * 📋 显示执行报告对话框（供外部调用）
     */
    fun showExecutionReport() {
        try {
            val reportDialog = com.elon.app.agent.infrastructure.floating.ReportSubmitDialog(this)
            reportDialog.show()
        } catch (e: Exception) {
            Log.e(TAG, "显示报告对话框失败", e)
            Toast.makeText(this, "无法显示报告: ${e.message}", Toast.LENGTH_SHORT).show()
        }
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // 🆕 转发给智能屏幕读取器（增量模式使用）
        smartScreenReader?.onAccessibilityEvent(event)
    }

    override fun onInterrupt() {
        Log.w(TAG, "服务被中断")
    }

    override fun onUnbind(intent: Intent?): Boolean {
        Log.i(TAG, "服务解绑")
        instance = null  // 清除实例
        socketServer?.stop()
        scope.cancel()
        
        // 🆕 注销系统无障碍按钮回调
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            accessibilityButtonCallback?.let {
                accessibilityButtonController?.unregisterAccessibilityButtonCallback(it)
            }
        }
        
        return super.onUnbind(intent)
    }
}
