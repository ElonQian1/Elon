// infrastructure/voice/SmartIntentAnalyzerAdapter.kt
// module: infrastructure/voice | layer: infrastructure | role: intent-analyzer-adapter
// summary: 智能意图分析器适配器 - 将 IntentAnalyzer 适配为 StreamingIntentAnalyzer

package com.elon.app.agent.infrastructure.voice

import android.util.Log
import com.elon.app.agent.application.IntentAnalyzer
import com.elon.app.agent.application.conversation.IntentAnalysisResult
import com.elon.app.agent.application.conversation.StreamingIntentAnalyzer

/**
 * 🧠 智能意图分析器适配器
 * 
 * 将现有的 IntentAnalyzer 适配为 ConversationManager 所需的 StreamingIntentAnalyzer 接口
 * 
 * 功能：
 * 1. 调用 IntentAnalyzer 判断输入是否完整
 * 2. 判断是否为操作性请求（需要执行任务）
 * 3. 提供置信度评估
 */
class SmartIntentAnalyzerAdapter(
    private val intentAnalyzer: IntentAnalyzer
) : StreamingIntentAnalyzer {
    
    companion object {
        private const val TAG = "SmartIntentAdapter"
        
        // 操作性请求关键词
        private val OPERATION_KEYWORDS = listOf(
            // 打开类
            "打开", "启动", "运行", "进入", "开启",
            // 搜索类
            "搜索", "搜一下", "查一下", "找", "查找", "搜",
            // 发送类
            "发送", "发", "回复", "转发", "分享",
            // 点击类
            "点击", "点", "按", "选择", "选",
            // 请求前缀
            "帮我", "帮忙", "请", "麻烦", "能不能", "可以",
            // 浏览类
            "看看", "看一下", "浏览", "查看",
            // 导航类
            "返回", "退出", "关闭", "后退",
            // 获取类
            "获取", "抓取", "采集", "下载",
            // 滑动类
            "滑动", "向上", "向下", "翻页"
        )
        
        // 纯对话/问答关键词
        private val CHAT_KEYWORDS = listOf(
            "是什么", "什么是", "怎么样", "如何", "为什么",
            "可以吗", "能吗", "会吗", "吗？", "吗?",
            "你好", "早上好", "晚上好", "嗨", "hello", "hi",
            "谢谢", "感谢", "不客气", "再见", "拜拜",
            "你是谁", "你叫什么", "介绍一下"
        )
    }
    
    override suspend fun analyze(input: String): IntentAnalysisResult {
        Log.d(TAG, "🧠 [意图分析开始] 输入: $input")
        
        val startTime = System.currentTimeMillis()
        
        // 1. 首先用现有的 IntentAnalyzer 判断是否完整
        val analysisResult = intentAnalyzer.analyze(input)
        Log.d(TAG, "🧠 [AI分析结果] complete=${analysisResult.isComplete}, goal=${analysisResult.goal}")
        
        // 2. 判断是否为操作性请求
        val isOperation = isOperationRequest(input)
        Log.d(TAG, "🧠 [操作判断] isOperation=$isOperation")
        
        // 3. 计算置信度
        val confidence = calculateConfidence(input, analysisResult.isComplete, isOperation)
        
        val elapsed = System.currentTimeMillis() - startTime
        Log.i(TAG, "✅ [意图分析完成] (${elapsed}ms) complete=${analysisResult.isComplete}, operation=$isOperation, confidence=$confidence")
        
        return IntentAnalysisResult(
            normalizedInput = analysisResult.goal,
            isComplete = analysisResult.isComplete,
            isOperation = isOperation,
            confidence = confidence,
            hint = if (!analysisResult.isComplete) generateHint(input) else null
        )
    }
    
    /**
     * 判断是否为操作性请求
     */
    private fun isOperationRequest(input: String): Boolean {
        val normalized = input.lowercase()
        
        // 1. 检查操作关键词
        val hasOperationKeyword = OPERATION_KEYWORDS.any { normalized.contains(it) }
        
        // 2. 检查纯对话关键词
        val hasChatKeyword = CHAT_KEYWORDS.any { normalized.contains(it) }
        
        // 3. 如果有操作关键词且不是纯问句，认为是操作请求
        if (hasOperationKeyword && !hasChatKeyword) {
            return true
        }
        
        // 4. 如果同时有操作和对话关键词，检查是否以操作词开头
        if (hasOperationKeyword && hasChatKeyword) {
            return OPERATION_KEYWORDS.any { normalized.startsWith(it) }
        }
        
        // 5. 检查是否是简短的 APP 名称（隐含打开意图）
        val appNames = listOf(
            "微信", "抖音", "小红书", "淘宝", "京东", "支付宝",
            "qq", "微博", "b站", "美团", "饿了么", "高德",
            "设置", "相册", "相机", "电话", "短信"
        )
        if (input.length <= 5 && appNames.any { normalized.contains(it) }) {
            return true
        }
        
        return false
    }
    
    /**
     * 计算置信度
     */
    private fun calculateConfidence(input: String, isComplete: Boolean, isOperation: Boolean): Float {
        var confidence = 0.5f
        
        // 完整性加分
        if (isComplete) confidence += 0.2f
        
        // 有明确操作词加分
        if (isOperation) confidence += 0.2f
        
        // 长度合理加分
        if (input.length in 4..30) confidence += 0.1f
        
        return confidence.coerceIn(0f, 1f)
    }
    
    /**
     * 生成提示（当输入不完整时）
     */
    private fun generateHint(input: String): String {
        val normalized = input.lowercase()
        
        return when {
            normalized.endsWith("打开") || normalized.endsWith("启动") -> "打开什么呢？"
            normalized.endsWith("搜索") || normalized.endsWith("查找") -> "搜索什么内容？"
            normalized.endsWith("发送") || normalized.endsWith("发") -> "发送给谁？发什么内容？"
            normalized.contains("帮我") && input.length < 5 -> "帮你做什么呢？"
            else -> "请继续说..."
        }
    }
}
