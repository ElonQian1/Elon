// infrastructure/accessibility/IncrementalScreenCache.kt
// module: infrastructure/accessibility | layer: infrastructure | role: incremental-cache
// summary: 增量屏幕缓存 - 监听无障碍事件，维护变化节点队列

package com.elon.app.agent.infrastructure.accessibility

import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.graphics.Rect
import com.elon.app.agent.domain.screen.*
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.atomic.AtomicReference

/**
 * 增量屏幕缓存
 * 
 * 通过监听 AccessibilityEvent 实现增量获取：
 * - 缓存最近的变化事件
 * - 维护最新的 UI 树快照
 * - 支持差异比较
 */
class IncrementalScreenCache {
    
    companion object {
        private const val MAX_EVENTS = 50  // 最多缓存的事件数
        private const val CACHE_VALIDITY_MS = 2000L  // 缓存有效期
    }
    
    // 变化事件队列（线程安全）
    private val eventQueue = ConcurrentLinkedQueue<ScreenChangeEvent>()
    
    // 最近一次完整快照
    private val lastSnapshot = AtomicReference<CachedSnapshot?>(null)
    
    // 当前包名
    private var currentPackage: String? = null
    
    /**
     * 处理无障碍事件（由 AgentService.onAccessibilityEvent 调用）
     */
    fun onAccessibilityEvent(event: AccessibilityEvent?) {
        event ?: return
        
        val changeType = mapEventType(event.eventType)
        val timestamp = System.currentTimeMillis()
        currentPackage = event.packageName?.toString()
        
        // 提取变化的节点
        val changedNode = try {
            event.source?.let { parseMinimalNode(it) }
        } catch (e: Exception) {
            null
        }
        
        // 构建事件描述
        val description = buildEventDescription(event, changeType)
        
        val changeEvent = ScreenChangeEvent(
            eventType = changeType,
            timestamp = timestamp,
            packageName = currentPackage,
            changedNode = changedNode,
            description = description
        )
        
        // 添加到队列
        eventQueue.add(changeEvent)
        
        // 限制队列大小
        while (eventQueue.size > MAX_EVENTS) {
            eventQueue.poll()
        }
        
        // 窗口切换时清空缓存
        if (changeType == ChangeType.WINDOW_CHANGED) {
            lastSnapshot.set(null)
        }
    }
    
    /**
     * 获取增量变化（自上次调用以来）
     */
    fun getIncrementalChanges(): List<ScreenChangeEvent> {
        val changes = mutableListOf<ScreenChangeEvent>()
        while (eventQueue.isNotEmpty()) {
            eventQueue.poll()?.let { changes.add(it) }
        }
        return changes
    }
    
    /**
     * 查看但不消费变化（用于检查是否有变化）
     */
    fun peekChanges(): List<ScreenChangeEvent> {
        return eventQueue.toList()
    }
    
    /**
     * 检查是否有待处理的变化
     */
    fun hasChanges(): Boolean = eventQueue.isNotEmpty()
    
    /**
     * 获取变化摘要（给 AI 看的简短描述）
     */
    fun getChangesSummary(): String {
        val changes = peekChanges()
        if (changes.isEmpty()) {
            return "无变化"
        }
        
        val grouped = changes.groupBy { it.eventType }
        val parts = mutableListOf<String>()
        
        grouped[ChangeType.WINDOW_CHANGED]?.let {
            parts.add("窗口切换 ${it.size} 次")
        }
        grouped[ChangeType.CONTENT_CHANGED]?.let {
            parts.add("内容变化 ${it.size} 处")
        }
        grouped[ChangeType.VIEW_CLICKED]?.let {
            parts.add("点击 ${it.size} 次")
        }
        grouped[ChangeType.VIEW_SCROLLED]?.let {
            parts.add("滚动 ${it.size} 次")
        }
        grouped[ChangeType.TEXT_CHANGED]?.let {
            parts.add("文本变化 ${it.size} 处")
        }
        
        return parts.joinToString("，")
    }
    
    /**
     * 更新快照
     */
    fun updateSnapshot(tree: UINode) {
        lastSnapshot.set(CachedSnapshot(
            tree = tree,
            timestamp = System.currentTimeMillis(),
            packageName = currentPackage
        ))
    }
    
    /**
     * 获取缓存的快照（如果有效）
     */
    fun getCachedSnapshot(): CachedSnapshot? {
        val snapshot = lastSnapshot.get() ?: return null
        val age = System.currentTimeMillis() - snapshot.timestamp
        return if (age < CACHE_VALIDITY_MS && !hasChanges()) {
            snapshot
        } else {
            null
        }
    }
    
    /**
     * 清空缓存
     */
    fun clear() {
        eventQueue.clear()
        lastSnapshot.set(null)
    }
    
    // === 私有方法 ===
    
    private fun mapEventType(eventType: Int): ChangeType {
        return when (eventType) {
            AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED -> ChangeType.WINDOW_CHANGED
            AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED -> ChangeType.CONTENT_CHANGED
            AccessibilityEvent.TYPE_VIEW_CLICKED -> ChangeType.VIEW_CLICKED
            AccessibilityEvent.TYPE_VIEW_SCROLLED -> ChangeType.VIEW_SCROLLED
            AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED -> ChangeType.TEXT_CHANGED
            AccessibilityEvent.TYPE_VIEW_FOCUSED -> ChangeType.FOCUS_CHANGED
            else -> ChangeType.UNKNOWN
        }
    }
    
    private fun buildEventDescription(event: AccessibilityEvent, type: ChangeType): String {
        val text = event.text?.joinToString(" ") ?: ""
        val className = event.className?.toString()?.substringAfterLast(".") ?: ""
        
        return when (type) {
            ChangeType.WINDOW_CHANGED -> "窗口切换到: ${event.packageName}"
            ChangeType.CONTENT_CHANGED -> "内容变化: $className ${text.take(50)}"
            ChangeType.VIEW_CLICKED -> "点击: $className ${text.take(30)}"
            ChangeType.VIEW_SCROLLED -> "滚动: $className"
            ChangeType.TEXT_CHANGED -> "文本变化: ${text.take(50)}"
            ChangeType.FOCUS_CHANGED -> "焦点变化: $className"
            ChangeType.UNKNOWN -> "未知事件"
        }
    }
    
    private fun parseMinimalNode(node: AccessibilityNodeInfo): UINode {
        val rect = Rect()
        node.getBoundsInScreen(rect)
        
        return UINode(
            className = node.className?.toString() ?: "",
            text = node.text?.toString(),
            contentDescription = node.contentDescription?.toString(),
            resourceId = node.viewIdResourceName,
            bounds = rect,
            isClickable = node.isClickable,
            isEnabled = node.isEnabled,
            isPassword = node.isPassword,
            children = emptyList()  // 增量模式不递归子节点
        )
    }
}

/**
 * 缓存的快照
 */
data class CachedSnapshot(
    val tree: UINode,
    val timestamp: Long,
    val packageName: String?
)
