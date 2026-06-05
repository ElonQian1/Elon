// application/IntentAnalyzer.kt
// module: application | layer: application | role: intent-analyzer
// summary: 轻量级意图分析器 - 判断用户输入是否完整

package com.elon.app.agent.application

import android.content.Context
import android.util.Log
import com.elon.app.agent.infrastructure.ai.AIClientFactory
import com.google.gson.Gson
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * 🧠 轻量级意图分析器
 * 
 * 用于快速判断用户输入是否完整，不需要 AccessibilityService
 *
 * **重构后**：不再接收 apiKey，通过 [AIClientFactory] 自动选择 AI 链路。
 */
class IntentAnalyzer(context: Context) {
    
    companion object {
        private const val TAG = "IntentAnalyzer"
        
        // APP 名称关键词
        private val APP_KEYWORDS = listOf(
            "小红书", "微信", "抖音", "淘宝", "京东", "支付宝",
            "qq", "微博", "b站", "哔哩哔哩", "美团", "饿了么",
            "高德", "百度地图", "网易云", "酷狗", "喜马拉雅",
            "今日头条", "知乎", "豆瓣", "闲鱼", "拼多多",
            "设置", "相册", "相机", "电话", "短信", "日历"
        )
        
        // 操作关键词
        private val OPERATION_KEYWORDS = listOf(
            "打开", "启动", "运行", "进入",
            "搜索", "查找", "查询", "找",
            "点击", "点一下", "按",
            "返回", "后退", "退出",
            "发送", "转发", "分享",
            "获取", "抓取", "采集"
        )
    }
    
    private val gson = Gson()
    private val aiClient = AIClientFactory.create(context)
    
    /**
     * 分析结果
     */
    data class AnalysisResult(
        val isComplete: Boolean,    // 表述是否完整
        val goal: String,           // 清理后的目标
        val needAI: Boolean = false // 是否使用了 AI 分析
    )
    
    /**
     * 快速分析：判断输入是否完整
     * 
     * 优先使用本地规则，复杂情况才调 AI
     */
    suspend fun analyze(input: String): AnalysisResult {
        val trimmed = input.trim()
        
        // 1. 尝试快速规则匹配
        val quickResult = quickAnalyze(trimmed)
        if (quickResult != null) {
            Log.d(TAG, "⚡ 快速匹配: $trimmed → complete=${quickResult.isComplete}")
            return quickResult
        }
        
        // 2. 需要 AI 分析
        return aiAnalyze(trimmed)
    }
    
    /**
     * ⚡ 快速本地分析（无需 AI）
     */
    private fun quickAnalyze(input: String): AnalysisResult? {
        val normalized = input.lowercase()
        
        val hasOperation = OPERATION_KEYWORDS.any { normalized.contains(it) }
        val hasApp = APP_KEYWORDS.any { normalized.contains(it) }
        
        // 情况1：操作词 + APP名 = 完整（如"打开微信"）
        if (hasOperation && hasApp) {
            return AnalysisResult(isComplete = true, goal = input)
        }
        
        // 情况2：单独 APP 名也完整（隐含"打开"）
        if (hasApp && !hasOperation && input.length <= 10) {
            return AnalysisResult(isComplete = true, goal = "打开$input")
        }
        
        // 情况3：只有操作词，没有目标 = 不完整
        if (hasOperation && !hasApp && input.length < 6) {
            // 检查是否以操作词结尾
            val endsWithOp = OPERATION_KEYWORDS.any { normalized.endsWith(it) }
            if (endsWithOp) {
                return AnalysisResult(isComplete = false, goal = input)
            }
        }
        
        // 情况4：较长的句子（8个字以上）且有操作词 = 大概率完整
        if (input.length >= 8 && hasOperation) {
            return AnalysisResult(isComplete = true, goal = input)
        }
        
        // 情况5：明显不完整的结尾
        val incompleteEndings = listOf("然后", "接着", "再", "还有", "和", "的")
        if (incompleteEndings.any { normalized.endsWith(it) }) {
            return AnalysisResult(isComplete = false, goal = input)
        }

        // 情况6：既无操作词也无 APP 名 → 这是闲聊/问答，不是手机操作指令。
        // 完整性判断只对"操作指令"有意义（要补全才能执行），闲聊本来就完整，
        // 直接判定完整、不调 AI，避免"你好"这类闲聊误发服务器 CLI 判断而卡顿/报错。
        if (!hasOperation && !hasApp) {
            return AnalysisResult(isComplete = true, goal = input)
        }

        // 无法确定，需要 AI
        return null
    }
    
    /**
     * 🧠 AI 分析
     */
    private suspend fun aiAnalyze(input: String): AnalysisResult = withContext(Dispatchers.IO) {
        try {
            val prompt = """
判断这句话是否是一个完整的手机操作指令。

用户说："$input"

只返回 JSON：
{"complete": true或false, "goal": "清理后的指令"}

示例：
"打开微信" → {"complete": true, "goal": "打开微信"}
"打开" → {"complete": false, "goal": "打开"}
"搜索热门" → {"complete": false, "goal": "搜索热门"}
"在小红书搜索美食" → {"complete": true, "goal": "在小红书搜索美食"}
"帮我" → {"complete": false, "goal": "帮我"}
""".trim()
            
            val messages = listOf(
                Message(role = "user", content = prompt)
            )
            
            val response = aiClient.chat(messages)
            parseAIResponse(response, input)
        } catch (e: Exception) {
            Log.e(TAG, "AI 分析失败: ${e.message}")
            // 失败时默认为完整（避免卡住）
            AnalysisResult(isComplete = true, goal = input, needAI = true)
        }
    }
    
    private fun parseAIResponse(response: String, original: String): AnalysisResult {
        return try {
            // 提取 JSON
            val jsonMatch = Regex("\\{[^}]+\\}").find(response)
            val jsonStr = jsonMatch?.value ?: throw Exception("No JSON found")
            
            val map = gson.fromJson(jsonStr, Map::class.java)
            val complete = map["complete"] as? Boolean ?: true
            val goal = map["goal"] as? String ?: original
            
            AnalysisResult(isComplete = complete, goal = goal, needAI = true)
        } catch (e: Exception) {
            Log.e(TAG, "解析 AI 响应失败: ${e.message}")
            AnalysisResult(isComplete = true, goal = original, needAI = true)
        }
    }
}
