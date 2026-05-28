// infrastructure/accessibility/ScreenDiffDetector.kt
// module: infrastructure/accessibility | layer: infrastructure | role: diff-detector
// summary: 屏幕差异检测器 - 比较两次 UI 树快照，生成精简的差异报告

package com.elon.app.agent.infrastructure.accessibility

import com.elon.app.agent.domain.screen.*

/**
 * 屏幕差异检测器
 * 
 * 比较两次 UI 树的差异，生成精简的差异报告给 AI：
 * - 新增的节点
 * - 消失的节点
 * - 修改的节点（文本、位置等变化）
 */
class ScreenDiffDetector {
    
    /**
     * 比较两棵 UI 树的差异
     */
    fun diff(oldTree: UINode?, newTree: UINode?): ScreenDiff {
        if (oldTree == null && newTree == null) {
            return ScreenDiff(
                hasChanges = false,
                addedNodes = emptyList(),
                removedNodes = emptyList(),
                modifiedNodes = emptyList(),
                summary = "无数据"
            )
        }
        
        if (oldTree == null) {
            val allNodes = flattenTree(newTree!!)
            return ScreenDiff(
                hasChanges = true,
                addedNodes = allNodes,
                removedNodes = emptyList(),
                modifiedNodes = emptyList(),
                summary = "新页面加载，共 ${allNodes.size} 个节点"
            )
        }
        
        if (newTree == null) {
            val allNodes = flattenTree(oldTree)
            return ScreenDiff(
                hasChanges = true,
                addedNodes = emptyList(),
                removedNodes = allNodes,
                modifiedNodes = emptyList(),
                summary = "页面已关闭"
            )
        }
        
        // 比较两棵树
        val oldFlat = flattenTreeWithKey(oldTree)
        val newFlat = flattenTreeWithKey(newTree)
        
        val addedNodes = mutableListOf<UINode>()
        val removedNodes = mutableListOf<UINode>()
        val modifiedNodes = mutableListOf<NodeChange>()
        
        // 找出新增的节点
        for ((key, node) in newFlat) {
            if (!oldFlat.containsKey(key)) {
                addedNodes.add(node)
            }
        }
        
        // 找出消失的节点
        for ((key, node) in oldFlat) {
            if (!newFlat.containsKey(key)) {
                removedNodes.add(node)
            }
        }
        
        // 找出修改的节点
        for ((key, newNode) in newFlat) {
            val oldNode = oldFlat[key] ?: continue
            val changes = compareNodes(oldNode, newNode)
            if (changes.isNotEmpty()) {
                for (change in changes) {
                    modifiedNodes.add(NodeChange(
                        node = newNode,
                        changeType = change.first,
                        oldValue = change.second,
                        newValue = change.third
                    ))
                }
            }
        }
        
        val hasChanges = addedNodes.isNotEmpty() || 
                         removedNodes.isNotEmpty() || 
                         modifiedNodes.isNotEmpty()
        
        return ScreenDiff(
            hasChanges = hasChanges,
            addedNodes = addedNodes.take(20),  // 限制数量
            removedNodes = removedNodes.take(20),
            modifiedNodes = modifiedNodes.take(30),
            summary = buildSummary(addedNodes, removedNodes, modifiedNodes)
        )
    }
    
    /**
     * 生成给 AI 看的简短差异描述
     */
    fun diffToAISummary(diff: ScreenDiff): String {
        if (!diff.hasChanges) {
            return "页面无变化"
        }
        
        val sb = StringBuilder()
        sb.appendLine("【页面变化摘要】")
        sb.appendLine(diff.summary)
        
        if (diff.addedNodes.isNotEmpty()) {
            sb.appendLine("\n新增元素:")
            diff.addedNodes.take(5).forEach { node ->
                val text = node.text ?: node.contentDescription ?: node.className
                sb.appendLine("  + $text")
            }
            if (diff.addedNodes.size > 5) {
                sb.appendLine("  ... 还有 ${diff.addedNodes.size - 5} 个")
            }
        }
        
        if (diff.removedNodes.isNotEmpty()) {
            sb.appendLine("\n消失元素:")
            diff.removedNodes.take(5).forEach { node ->
                val text = node.text ?: node.contentDescription ?: node.className
                sb.appendLine("  - $text")
            }
            if (diff.removedNodes.size > 5) {
                sb.appendLine("  ... 还有 ${diff.removedNodes.size - 5} 个")
            }
        }
        
        if (diff.modifiedNodes.isNotEmpty()) {
            sb.appendLine("\n修改元素:")
            diff.modifiedNodes.take(5).forEach { change ->
                val text = change.node.text ?: change.node.className
                sb.appendLine("  ~ $text: ${change.changeType} 从 '${change.oldValue}' 变为 '${change.newValue}'")
            }
        }
        
        return sb.toString()
    }
    
    // === 私有方法 ===
    
    private fun flattenTree(node: UINode): List<UINode> {
        val result = mutableListOf<UINode>()
        flattenTreeRecursive(node, result)
        return result
    }
    
    private fun flattenTreeRecursive(node: UINode, result: MutableList<UINode>) {
        // 只添加有意义的节点
        if (node.text != null || node.contentDescription != null || node.isClickable) {
            result.add(node)
        }
        for (child in node.children) {
            flattenTreeRecursive(child, result)
        }
    }
    
    private fun flattenTreeWithKey(node: UINode): Map<String, UINode> {
        val result = mutableMapOf<String, UINode>()
        flattenTreeWithKeyRecursive(node, result, "")
        return result
    }
    
    private fun flattenTreeWithKeyRecursive(
        node: UINode, 
        result: MutableMap<String, UINode>,
        path: String
    ) {
        val key = generateNodeKey(node, path)
        
        // 只添加有意义的节点
        if (node.text != null || node.contentDescription != null || 
            node.isClickable || node.resourceId != null) {
            result[key] = node
        }
        
        for ((index, child) in node.children.withIndex()) {
            flattenTreeWithKeyRecursive(child, result, "$path/$index")
        }
    }
    
    /**
     * 生成节点唯一标识
     * 优先使用 resourceId，否则用位置+类名
     */
    private fun generateNodeKey(node: UINode, path: String): String {
        return if (node.resourceId != null) {
            node.resourceId!!
        } else {
            // 用位置和类名作为 key
            "${node.bounds.left},${node.bounds.top}-${node.className}-$path"
        }
    }
    
    /**
     * 比较两个节点的差异
     */
    private fun compareNodes(
        oldNode: UINode, 
        newNode: UINode
    ): List<Triple<String, String?, String?>> {
        val changes = mutableListOf<Triple<String, String?, String?>>()
        
        // 文本变化
        if (oldNode.text != newNode.text) {
            changes.add(Triple("text", oldNode.text, newNode.text))
        }
        
        // 描述变化
        if (oldNode.contentDescription != newNode.contentDescription) {
            changes.add(Triple("description", oldNode.contentDescription, newNode.contentDescription))
        }
        
        // 可点击状态变化
        if (oldNode.isClickable != newNode.isClickable) {
            changes.add(Triple("clickable", oldNode.isClickable.toString(), newNode.isClickable.toString()))
        }
        
        // 启用状态变化
        if (oldNode.isEnabled != newNode.isEnabled) {
            changes.add(Triple("enabled", oldNode.isEnabled.toString(), newNode.isEnabled.toString()))
        }
        
        // 位置明显变化（超过 50px）
        val boundsDiff = Math.abs(oldNode.bounds.left - newNode.bounds.left) + 
                        Math.abs(oldNode.bounds.top - newNode.bounds.top)
        if (boundsDiff > 50) {
            changes.add(Triple(
                "position", 
                "${oldNode.bounds.left},${oldNode.bounds.top}",
                "${newNode.bounds.left},${newNode.bounds.top}"
            ))
        }
        
        return changes
    }
    
    private fun buildSummary(
        added: List<UINode>,
        removed: List<UINode>,
        modified: List<NodeChange>
    ): String {
        val parts = mutableListOf<String>()
        
        if (added.isNotEmpty()) {
            parts.add("新增 ${added.size} 个元素")
        }
        if (removed.isNotEmpty()) {
            parts.add("消失 ${removed.size} 个元素")
        }
        if (modified.isNotEmpty()) {
            parts.add("修改 ${modified.size} 处")
        }
        
        return if (parts.isEmpty()) "无明显变化" else parts.joinToString("，")
    }
}
