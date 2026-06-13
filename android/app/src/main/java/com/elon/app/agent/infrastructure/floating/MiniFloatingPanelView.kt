// infrastructure/floating/MiniFloatingPanelView.kt
// module: infrastructure/floating | layer: infrastructure | role: mini-floating-panel
// summary: 迷你悬浮面板 - 可拖拽、展开/收起、显示执行状态和日志、停止按钮

package com.elon.app.agent.infrastructure.floating

import android.animation.AnimatorSet
import android.animation.ObjectAnimator
import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.animation.AccelerateDecelerateInterpolator
import android.view.animation.OvershootInterpolator
import android.widget.*
import com.elon.app.agent.domain.execution.ExecutionInfo
import com.elon.app.agent.domain.execution.ExecutionLogEntry
import com.elon.app.agent.domain.execution.ExecutionState
import com.elon.app.agent.domain.execution.ExecutionStateManager

/**
 * 🎛️ 迷你悬浮面板视图
 * 
 * 功能：
 * - 可拖拽移动
 * - 展开/收起两种模式
 * - 实时显示执行状态和进度
 * - 显示最近 5 条执行日志
 * - 停止按钮支持中断执行
 * - 玻璃拟态设计风格
 * 
 * 布局结构（展开状态）：
 * ┌─────────────────────────────────────┐
 * │  🤖 执行中: 任务名称          [×]   │  ← 标题栏 (可拖拽)
 * ├─────────────────────────────────────┤
 * │  步骤 3/7: 点击搜索框               │  ← 进度显示
 * │  ▓▓▓▓▓▓▓░░░░░░░░░░░░  43%          │  ← 进度条
 * ├─────────────────────────────────────┤
 * │  [10:23:45] 找到搜索框元素          │  ← 日志区域
 * │  [10:23:46] 执行点击操作            │
 * │  [10:23:47] 等待页面响应...         │
 * ├─────────────────────────────────────┤
 * │      [ ⏹ 停止执行 ]                 │  ← 停止按钮
 * └─────────────────────────────────────┘
 * 
 * 布局结构（收起状态）：
 * ┌───────────────┐
 * │ 🔄 3/7 [⏹][↗] │  ← 简化显示
 * └───────────────┘
 */
@SuppressLint("ViewConstructor")
class MiniFloatingPanelView(context: Context) : FrameLayout(context), 
    ExecutionStateManager.StateObserver {
    
    companion object {
        private const val TAG = "MiniFloatingPanel"
        
        // 尺寸 (dp)
        private const val EXPANDED_WIDTH = 280
        private const val EXPANDED_HEIGHT = 220
        private const val COLLAPSED_WIDTH = 140
        private const val COLLAPSED_HEIGHT = 44
        
        // 颜色
        private val COLOR_BG_DARK = Color.parseColor("#E8181B20")      // 深色背景 (90% 不透明)
        private val COLOR_BG_HEADER = Color.parseColor("#30283140")    // 标题栏背景
        private val COLOR_TEXT_PRIMARY = Color.parseColor("#D6D6D6")   // 主文字
        private val COLOR_TEXT_SECONDARY = Color.parseColor("#A8A8A8") // 次要文字
        private val COLOR_ACCENT = Color.parseColor("#58BE6A")         // 强调色
        private val COLOR_STOP = Color.parseColor("#D97A7A")           // 停止按钮
        private val COLOR_SUCCESS = Color.parseColor("#58BE6A")        // 成功
        private val COLOR_FAILED = Color.parseColor("#D97A7A")         // 失败
        
        // 触摸检测
        private const val CLICK_THRESHOLD = 10
    }
    
    // ==================== 回调 ====================
    
    /** 停止按钮点击回调 */
    var onStopClick: (() -> Unit)? = null
    
    /** 关闭面板回调 */
    var onCloseClick: (() -> Unit)? = null
    
    // ==================== 视图组件 ====================
    
    private val rootContainer: LinearLayout       // 根容器
    private val headerLayout: LinearLayout        // 标题栏
    private val titleText: TextView               // 标题文字
    private val toggleButton: TextView            // 展开/收起按钮
    private val closeButton: TextView             // 关闭按钮
    
    // 展开时的组件
    private val expandedContent: LinearLayout     // 展开内容容器
    private val progressText: TextView            // 进度文字
    private val progressBar: ProgressBar          // 进度条
    private val logContainer: LinearLayout        // 日志容器
    private val stopButton: Button                // 停止按钮
    
    // 收起时的组件（在 headerLayout 中）
    private val collapsedProgressText: TextView   // 收起时的进度文字
    private val collapsedStopButton: TextView     // 收起时的停止按钮
    
    // ==================== 状态 ====================
    
    private var isExpanded = true
    private val density = context.resources.displayMetrics.density
    private val mainHandler = Handler(Looper.getMainLooper())
    
    // 触摸拖拽
    private var initialX = 0
    private var initialY = 0
    private var initialTouchX = 0f
    private var initialTouchY = 0f
    private var isDragging = false
    
    // ==================== 初始化 ====================
    
    init {
        val expandedWidthPx = (EXPANDED_WIDTH * density).toInt()
        val expandedHeightPx = (EXPANDED_HEIGHT * density).toInt()
        
        // 根容器
        rootContainer = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            background = createPanelBackground()
            elevation = 12 * density
            clipToOutline = true
        }
        
        // === 标题栏 ===
        headerLayout = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setBackgroundColor(COLOR_BG_HEADER)
            setPadding(dp(12), dp(8), dp(8), dp(8))
        }
        
        // 状态图标 + 标题
        titleText = TextView(context).apply {
            text = "🤖 等待执行"
            textSize = 13f
            setTextColor(COLOR_TEXT_PRIMARY)
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
        }
        headerLayout.addView(titleText, LinearLayout.LayoutParams(0, LayoutParams.WRAP_CONTENT, 1f))
        
        // 收起时的进度文字（初始隐藏）
        collapsedProgressText = TextView(context).apply {
            text = "0/0"
            textSize = 12f
            setTextColor(COLOR_ACCENT)
            visibility = View.GONE
            setPadding(dp(4), 0, dp(4), 0)
        }
        headerLayout.addView(collapsedProgressText)
        
        // 收起时的停止按钮（初始隐藏）
        collapsedStopButton = TextView(context).apply {
            text = "⏹"
            textSize = 16f
            setTextColor(COLOR_STOP)
            visibility = View.GONE
            setPadding(dp(8), 0, dp(8), 0)
            setOnClickListener { onStopClick?.invoke() }
        }
        headerLayout.addView(collapsedStopButton)
        
        // 展开/收起按钮
        toggleButton = TextView(context).apply {
            text = "▼"
            textSize = 14f
            setTextColor(COLOR_TEXT_SECONDARY)
            setPadding(dp(8), 0, dp(4), 0)
            setOnClickListener { toggleExpand() }
        }
        headerLayout.addView(toggleButton)
        
        // 关闭按钮
        closeButton = TextView(context).apply {
            text = "✕"
            textSize = 14f
            setTextColor(COLOR_TEXT_SECONDARY)
            setPadding(dp(4), 0, dp(4), 0)
            setOnClickListener { onCloseClick?.invoke() }
        }
        headerLayout.addView(closeButton)
        
        rootContainer.addView(headerLayout, LinearLayout.LayoutParams(
            LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT
        ))
        
        // === 展开内容区 ===
        expandedContent = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(8), dp(12), dp(12))
        }
        
        // 进度文字
        progressText = TextView(context).apply {
            text = "等待执行..."
            textSize = 12f
            setTextColor(COLOR_TEXT_PRIMARY)
        }
        expandedContent.addView(progressText, LinearLayout.LayoutParams(
            LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT
        ))
        
        // 进度条
        progressBar = ProgressBar(context, null, android.R.attr.progressBarStyleHorizontal).apply {
            max = 100
            progress = 0
            progressDrawable.setTint(COLOR_ACCENT)
        }
        expandedContent.addView(progressBar, LinearLayout.LayoutParams(
            LayoutParams.MATCH_PARENT, dp(6)
        ).apply { topMargin = dp(6) })
        
        // 分隔线
        expandedContent.addView(View(context).apply {
            setBackgroundColor(Color.parseColor("#30283140"))
        }, LinearLayout.LayoutParams(LayoutParams.MATCH_PARENT, dp(1)).apply {
            topMargin = dp(8)
            bottomMargin = dp(4)
        })
        
        // 日志容器
        logContainer = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
        }
        val scrollView = ScrollView(context).apply {
            addView(logContainer, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT))
            isVerticalScrollBarEnabled = false
        }
        expandedContent.addView(scrollView, LinearLayout.LayoutParams(
            LayoutParams.MATCH_PARENT, dp(70)
        ))
        
        // 停止按钮
        stopButton = Button(context).apply {
            text = "⏹ 停止执行"
            textSize = 13f
            setTextColor(Color.parseColor("#D6D6D6"))
            background = createStopButtonBackground()
            setPadding(dp(16), dp(8), dp(16), dp(8))
            setOnClickListener { onStopClick?.invoke() }
        }
        expandedContent.addView(stopButton, LinearLayout.LayoutParams(
            LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(8) })
        
        rootContainer.addView(expandedContent, LinearLayout.LayoutParams(
            LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT
        ))
        
        addView(rootContainer, LayoutParams(expandedWidthPx, LayoutParams.WRAP_CONTENT))
        
        // 设置触摸监听（拖拽）
        setupTouchListener()
        
        // 注册状态观察者
        ExecutionStateManager.addObserver(this)
        
        // 初始状态更新
        updateUI(ExecutionStateManager.currentInfo)
    }
    
    // ==================== 状态观察者回调 ====================
    
    override fun onStateChanged(info: ExecutionInfo) {
        mainHandler.post { updateUI(info) }
    }
    
    override fun onLogAdded(entry: ExecutionLogEntry) {
        mainHandler.post { addLogEntry(entry) }
    }
    
    // ==================== UI 更新 ====================
    
    private fun updateUI(info: ExecutionInfo) {
        // 更新标题
        val stateIcon = when (info.state) {
            ExecutionState.IDLE -> "🤖"
            ExecutionState.EXECUTING -> "🔄"
            ExecutionState.STOPPING -> "⏳"
            ExecutionState.STOPPED -> "⏹"
            ExecutionState.SUCCESS -> "✅"
            ExecutionState.FAILED -> "❌"
        }
        
        val titleStr = if (info.taskGoal.isNotEmpty()) {
            "$stateIcon ${info.taskGoal.take(15)}${if (info.taskGoal.length > 15) "..." else ""}"
        } else {
            "$stateIcon 等待执行"
        }
        titleText.text = titleStr
        
        // 更新进度
        progressText.text = info.progressText
        progressBar.progress = info.progressPercent
        
        // 收起状态的进度
        collapsedProgressText.text = "${info.currentStep}/${info.totalSteps}"
        
        // 更新按钮状态
        val canStop = info.canStop
        stopButton.isEnabled = canStop
        stopButton.alpha = if (canStop) 1f else 0.5f
        collapsedStopButton.isEnabled = canStop
        collapsedStopButton.alpha = if (canStop) 1f else 0.3f
        
        // 更新进度条颜色
        val progressColor = when (info.state) {
            ExecutionState.SUCCESS -> COLOR_SUCCESS
            ExecutionState.FAILED, ExecutionState.STOPPED -> COLOR_FAILED
            ExecutionState.STOPPING -> Color.parseColor("#8DDC9B")
            else -> COLOR_ACCENT
        }
        progressBar.progressDrawable.setTint(progressColor)
    }
    
    private fun addLogEntry(entry: ExecutionLogEntry) {
        // 添加日志行
        val logLine = TextView(context).apply {
            text = entry.formattedText
            textSize = 10f
            setTextColor(when (entry.level) {
                ExecutionLogEntry.LogLevel.ERROR -> COLOR_FAILED
                ExecutionLogEntry.LogLevel.WARNING -> Color.parseColor("#8DDC9B")
                else -> COLOR_TEXT_SECONDARY
            })
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
        }
        logContainer.addView(logLine)
        
        // 保留最近 5 条
        while (logContainer.childCount > 5) {
            logContainer.removeViewAt(0)
        }
        
        // 滚动到底部
        (logContainer.parent as? ScrollView)?.post {
            (logContainer.parent as? ScrollView)?.fullScroll(View.FOCUS_DOWN)
        }
    }
    
    // ==================== 展开/收起 ====================
    
    private fun toggleExpand() {
        isExpanded = !isExpanded
        
        val targetWidth = if (isExpanded) EXPANDED_WIDTH else COLLAPSED_WIDTH
        
        // 动画
        if (isExpanded) {
            expandedContent.visibility = View.VISIBLE
            collapsedProgressText.visibility = View.GONE
            collapsedStopButton.visibility = View.GONE
            toggleButton.text = "▼"
        } else {
            expandedContent.visibility = View.GONE
            collapsedProgressText.visibility = View.VISIBLE
            collapsedStopButton.visibility = View.VISIBLE
            toggleButton.text = "▲"
        }
        
        // 更新宽度
        val params = rootContainer.layoutParams
        params.width = dp(targetWidth)
        rootContainer.layoutParams = params
        
        // 更新 WindowManager
        updateWindowLayoutParams()
        
        Log.d(TAG, "面板 ${if (isExpanded) "展开" else "收起"}")
    }
    
    private fun updateWindowLayoutParams() {
        val wm = context.getSystemService(Context.WINDOW_SERVICE) as? WindowManager
        try {
            wm?.updateViewLayout(this, layoutParams)
        } catch (e: Exception) {
            Log.e(TAG, "更新窗口布局失败", e)
        }
    }
    
    // ==================== 拖拽处理 ====================
    
    @SuppressLint("ClickableViewAccessibility")
    private fun setupTouchListener() {
        headerLayout.setOnTouchListener { _, event ->
            when (event.action) {
                MotionEvent.ACTION_DOWN -> {
                    val lp = layoutParams as WindowManager.LayoutParams
                    initialX = lp.x
                    initialY = lp.y
                    initialTouchX = event.rawX
                    initialTouchY = event.rawY
                    isDragging = false
                    true
                }
                MotionEvent.ACTION_MOVE -> {
                    val dx = event.rawX - initialTouchX
                    val dy = event.rawY - initialTouchY
                    
                    if (kotlin.math.abs(dx) > CLICK_THRESHOLD || kotlin.math.abs(dy) > CLICK_THRESHOLD) {
                        isDragging = true
                    }
                    
                    if (isDragging) {
                        val lp = layoutParams as WindowManager.LayoutParams
                        lp.x = initialX + dx.toInt()
                        lp.y = initialY + dy.toInt()
                        updateWindowLayoutParams()
                    }
                    true
                }
                MotionEvent.ACTION_UP -> {
                    isDragging = false
                    true
                }
                else -> false
            }
        }
    }
    
    // ==================== 辅助方法 ====================
    
    private fun dp(value: Int): Int = (value * density).toInt()
    
    private fun createPanelBackground(): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = 16 * density
            setColor(COLOR_BG_DARK)
            setStroke((1 * density).toInt(), Color.parseColor("#30283140"))
        }
    }
    
    private fun createStopButtonBackground(): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = 8 * density
            setColor(COLOR_STOP)
        }
    }
    
    // ==================== 生命周期 ====================
    
    fun onDestroy() {
        ExecutionStateManager.removeObserver(this)
    }
    
    /**
     * 显示面板（执行开始时）
     */
    fun show() {
        visibility = View.VISIBLE
        // 清空日志
        logContainer.removeAllViews()
        // 默认展开
        if (!isExpanded) {
            toggleExpand()
        }
        
        // 入场动画
        alpha = 0f
        scaleX = 0.8f
        scaleY = 0.8f
        animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .setDuration(200)
            .setInterpolator(OvershootInterpolator(1.2f))
            .start()
    }
    
    /**
     * 隐藏面板
     */
    fun hide() {
        animate()
            .alpha(0f)
            .scaleX(0.8f)
            .scaleY(0.8f)
            .setDuration(150)
            .setInterpolator(AccelerateDecelerateInterpolator())
            .withEndAction { visibility = View.GONE }
            .start()
    }
}
