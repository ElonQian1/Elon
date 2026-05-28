// infrastructure/accessibility/SmartScreenReader.kt
// module: infrastructure/accessibility | layer: infrastructure | role: smart-reader
// summary: 智能屏幕读取器 - 统一支持三种模式：全量/增量/差异

package com.elon.app.agent.infrastructure.accessibility

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent
import com.elon.app.agent.domain.screen.*
import com.elon.app.agent.application.ScreenReader
import android.util.Log

/**
 * 智能屏幕读取器
 * 
 * 统一管理三种屏幕获取模式：
 * - FULL_DUMP: 完整 UI 树
 * - INCREMENTAL: 增量变化
 * - DIFF: 差异对比
 * 
 * 可以根据场景动态切换模式，优化 AI Token 消耗
 */
class SmartScreenReader(
    private val service: AccessibilityService
) : ScreenReader {
    
    companion object {
        private const val TAG = "SmartScreenReader"
    }
    
    // 当前模式
    @Volatile
    var currentMode: ScreenCaptureMode = ScreenCaptureMode.FULL_DUMP
        private set
    
    // 组件
    private val parser = UITreeParser(service)
    private val cache = IncrementalScreenCache()
    private val diffDetector = ScreenDiffDetector()
    
    // 基准快照（用于 DIFF 模式）
    private var baselineSnapshot: UINode? = null
    
    // 统计信息
    private var fullDumpCount = 0
    private var incrementalCount = 0
    private var diffCount = 0
    
    /**
     * 切换模式
     */
    fun setMode(mode: ScreenCaptureMode) {
        Log.d(TAG, "切换屏幕获取模式: ${currentMode.displayName} -> ${mode.displayName}")
        currentMode = mode
        
        // 切换到 DIFF 模式时，自动拍摄基准快照
        if (mode == ScreenCaptureMode.DIFF && baselineSnapshot == null) {
            takeBaselineSnapshot()
        }
    }
    
    /**
     * 处理无障碍事件（由 AgentService 调用）
     */
    fun onAccessibilityEvent(event: AccessibilityEvent?) {
        cache.onAccessibilityEvent(event)
    }
    
    /**
     * 根据当前模式获取屏幕信息
     */
    override suspend fun readCurrentScreen(): UINode {
        return when (currentMode) {
            ScreenCaptureMode.FULL_DUMP -> readFullDump()
            ScreenCaptureMode.INCREMENTAL -> readIncremental()
            ScreenCaptureMode.DIFF -> readDiff()
        }
    }
    
    /**
     * 强制全量获取（忽略当前模式）- 同步版本
     */
    fun forceFullDump(): UINode {
        return readFullDump()
    }
    
    /**
     * 强制全量获取（忽略当前模式）- 挂起版本
     */
    suspend fun forceFullDumpAsync(): UINode {
        return readFullDump()
    }
    
    /**
     * 获取增量变化
     */
    fun getIncrementalChanges(): List<ScreenChangeEvent> {
        incrementalCount++
        return cache.getIncrementalChanges()
    }
    
    /**
     * 检查是否有变化
     */
    fun hasChanges(): Boolean = cache.hasChanges()
    
    /**
     * 获取变化摘要（给 AI 的精简描述）
     */
    fun getChangesSummary(): String = cache.getChangesSummary()
    
    /**
     * 拍摄基准快照（用于 DIFF 模式）
     */
    fun takeBaselineSnapshot() {
        Log.d(TAG, "拍摄基准快照")
        baselineSnapshot = parser.readCurrentScreenSync()
        cache.updateSnapshot(baselineSnapshot!!)
    }
    
    /**
     * 获取与基准的差异
     */
    fun getDiffFromBaseline(): ScreenDiff {
        diffCount++
        val currentTree = parser.readCurrentScreenSync()
        return diffDetector.diff(baselineSnapshot, currentTree)
    }
    
    /**
     * 获取差异的 AI 摘要
     */
    fun getDiffSummaryForAI(): String {
        val diff = getDiffFromBaseline()
        return diffDetector.diffToAISummary(diff)
    }
    
    /**
     * 获取统计信息
     */
    fun getStats(): ScreenReaderStats {
        return ScreenReaderStats(
            currentMode = currentMode,
            fullDumpCount = fullDumpCount,
            incrementalCount = incrementalCount,
            diffCount = diffCount,
            hasCachedSnapshot = baselineSnapshot != null,
            pendingChanges = cache.peekChanges().size
        )
    }
    
    /**
     * 重置统计和缓存
     */
    fun reset() {
        cache.clear()
        baselineSnapshot = null
        fullDumpCount = 0
        incrementalCount = 0
        diffCount = 0
    }
    
    /**
     * 智能选择模式（根据场景自动选择最优模式）
     */
    fun autoSelectMode(scenario: String): ScreenCaptureMode {
        val recommendedMode = when {
            scenario.contains("首次") || scenario.contains("分析") -> ScreenCaptureMode.FULL_DUMP
            scenario.contains("等待") || scenario.contains("检测") -> ScreenCaptureMode.INCREMENTAL
            scenario.contains("验证") || scenario.contains("确认") -> ScreenCaptureMode.DIFF
            else -> currentMode
        }
        
        Log.d(TAG, "场景 '$scenario' 推荐模式: ${recommendedMode.displayName}")
        setMode(recommendedMode)
        return recommendedMode
    }
    
    // === 私有方法 ===
    
    private fun readFullDump(): UINode {
        fullDumpCount++
        Log.d(TAG, "执行全量 Dump (#$fullDumpCount)")
        
        val tree = parser.readCurrentScreenSync()
        cache.updateSnapshot(tree)
        return tree
    }
    
    private fun readIncremental(): UINode {
        incrementalCount++
        
        // 检查缓存是否有效
        val cached = cache.getCachedSnapshot()
        if (cached != null && !cache.hasChanges()) {
            Log.d(TAG, "使用缓存快照 (增量模式 #$incrementalCount)")
            return cached.tree
        }
        
        // 缓存失效，降级为全量
        Log.d(TAG, "缓存失效，降级为全量 Dump")
        return readFullDump()
    }
    
    private fun readDiff(): UINode {
        diffCount++
        
        // 如果没有基准，先拍摄
        if (baselineSnapshot == null) {
            Log.d(TAG, "DIFF 模式：拍摄基准快照")
            takeBaselineSnapshot()
            return baselineSnapshot!!
        }
        
        // 返回当前快照（差异通过 getDiffFromBaseline 获取）
        Log.d(TAG, "DIFF 模式：获取当前快照 (#$diffCount)")
        return parser.readCurrentScreenSync()
    }
}

/**
 * 屏幕读取器统计信息
 */
data class ScreenReaderStats(
    val currentMode: ScreenCaptureMode,
    val fullDumpCount: Int,
    val incrementalCount: Int,
    val diffCount: Int,
    val hasCachedSnapshot: Boolean,
    val pendingChanges: Int
) {
    fun toJson(): String {
        return """
        {
            "current_mode": "${currentMode.name}",
            "mode_display": "${currentMode.displayName}",
            "token_cost": "${currentMode.tokenCost}",
            "stats": {
                "full_dump_count": $fullDumpCount,
                "incremental_count": $incrementalCount,
                "diff_count": $diffCount,
                "has_cached_snapshot": $hasCachedSnapshot,
                "pending_changes": $pendingChanges
            }
        }
        """.trimIndent()
    }
}
