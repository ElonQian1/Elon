// domain/execution/ExecutionStateManager.kt
// module: domain/execution | layer: domain | role: execution-state-manager
// summary: 执行状态管理器 - 单例，管理全局执行状态、日志、取消令牌

package com.elon.app.agent.domain.execution

import android.os.Handler
import android.os.Looper
import android.util.Log
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicBoolean

/**
 * 🎯 执行状态管理器 - 单例
 * 
 * 核心职责：
 * 1. 管理全局执行状态（IDLE/EXECUTING/STOPPING/STOPPED/SUCCESS/FAILED）
 * 2. 维护执行日志（环形缓冲，最多 100 条）
 * 3. 提供取消令牌机制（支持优雅中断）
 * 4. 观察者模式通知状态变化（悬浮窗、主界面等）
 */
object ExecutionStateManager {
    
    private const val TAG = "ExecutionStateManager"
    private const val MAX_LOG_ENTRIES = 100
    
    // ==================== 状态 ====================
    
    /** 当前执行信息 */
    @Volatile
    private var _currentInfo: ExecutionInfo = ExecutionInfo()
    val currentInfo: ExecutionInfo get() = _currentInfo
    
    /** 取消令牌 - 用于中断执行 */
    private val _isCancelled = AtomicBoolean(false)
    val isCancelled: Boolean get() = _isCancelled.get()
    
    // ==================== 日志 ====================
    
    /** 执行日志（环形缓冲） */
    private val _logEntries = CopyOnWriteArrayList<ExecutionLogEntry>()
    val logEntries: List<ExecutionLogEntry> get() = _logEntries.toList()
    
    /** 获取最近 N 条日志 */
    fun getRecentLogs(count: Int = 5): List<ExecutionLogEntry> {
        val entries = _logEntries.toList()
        return if (entries.size <= count) entries
        else entries.takeLast(count)
    }
    
    // ==================== 观察者 ====================
    
    /** 状态观察者接口 */
    interface StateObserver {
        /** 执行信息变化时调用 */
        fun onStateChanged(info: ExecutionInfo)
        
        /** 新日志添加时调用 */
        fun onLogAdded(entry: ExecutionLogEntry)
    }
    
    private val observers = CopyOnWriteArrayList<StateObserver>()
    private val mainHandler = Handler(Looper.getMainLooper())
    
    /** 添加观察者 */
    fun addObserver(observer: StateObserver) {
        if (!observers.contains(observer)) {
            observers.add(observer)
            Log.d(TAG, "观察者已添加，当前数量: ${observers.size}")
            // 立即通知当前状态
            mainHandler.post {
                observer.onStateChanged(_currentInfo)
            }
        }
    }
    
    /** 移除观察者 */
    fun removeObserver(observer: StateObserver) {
        observers.remove(observer)
        Log.d(TAG, "观察者已移除，当前数量: ${observers.size}")
    }
    
    /** 通知所有观察者状态变化 */
    private fun notifyStateChanged() {
        val info = _currentInfo
        mainHandler.post {
            observers.forEach { it.onStateChanged(info) }
        }
    }
    
    /** 通知所有观察者新日志 */
    private fun notifyLogAdded(entry: ExecutionLogEntry) {
        mainHandler.post {
            observers.forEach { it.onLogAdded(entry) }
        }
    }
    
    // ==================== 执行控制 ====================
    
    /**
     * 开始执行任务
     * 
     * @param goal 任务目标
     * @param totalSteps 预估总步骤数（可为 0 表示未知）
     */
    @Synchronized
    fun startExecution(goal: String, totalSteps: Int = 0) {
        Log.i(TAG, "📍 开始执行: $goal (预计 $totalSteps 步)")
        
        // 重置取消令牌
        _isCancelled.set(false)
        
        // 清空日志
        _logEntries.clear()
        
        // 更新状态
        _currentInfo = ExecutionInfo(
            state = ExecutionState.EXECUTING,
            taskGoal = goal,
            currentStep = 0,
            totalSteps = totalSteps,
            stepName = "初始化...",
            startTime = System.currentTimeMillis()
        )
        
        addLogInternal("🚀 开始执行: $goal")
        notifyStateChanged()
    }
    
    /**
     * 更新总步骤数（脚本生成后可能才知道具体步骤数）
     */
    @Synchronized
    fun updateTotalSteps(totalSteps: Int) {
        if (_currentInfo.state == ExecutionState.EXECUTING) {
            _currentInfo = _currentInfo.copy(totalSteps = totalSteps)
            notifyStateChanged()
        }
    }
    
    /**
     * 更新当前步骤
     * 
     * @param step 当前步骤序号（从 1 开始）
     * @param name 步骤名称
     */
    @Synchronized
    fun updateStep(step: Int, name: String) {
        if (_currentInfo.state == ExecutionState.EXECUTING) {
            _currentInfo = _currentInfo.copy(
                currentStep = step,
                stepName = name
            )
            addLogInternal("📌 步骤 $step: $name")
            notifyStateChanged()
        }
    }
    
    /**
     * 添加执行日志
     */
    fun addLog(message: String, level: ExecutionLogEntry.LogLevel = ExecutionLogEntry.LogLevel.INFO) {
        addLogInternal(message, level)
    }
    
    private fun addLogInternal(message: String, level: ExecutionLogEntry.LogLevel = ExecutionLogEntry.LogLevel.INFO) {
        val entry = ExecutionLogEntry(
            message = message,
            level = level
        )
        
        // 环形缓冲：超过最大数量时移除最早的
        while (_logEntries.size >= MAX_LOG_ENTRIES) {
            _logEntries.removeAt(0)
        }
        _logEntries.add(entry)
        
        Log.d(TAG, "📝 $message")
        notifyLogAdded(entry)
    }
    
    /**
     * 请求停止执行
     * 
     * 设置取消令牌，ScriptEngine 会在下一个检查点响应
     */
    @Synchronized
    fun requestStop() {
        if (_currentInfo.state == ExecutionState.EXECUTING) {
            Log.i(TAG, "⏹️ 用户请求停止执行")
            _isCancelled.set(true)
            _currentInfo = _currentInfo.copy(state = ExecutionState.STOPPING)
            addLogInternal("⏹️ 正在停止...", ExecutionLogEntry.LogLevel.WARNING)
            notifyStateChanged()
        }
    }
    
    /**
     * 检查是否应该取消执行
     * 
     * ScriptEngine 在每步执行前后应调用此方法
     * @return true 表示应该停止执行
     */
    fun shouldCancel(): Boolean = _isCancelled.get()
    
    /**
     * 执行成功完成
     */
    @Synchronized
    fun executionSuccess(resultMessage: String = "") {
        Log.i(TAG, "✅ 执行成功: $resultMessage")
        _currentInfo = _currentInfo.copy(
            state = ExecutionState.SUCCESS,
            resultMessage = resultMessage
        )
        addLogInternal("✅ 执行成功!", ExecutionLogEntry.LogLevel.INFO)
        notifyStateChanged()
    }
    
    /**
     * 执行失败
     */
    @Synchronized
    fun executionFailed(errorMessage: String) {
        Log.e(TAG, "❌ 执行失败: $errorMessage")
        _currentInfo = _currentInfo.copy(
            state = ExecutionState.FAILED,
            resultMessage = errorMessage
        )
        addLogInternal("❌ 失败: $errorMessage", ExecutionLogEntry.LogLevel.ERROR)
        notifyStateChanged()
    }
    
    /**
     * 执行被用户停止
     */
    @Synchronized
    fun executionStopped() {
        Log.i(TAG, "⏹️ 执行已停止")
        _currentInfo = _currentInfo.copy(state = ExecutionState.STOPPED)
        addLogInternal("⏹️ 已停止", ExecutionLogEntry.LogLevel.WARNING)
        notifyStateChanged()
    }
    
    /**
     * 重置为空闲状态
     */
    @Synchronized
    fun reset() {
        Log.d(TAG, "🔄 重置状态")
        _isCancelled.set(false)
        _currentInfo = ExecutionInfo()
        notifyStateChanged()
    }
    
    // ==================== 便捷方法 ====================
    
    /** 是否正在执行 */
    val isExecuting: Boolean get() = _currentInfo.state == ExecutionState.EXECUTING
    
    /** 是否正在停止 */
    val isStopping: Boolean get() = _currentInfo.state == ExecutionState.STOPPING
    
    /** 是否空闲 */
    val isIdle: Boolean get() = _currentInfo.state == ExecutionState.IDLE
}
