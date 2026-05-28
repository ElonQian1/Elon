// infrastructure/vision/ScreenAnalyzer.kt
// module: infrastructure/vision | layer: infrastructure | role: screen-analyzer
// summary: 智能屏幕分析器，提取结构化信息用于 AI 决策

package com.elon.app.agent.infrastructure.vision

import android.graphics.Rect
import com.elon.app.agent.domain.screen.UINode

/**
 * 屏幕分析结果
 */
data class ScreenAnalysis(
    val appContext: String,               // 应用上下文 (xiaohongshu, weixin 等)
    val pageType: String,                 // 页面类型 (feed_list, detail_page 等)
    val interactiveElements: List<InteractiveElement>,  // 可交互元素
    val dataElements: List<DataElement>,               // 数据元素（点赞数等）
    val hotContent: List<HotContent>,                  // 高热度内容
    val navigationElements: List<NavigationElement>,   // 导航元素
    val summary: String                                // 分析摘要
)

data class InteractiveElement(
    val text: String,
    val bounds: Rect,
    val className: String,
    val resourceId: String?
)

data class DataElement(
    val type: String,       // likes, comments, favorites, shares
    val rawText: String,
    val value: Double,
    val bounds: Rect
)

data class HotContent(
    val text: String,
    val value: Double,
    val bounds: Rect,
    val isClickable: Boolean
)

data class NavigationElement(
    val text: String,
    val bounds: Rect,
    val position: String    // top, bottom
)

/**
 * 智能屏幕分析器
 * 
 * 功能：
 * - 识别应用上下文
 * - 分类页面类型
 * - 提取可交互元素
 * - 识别数据元素（点赞、评论数等）
 * - 发现高热度内容
 */
class ScreenAnalyzer {
    
    companion object {
        private const val SCREEN_HEIGHT = 1920  // 默认屏幕高度
        private const val HOT_THRESHOLD = 10000.0  // 热门阈值（1万）
        
        // 应用包名映射
        private val APP_CONTEXT_MAP = mapOf(
            "com.xingin.xhs" to "xiaohongshu",
            "com.tencent.mm" to "weixin",
            "com.ss.android.ugc.aweme" to "douyin",
            "com.sina.weibo" to "weibo"
        )
    }
    
    /**
     * 分析屏幕 UI 树
     */
    fun analyze(root: UINode, packageName: String? = null, focus: String = "all"): ScreenAnalysis {
        val interactiveElements = mutableListOf<InteractiveElement>()
        val dataElements = mutableListOf<DataElement>()
        val hotContent = mutableListOf<HotContent>()
        val navigationElements = mutableListOf<NavigationElement>()
        
        // 递归遍历所有节点
        traverseAndAnalyze(root, interactiveElements, dataElements, hotContent, navigationElements)
        
        // 检测应用上下文
        val appContext = detectAppContext(packageName, root)
        
        // 推断页面类型
        val pageType = inferPageType(interactiveElements, dataElements, navigationElements)
        
        // 根据 focus 过滤结果
        val filteredInteractive = if (focus == "all" || focus == "interactive") interactiveElements else emptyList()
        val filteredData = if (focus == "all" || focus == "data") dataElements else emptyList()
        val filteredHot = if (focus == "all" || focus == "data") hotContent else emptyList()
        val filteredNav = if (focus == "all" || focus == "navigation") navigationElements else emptyList()
        
        // 生成摘要
        val summary = "发现 ${interactiveElements.size} 个可交互元素，" +
                "${dataElements.size} 个数据元素，" +
                "其中 ${hotContent.size} 个高热度内容（点赞过万）"
        
        return ScreenAnalysis(
            appContext = appContext,
            pageType = pageType,
            interactiveElements = filteredInteractive,
            dataElements = filteredData,
            hotContent = filteredHot,
            navigationElements = filteredNav,
            summary = summary
        )
    }
    
    /**
     * 递归遍历并分析节点
     */
    private fun traverseAndAnalyze(
        node: UINode,
        interactive: MutableList<InteractiveElement>,
        data: MutableList<DataElement>,
        hot: MutableList<HotContent>,
        navigation: MutableList<NavigationElement>
    ) {
        val displayText = node.text ?: node.contentDescription ?: ""
        val isClickable = node.isClickable
        val className = node.className
        val resourceId = node.resourceId
        
        // 可交互元素
        if (isClickable && displayText.isNotBlank()) {
            interactive.add(InteractiveElement(
                text = displayText,
                bounds = node.bounds,
                className = className,
                resourceId = resourceId
            ))
        }
        
        // 数据元素（点赞数、评论数等）
        val engagement = extractEngagementNumber(displayText)
        if (engagement != null) {
            val elementType = classifyEngagementType(displayText, resourceId)
            data.add(DataElement(
                type = elementType,
                rawText = displayText,
                value = engagement,
                bounds = node.bounds
            ))
            
            // 高热度内容
            if (elementType == "likes" && engagement >= HOT_THRESHOLD) {
                hot.add(HotContent(
                    text = displayText,
                    value = engagement,
                    bounds = node.bounds,
                    isClickable = isClickable
                ))
            }
        }
        
        // 导航元素（顶部或底部）
        val y = node.bounds.top
        val classLower = className.lowercase()
        if ((y > SCREEN_HEIGHT - 200 || y < 200) && isClickable && displayText.isNotBlank()) {
            if (classLower.contains("tab") || classLower.contains("button") ||
                resourceId?.contains("tab") == true || resourceId?.contains("nav") == true) {
                navigation.add(NavigationElement(
                    text = displayText,
                    bounds = node.bounds,
                    position = if (y < 200) "top" else "bottom"
                ))
            }
        }
        
        // 递归子节点
        node.children.forEach { child ->
            traverseAndAnalyze(child, interactive, data, hot, navigation)
        }
    }
    
    /**
     * 提取互动数据（点赞数、评论数等）
     */
    private fun extractEngagementNumber(text: String): Double? {
        if (text.isBlank()) return null
        
        // 匹配：1.8万、2475、10w+、1000+ 等
        val patterns = listOf(
            Regex("""(\d+\.?\d*)\s*[万w]"""),  // 万/w
            Regex("""(\d+\.?\d*)\s*[千k]"""),  // 千/k
            Regex("""^(\d+)\+?$"""),           // 纯数字
            Regex("""(\d+)\s*(?:赞|评|藏|转)""")  // 点赞/评论/收藏/转发
        )
        
        for (pattern in patterns) {
            val match = pattern.find(text)
            if (match != null) {
                val num = match.groupValues[1].toDoubleOrNull() ?: continue
                return when {
                    text.contains("万") || text.lowercase().contains("w") -> num * 10000
                    text.contains("千") || text.lowercase().contains("k") -> num * 1000
                    else -> num
                }
            }
        }
        return null
    }
    
    /**
     * 分类互动数据类型
     */
    private fun classifyEngagementType(text: String, resourceId: String?): String {
        val combined = "${text.lowercase()} ${resourceId?.lowercase() ?: ""}"
        return when {
            combined.contains("like") || combined.contains("赞") || combined.contains("❤") -> "likes"
            combined.contains("comment") || combined.contains("评论") -> "comments"
            combined.contains("collect") || combined.contains("收藏") || combined.contains("⭐") -> "favorites"
            combined.contains("share") || combined.contains("转发") || combined.contains("分享") -> "shares"
            else -> "unknown"
        }
    }
    
    /**
     * 检测应用上下文
     */
    private fun detectAppContext(packageName: String?, root: UINode): String {
        // 优先使用包名
        packageName?.let { pkg ->
            APP_CONTEXT_MAP.entries.forEach { (key, value) ->
                if (pkg.contains(key)) return value
            }
        }
        
        // 回退：从 UI 内容推断
        val allText = root.getAllTexts().joinToString(" ")
        return when {
            allText.contains("小红书") -> "xiaohongshu"
            allText.contains("微信") -> "weixin"
            allText.contains("抖音") -> "douyin"
            allText.contains("微博") -> "weibo"
            else -> "other"
        }
    }
    
    /**
     * 推断页面类型
     */
    private fun inferPageType(
        interactive: List<InteractiveElement>,
        data: List<DataElement>,
        navigation: List<NavigationElement>
    ): String {
        val hasBottomNav = navigation.any { it.position == "bottom" }
        val hasManyData = data.size > 5
        val hasEngagement = data.any { it.type == "likes" || it.type == "comments" }
        
        return when {
            hasBottomNav && hasManyData -> "feed_list"      // 信息流/首页
            hasEngagement && !hasBottomNav -> "detail_page" // 详情页
            navigation.size > 3 -> "navigation_page"
            else -> "unknown"
        }
    }
    
    /**
     * 生成 AI 可读的摘要（用于提示词）
     */
    fun generateAISummary(analysis: ScreenAnalysis): String {
        return buildString {
            appendLine("## 屏幕分析结果")
            appendLine()
            appendLine("**应用**: ${analysis.appContext}")
            appendLine("**页面类型**: ${analysis.pageType}")
            appendLine("**摘要**: ${analysis.summary}")
            appendLine()
            
            if (analysis.hotContent.isNotEmpty()) {
                appendLine("### 🔥 高热度内容 (点赞过万)")
                analysis.hotContent.forEach { hot ->
                    val center = Pair(hot.bounds.centerX(), hot.bounds.centerY())
                    appendLine("- \"${hot.text}\" (${hot.value.toLong()}赞) @ 坐标(${center.first}, ${center.second})")
                }
                appendLine()
            }
            
            if (analysis.interactiveElements.isNotEmpty()) {
                appendLine("### 🔘 可交互元素 (前10个)")
                analysis.interactiveElements.take(10).forEach { elem ->
                    val center = Pair(elem.bounds.centerX(), elem.bounds.centerY())
                    appendLine("- \"${elem.text}\" @ 坐标(${center.first}, ${center.second})")
                }
            }
        }
    }
}
