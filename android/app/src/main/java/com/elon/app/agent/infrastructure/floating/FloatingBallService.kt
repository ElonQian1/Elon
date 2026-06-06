// infrastructure/floating/FloatingBallService.kt
// module: infrastructure/floating | layer: infrastructure | role: floating-ball-service
// summary: 悬浮球服务 - 提供全局悬浮球、迷你执行面板，单击语音输入，双击文字输入

package com.elon.app.agent.infrastructure.floating

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.graphics.PixelFormat
import android.os.Build
import android.os.IBinder
import android.provider.Settings
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.Toast
import androidx.core.app.NotificationCompat
import com.elon.app.agent.domain.execution.ExecutionInfo
import com.elon.app.agent.domain.execution.ExecutionLogEntry
import com.elon.app.agent.domain.execution.ExecutionState
import com.elon.app.agent.domain.execution.ExecutionStateManager

/**
 * 🎈 悬浮球服务
 * 
 * 功能：
 * - 单击：语音输入任务
 * - 双击：文字输入任务
 * - 长按：拖拽移动
 * - 执行中：显示旋转动画
 * - 🆕 迷你面板：实时显示执行进度和日志
 * - 🆕 停止按钮：支持中途停止执行
 */
class FloatingBallService : Service() {
    
    companion object {
        private const val TAG = "FloatingBall"
        private const val NOTIFICATION_ID = 2001
        private const val CHANNEL_ID = "floating_ball_channel"
        
        // 服务状态
        var isRunning = false
            private set
        
        // 任务执行回调（由 AgentService 设置）
        var onTaskSubmit: ((String) -> Unit)? = null
        
        /**
         * 启动悬浮球服务
         */
        fun start(context: Context) {
            Log.i(TAG, "尝试启动悬浮球服务...")
            
            if (!canDrawOverlays(context)) {
                Log.w(TAG, "没有悬浮窗权限")
                Toast.makeText(context, "请先授予悬浮窗权限", Toast.LENGTH_LONG).show()
                requestOverlayPermission(context)
                return
            }
            
            try {
                val intent = Intent(context, FloatingBallService::class.java)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    Log.i(TAG, "使用 startForegroundService")
                    context.startForegroundService(intent)
                } else {
                    Log.i(TAG, "使用 startService")
                    context.startService(intent)
                }
                Log.i(TAG, "悬浮球服务启动命令已发送")
            } catch (e: Exception) {
                Log.e(TAG, "启动悬浮球服务失败", e)
                Toast.makeText(context, "启动失败: ${e.message}", Toast.LENGTH_LONG).show()
            }
        }
        
        /**
         * 停止悬浮球服务
         */
        fun stop(context: Context) {
            context.stopService(Intent(context, FloatingBallService::class.java))
        }
        
        /**
         * 检查是否有悬浮窗权限
         */
        fun canDrawOverlays(context: Context): Boolean {
            return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                Settings.canDrawOverlays(context)
            } else {
                true
            }
        }
        
        /**
         * 请求悬浮窗权限
         */
        fun requestOverlayPermission(context: Context) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val intent = Intent(
                    Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                    android.net.Uri.parse("package:${context.packageName}")
                )
                intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                context.startActivity(intent)
            }
        }
    }
    
    private var windowManager: WindowManager? = null
    private var floatingBallView: FloatingBallView? = null
    
    // 🆕 迷你悬浮面板
    private var miniPanelView: MiniFloatingPanelView? = null
    
    // 🆕 状态观察者（监听执行状态变化以更新悬浮球动画）
    private val stateObserver = object : ExecutionStateManager.StateObserver {
        override fun onStateChanged(info: ExecutionInfo) {
            updateFloatingBallFromState(info)
            updateMiniPanelVisibility(info)
        }
        override fun onLogAdded(entry: ExecutionLogEntry) {
            // 迷你面板会自己处理日志
        }
    }
    
    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "=== 悬浮球服务 onCreate ===")
        isRunning = true
        
        try {
            createNotificationChannel()
            Log.i(TAG, "通知渠道已创建")
            
            startForeground(NOTIFICATION_ID, createNotification())
            Log.i(TAG, "前台服务已启动")
            
            showFloatingBall()
            Log.i(TAG, "悬浮球已显示")
            
            // 🆕 显示迷你面板
            showMiniPanel()
            Log.i(TAG, "迷你面板已创建")
            
            // 🆕 注册状态观察者
            ExecutionStateManager.addObserver(stateObserver)
        } catch (e: Exception) {
            Log.e(TAG, "onCreate 失败", e)
        }
    }
    
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i(TAG, "=== 悬浮球服务 onStartCommand ===")
        return START_STICKY
    }
    
    override fun onDestroy() {
        super.onDestroy()
        Log.i(TAG, "=== 悬浮球服务 onDestroy ===")
        isRunning = false
        
        // 🆕 移除状态观察者
        ExecutionStateManager.removeObserver(stateObserver)
        
        hideFloatingBall()
        hideMiniPanel()
    }
    
    override fun onBind(intent: Intent?): IBinder? = null
    
    // ==================== 悬浮球显示 ====================
    
    private fun showFloatingBall() {
        if (floatingBallView != null) return
        
        windowManager = getSystemService(Context.WINDOW_SERVICE) as WindowManager
        
        // 创建悬浮球视图
        floatingBallView = FloatingBallView(this).apply {
            // 单击 -> 语音输入
            onSingleClick = {
                Log.i(TAG, "单击 -> 启动语音输入")
                showVoiceInputDialog()
            }
            
            // 双击 -> 文字输入
            onDoubleClick = {
                Log.i(TAG, "双击 -> 显示文字输入")
                showTextInputDialog()
            }
        }
        
        // 悬浮窗参数
        val layoutParams = WindowManager.LayoutParams().apply {
            width = WindowManager.LayoutParams.WRAP_CONTENT
            height = WindowManager.LayoutParams.WRAP_CONTENT
            
            type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
            } else {
                @Suppress("DEPRECATION")
                WindowManager.LayoutParams.TYPE_PHONE
            }
            
            flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS
            
            format = PixelFormat.TRANSLUCENT
            gravity = Gravity.TOP or Gravity.START
            
            // 初始位置：右侧中间
            val displayMetrics = resources.displayMetrics
            x = displayMetrics.widthPixels - 150
            y = displayMetrics.heightPixels / 3
        }
        
        try {
            windowManager?.addView(floatingBallView, layoutParams)
            Log.i(TAG, "悬浮球已显示")
        } catch (e: Exception) {
            Log.e(TAG, "显示悬浮球失败", e)
        }
    }
    
    private fun hideFloatingBall() {
        floatingBallView?.let {
            try {
                windowManager?.removeView(it)
                Log.i(TAG, "悬浮球已隐藏")
            } catch (e: Exception) {
                Log.e(TAG, "隐藏悬浮球失败", e)
            }
        }
        floatingBallView = null
    }
    
    // ==================== 迷你面板 ====================
    
    private fun showMiniPanel() {
        if (miniPanelView != null) return
        
        miniPanelView = MiniFloatingPanelView(this).apply {
            // 停止按钮回调
            onStopClick = {
                Log.i(TAG, "用户点击停止按钮")
                ExecutionStateManager.requestStop()
            }
            
            // 关闭面板回调
            onCloseClick = {
                Log.i(TAG, "用户关闭面板")
                this.hide()
            }
            
            // 初始隐藏（等待执行开始）
            visibility = View.GONE
        }
        
        val layoutParams = WindowManager.LayoutParams().apply {
            width = WindowManager.LayoutParams.WRAP_CONTENT
            height = WindowManager.LayoutParams.WRAP_CONTENT
            
            type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
            } else {
                @Suppress("DEPRECATION")
                WindowManager.LayoutParams.TYPE_PHONE
            }
            
            flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS
            
            format = PixelFormat.TRANSLUCENT
            gravity = Gravity.TOP or Gravity.START
            
            // 初始位置：左上角
            val displayMetrics = resources.displayMetrics
            x = 20
            y = displayMetrics.heightPixels / 4
        }
        
        try {
            windowManager?.addView(miniPanelView, layoutParams)
            Log.i(TAG, "迷你面板已添加")
        } catch (e: Exception) {
            Log.e(TAG, "显示迷你面板失败", e)
        }
    }
    
    private fun hideMiniPanel() {
        miniPanelView?.let {
            try {
                it.onDestroy()
                windowManager?.removeView(it)
                Log.i(TAG, "迷你面板已隐藏")
            } catch (e: Exception) {
                Log.e(TAG, "隐藏迷你面板失败", e)
            }
        }
        miniPanelView = null
    }
    
    /**
     * 🆕 根据执行状态更新悬浮球动画
     */
    private fun updateFloatingBallFromState(info: ExecutionInfo) {
        val ballState = when (info.state) {
            ExecutionState.IDLE -> FloatingBallState.IDLE
            ExecutionState.EXECUTING -> FloatingBallState.EXECUTING
            ExecutionState.STOPPING -> FloatingBallState.EXECUTING
            ExecutionState.STOPPED -> FloatingBallState.IDLE
            ExecutionState.SUCCESS -> FloatingBallState.IDLE
            ExecutionState.FAILED -> FloatingBallState.ERROR
        }
        floatingBallView?.setState(ballState)
    }
    
    /**
     * 🆕 根据执行状态更新迷你面板可见性
     */
    private fun updateMiniPanelVisibility(info: ExecutionInfo) {
        miniPanelView?.let { panel ->
            when (info.state) {
                ExecutionState.EXECUTING, ExecutionState.STOPPING -> {
                    if (panel.visibility != View.VISIBLE) {
                        panel.show()
                    } else {
                        // 已经可见，无需操作
                    }
                }
                ExecutionState.SUCCESS, ExecutionState.FAILED, ExecutionState.STOPPED -> {
                    // 执行完成后保持显示 3 秒再隐藏
                    panel.postDelayed({
                        if (!ExecutionStateManager.isExecuting) {
                            panel.hide()
                        }
                    }, 3000)
                }
                ExecutionState.IDLE -> {
                    // 空闲时隐藏
                    if (panel.visibility == View.VISIBLE) {
                        panel.hide()
                    } else {
                        // 已经隐藏，无需操作
                    }
                }
            }
        }
    }
    
    // ==================== 语音输入 ====================
    
    private fun showVoiceInputDialog() {
        // 使用新的智能对话系统 V2
        val intent = Intent(this, ConversationalVoiceActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
        }
        startActivity(intent)
    }
    
    // ==================== 文字输入 ====================
    
    private fun showTextInputDialog() {
        val intent = Intent(this, FloatingInputActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
        }
        startActivity(intent)
    }
    
    // ==================== 状态更新 ====================
    
    /**
     * 更新悬浮球状态
     */
    fun updateState(state: FloatingBallState) {
        floatingBallView?.setState(state)
    }
    
    // ==================== 通知 ====================
    
    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "悬浮球服务",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "保持悬浮球运行"
                setShowBadge(false)
            }
            
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }
    
    private fun createNotification(): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("🎈 悬浮球运行中")
            .setContentText("单击语音 | 双击文字 | 长按拖动")
            .setSmallIcon(android.R.drawable.ic_menu_compass)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .build()
    }
}

/**
 * 悬浮球状态
 */
enum class FloatingBallState {
    IDLE,       // 空闲 - 绿色
    LISTENING,  // 监听中 - 蓝色脉冲
    EXECUTING,  // 执行中 - 蓝色旋转
    ERROR       // 错误 - 红色
}
