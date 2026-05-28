// infrastructure/vision/MultimodalScreenAnalyzer.kt
// module: infrastructure/vision | layer: infrastructure | role: multimodal-analyzer
// summary: 多模态屏幕分析器，融合 UI 树 + 截图分析

package com.elon.app.agent.infrastructure.vision

import android.util.Log
import com.elon.app.agent.domain.screen.UINode
import com.elon.app.agent.infrastructure.accessibility.UITreeParser
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope

/**
 * 多模态屏幕分析器
 * 
 * 融合策略：
 * 1. UI 树 - 精确的元素结构和坐标
 * 2. 截图 - 视觉上下文和 OCR 补充
 */
class MultimodalScreenAnalyzer(
    private val screenshotCapture: ScreenshotCapture,
    private val visionClient: VisionClient?,
    private val uiTreeParser: UITreeParser
) {
    companion object {
        private const val TAG = "MultimodalAnalyzer"
        
        private const val DEFAULT_VISION_PROMPT = """
分析这个 Android 屏幕截图，请描述：
1. 当前在什么 App 的什么页面
2. 页面上有哪些可交互的元素（按钮、输入框、列表项等）
3. 每个元素的大致位置（上/中/下，左/中/右）
4. 如果有弹窗或对话框，请特别标注

请用 JSON 格式返回：
{
  "app": "应用名称",
  "page": "页面类型",
  "elements": [
    {"text": "元素文本", "type": "button/input/text/image", "position": "位置描述"}
  ],
  "hasDialog": false,
  "summary": "一句话描述当前状态"
}
"""
        
        private const val HYBRID_VISION_PROMPT = """
UI 树信息不完整，请通过截图补充分析：
1. 识别图片中的文字（OCR）
2. 识别可能的按钮和可点击区域
3. 描述页面整体布局

简洁返回 JSON：
{
  "ocrTexts": ["识别到的文字"],
  "clickableAreas": [{"description": "描述", "position": "位置"}],
  "layout": "布局描述"
}
"""
    }
    
    /**
     * 分析模式
     */
    enum class AnalysisMode {
        UI_TREE_ONLY,      // 仅 UI 树（快速）
        SCREENSHOT_ONLY,   // 仅截图（兼容）
        HYBRID,            // 混合模式（推荐）
        VISION_PRIORITY    // Vision 优先（复杂场景）
    }
    
    /**
     * 获取屏幕分析结果
     */
    suspend fun analyzeScreen(
        mode: AnalysisMode = AnalysisMode.HYBRID,
        customPrompt: String? = null
    ): ScreenAnalysisResult = coroutineScope {
        when (mode) {
            AnalysisMode.UI_TREE_ONLY -> analyzeWithUITree()
            AnalysisMode.SCREENSHOT_ONLY -> analyzeWithScreenshot(customPrompt)
            AnalysisMode.HYBRID -> analyzeHybrid(customPrompt)
            AnalysisMode.VISION_PRIORITY -> analyzeVisionPriority(customPrompt)
        }
    }
    
    /**
     * 仅 UI 树分析
     */
    private suspend fun analyzeWithUITree(): ScreenAnalysisResult {
        return try {
            val uiTree = uiTreeParser.readCurrentScreen()
            val elements = flattenUITree(uiTree)
            
            ScreenAnalysisResult(
                success = true,
                uiTree = uiTree,
                elements = elements,
                screenshotBase64 = null,
                visionDescription = null,
                analysisMode = AnalysisMode.UI_TREE_ONLY
            )
        } catch (e: Exception) {
            Log.e(TAG, "UI 树分析失败", e)
            ScreenAnalysisResult(
                success = false,
                error = e.message,
                analysisMode = AnalysisMode.UI_TREE_ONLY
            )
        }
    }
    
    /**
     * 仅截图分析（需要 Vision API）
     */
    private suspend fun analyzeWithScreenshot(prompt: String?): ScreenAnalysisResult {
        if (visionClient == null) {
            return ScreenAnalysisResult(
                success = false,
                error = "Vision 客户端未配置",
                analysisMode = AnalysisMode.SCREENSHOT_ONLY
            )
        }
        
        return try {
            val screenshotResult = screenshotCapture.captureScreenBase64()
            
            when (screenshotResult) {
                is ScreenshotResult.Success -> {
                    val analysisPrompt = prompt ?: DEFAULT_VISION_PROMPT
                    val visionResult = visionClient.analyzeImage(
                        screenshotResult.base64,
                        analysisPrompt
                    )
                    
                    when (visionResult) {
                        is VisionResult.Success -> ScreenAnalysisResult(
                            success = true,
                            screenshotBase64 = screenshotResult.base64,
                            visionDescription = visionResult.content,
                            screenWidth = screenshotResult.width,
                            screenHeight = screenshotResult.height,
                            analysisMode = AnalysisMode.SCREENSHOT_ONLY
                        )
                        is VisionResult.Failure -> ScreenAnalysisResult(
                            success = false,
                            error = visionResult.error,
                            screenshotBase64 = screenshotResult.base64,
                            analysisMode = AnalysisMode.SCREENSHOT_ONLY
                        )
                    }
                }
                is ScreenshotResult.Failure -> ScreenAnalysisResult(
                    success = false,
                    error = screenshotResult.error,
                    analysisMode = AnalysisMode.SCREENSHOT_ONLY
                )
            }
        } catch (e: Exception) {
            Log.e(TAG, "截图分析失败", e)
            ScreenAnalysisResult(
                success = false,
                error = e.message,
                analysisMode = AnalysisMode.SCREENSHOT_ONLY
            )
        }
    }
    
    /**
     * 混合分析（推荐）
     */
    private suspend fun analyzeHybrid(prompt: String?): ScreenAnalysisResult = coroutineScope {
        val uiTreeDeferred = async { runCatching { uiTreeParser.readCurrentScreen() } }
        val screenshotDeferred = async { screenshotCapture.captureScreenBase64() }
        
        val uiTreeResult = uiTreeDeferred.await()
        val screenshotResult = screenshotDeferred.await()
        
        val uiTree = uiTreeResult.getOrNull()
        val elements = uiTree?.let { flattenUITree(it) } ?: emptyList()
        
        // 如果 UI 树获取成功且有足够元素，不需要 Vision
        if (uiTree != null && elements.size >= 5) {
            return@coroutineScope ScreenAnalysisResult(
                success = true,
                uiTree = uiTree,
                elements = elements,
                screenshotBase64 = (screenshotResult as? ScreenshotResult.Success)?.base64,
                screenWidth = (screenshotResult as? ScreenshotResult.Success)?.width,
                screenHeight = (screenshotResult as? ScreenshotResult.Success)?.height,
                analysisMode = AnalysisMode.HYBRID
            )
        }
        
        // UI 树不足，需要 Vision 补充
        if (visionClient != null && screenshotResult is ScreenshotResult.Success) {
            val visionResult = visionClient.analyzeImage(
                screenshotResult.base64,
                prompt ?: HYBRID_VISION_PROMPT
            )
            
            return@coroutineScope ScreenAnalysisResult(
                success = true,
                uiTree = uiTree,
                elements = elements,
                screenshotBase64 = screenshotResult.base64,
                visionDescription = (visionResult as? VisionResult.Success)?.content,
                screenWidth = screenshotResult.width,
                screenHeight = screenshotResult.height,
                analysisMode = AnalysisMode.HYBRID
            )
        }
        
        ScreenAnalysisResult(
            success = uiTree != null || screenshotResult is ScreenshotResult.Success,
            uiTree = uiTree,
            elements = elements,
            screenshotBase64 = (screenshotResult as? ScreenshotResult.Success)?.base64,
            error = if (uiTree == null && screenshotResult is ScreenshotResult.Failure) 
                "UI 树和截图都失败" else null,
            analysisMode = AnalysisMode.HYBRID
        )
    }
    
    /**
     * Vision 优先分析
     */
    private suspend fun analyzeVisionPriority(prompt: String?): ScreenAnalysisResult = coroutineScope {
        if (visionClient == null) {
            return@coroutineScope analyzeWithUITree()
        }
        
        val uiTreeDeferred = async { runCatching { uiTreeParser.readCurrentScreen() } }
        val screenshotDeferred = async { screenshotCapture.captureScreenBase64() }
        
        val uiTreeResult = uiTreeDeferred.await()
        val screenshotResult = screenshotDeferred.await()
        
        if (screenshotResult !is ScreenshotResult.Success) {
            return@coroutineScope analyzeWithUITree()
        }
        
        val uiTree = uiTreeResult.getOrNull()
        val elements = uiTree?.let { flattenUITree(it) } ?: emptyList()
        
        val enhancedPrompt = buildEnhancedPrompt(prompt, elements)
        val visionResult = visionClient.analyzeImage(screenshotResult.base64, enhancedPrompt)
        
        ScreenAnalysisResult(
            success = true,
            uiTree = uiTree,
            elements = elements,
            screenshotBase64 = screenshotResult.base64,
            visionDescription = (visionResult as? VisionResult.Success)?.content,
            screenWidth = screenshotResult.width,
            screenHeight = screenshotResult.height,
            analysisMode = AnalysisMode.VISION_PRIORITY
        )
    }
    
    /**
     * 将 UI 树扁平化为元素列表
     */
    private fun flattenUITree(node: UINode?, list: MutableList<UIElement> = mutableListOf()): List<UIElement> {
        if (node == null) return list
        
        if (node.isClickable || !node.text.isNullOrBlank() || !node.contentDescription.isNullOrBlank()) {
            list.add(UIElement(
                id = list.size,
                text = node.text ?: node.contentDescription ?: "",
                className = node.className,
                bounds = "[${node.bounds.left},${node.bounds.top}][${node.bounds.right},${node.bounds.bottom}]",
                isClickable = node.isClickable,
                resourceId = node.resourceId
            ))
        }
        
        node.children.forEach { child ->
            flattenUITree(child, list)
        }
        
        return list
    }
    
    private fun buildEnhancedPrompt(customPrompt: String?, elements: List<UIElement>): String {
        val elementsSummary = if (elements.isNotEmpty()) {
            "\n\n已知的 UI 元素：\n" + elements.take(20).mapIndexed { i, e ->
                "[$i] ${e.text.take(30)} (${e.className?.substringAfterLast('.')})"
            }.joinToString("\n")
        } else ""
        
        return (customPrompt ?: DEFAULT_VISION_PROMPT) + elementsSummary
    }
}

/**
 * 屏幕分析结果
 */
data class ScreenAnalysisResult(
    val success: Boolean,
    val uiTree: UINode? = null,
    val elements: List<UIElement> = emptyList(),
    val screenshotBase64: String? = null,
    val visionDescription: String? = null,
    val screenWidth: Int? = null,
    val screenHeight: Int? = null,
    val error: String? = null,
    val analysisMode: MultimodalScreenAnalyzer.AnalysisMode
) {
    fun toContextString(): String {
        val parts = mutableListOf<String>()
        
        if (elements.isNotEmpty()) {
            parts.add("=== 可交互元素 ===")
            elements.forEachIndexed { index, element ->
                parts.add("[$index] ${element.text.ifBlank { element.className?.substringAfterLast('.') ?: "未知" }} " +
                    "(${if (element.isClickable) "可点击" else ""}) 坐标: ${element.bounds}")
            }
        }
        
        visionDescription?.let {
            parts.add("\n=== 视觉分析 ===")
            parts.add(it)
        }
        
        return parts.joinToString("\n")
    }
}

/**
 * UI 元素（简化版）
 */
data class UIElement(
    val id: Int,
    val text: String,
    val className: String?,
    val bounds: String?,
    val isClickable: Boolean,
    val resourceId: String?
)
