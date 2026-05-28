// infrastructure/vision/ScriptGenerator.kt
// module: infrastructure/vision | layer: infrastructure | role: script-generator
// summary: 智能脚本生成器，根据目标和屏幕分析生成可执行脚本

package com.elon.app.agent.infrastructure.vision

import android.graphics.Rect

/**
 * AI Agent 脚本步骤
 */
data class AgentStep(
    val stepId: Int,
    val action: String,
    val params: Map<String, Any>,
    val description: String,
    val outputKey: String? = null
)

/**
 * AI Agent 脚本
 */
data class AgentScript(
    val name: String,
    val goal: String,
    val description: String,
    val steps: List<AgentStep>,
    val metadata: Map<String, String>
)

/**
 * 智能脚本生成器
 * 
 * 功能：
 * - 解析自然语言目标
 * - 根据屏幕分析结果生成执行计划
 * - 生成可复用的 AI Agent 脚本
 */
class ScriptGenerator {
    
    /**
     * 根据目标生成脚本
     */
    fun generateScript(
        goal: String,
        analysis: ScreenAnalysis,
        deviceId: String? = null
    ): AgentScript {
        val goalLower = goal.lowercase()
        
        // 解析目标关键词
        val wantsHotContent = goalLower.contains("热") || goalLower.contains("万") ||
                goalLower.contains("高赞") || goalLower.contains("点赞")
        val wantsComments = goalLower.contains("评论")
        val wantsXiaohongshu = goalLower.contains("小红书") || analysis.appContext == "xiaohongshu"
        
        // 解析数量
        val commentCount = extractNumberFromGoal(goalLower, "评论")?.toInt() ?: 5
        val likeThreshold = extractNumberFromGoal(goalLower, "万")?.let { it * 10000 } ?: 10000.0
        
        val steps = mutableListOf<AgentStep>()
        var stepId = 1
        
        // 步骤 1：启动应用（如果需要）
        if (wantsXiaohongshu && analysis.pageType != "feed_list") {
            steps.add(AgentStep(
                stepId = stepId++,
                action = "launch_app",
                params = mapOf(
                    "package" to "com.xingin.xhs",
                    "activity" to "com.xingin.xhs.index.v2.IndexActivityV2"
                ),
                description = "启动小红书"
            ))
        }
        
        // 步骤 2：点击高热度内容
        if (wantsHotContent && analysis.hotContent.isNotEmpty()) {
            val firstHot = analysis.hotContent.first()
            val centerX = firstHot.bounds.centerX()
            val centerY = firstHot.bounds.centerY()
            
            steps.add(AgentStep(
                stepId = stepId++,
                action = "tap",
                params = mapOf("x" to centerX, "y" to centerY),
                description = "点击高热度内容 (${firstHot.value.toLong()}赞)"
            ))
        } else if (wantsHotContent) {
            // 需要先查找
            steps.add(AgentStep(
                stepId = stepId++,
                action = "find_elements",
                params = mapOf(
                    "selector" to mapOf(
                        "type" to "engagement",
                        "min_value" to likeThreshold,
                        "metric" to "likes"
                    ),
                    "limit" to 1
                ),
                description = "查找点赞超过${likeThreshold.toLong()}的笔记",
                outputKey = "hot_notes"
            ))
            
            steps.add(AgentStep(
                stepId = stepId++,
                action = "tap_relative",
                params = mapOf(
                    "relative_to" to "hot_notes[0]",
                    "position" to "center"
                ),
                description = "点击找到的高热度笔记"
            ))
        }
        
        // 步骤 3：等待页面加载
        steps.add(AgentStep(
            stepId = stepId++,
            action = "wait",
            params = mapOf("duration_ms" to 2000),
            description = "等待详情页加载"
        ))
        
        // 步骤 4：提取评论
        if (wantsComments) {
            steps.add(AgentStep(
                stepId = stepId++,
                action = "extract_comments",
                params = mapOf(
                    "count" to commentCount,
                    "scroll_if_needed" to true,
                    "filter" to mapOf(
                        "min_length" to 5,
                        "exclude_author" to true
                    )
                ),
                description = "提取前${commentCount}条有意义评论",
                outputKey = "extracted_comments"
            ))
        }
        
        return AgentScript(
            name = "auto_generated_${System.currentTimeMillis()}",
            goal = goal,
            description = "AI 自动生成的脚本，目标：$goal",
            steps = steps,
            metadata = mapOf(
                "generated_by" to "android_agent",
                "app_context" to analysis.appContext,
                "page_context" to analysis.pageType
            )
        )
    }
    
    /**
     * 从目标描述中提取数字
     */
    private fun extractNumberFromGoal(goal: String, context: String): Double? {
        val contextIndex = goal.indexOf(context)
        if (contextIndex == -1) return null
        
        // 向前查找数字
        val before = goal.substring(0, contextIndex)
        val pattern = Regex("""(\d+)""")
        return pattern.findAll(before).lastOrNull()?.groupValues?.get(1)?.toDoubleOrNull()
    }
    
    /**
     * 将脚本转换为 JSON 字符串（用于保存或传输）
     */
    fun toJson(script: AgentScript): String {
        return buildString {
            appendLine("{")
            appendLine("  \"format\": \"ai_agent_script\",")
            appendLine("  \"version\": \"1.0.0\",")
            appendLine("  \"name\": \"${script.name}\",")
            appendLine("  \"description\": \"${script.description}\",")
            appendLine("  \"goal\": \"${script.goal}\",")
            appendLine("  \"steps\": [")
            script.steps.forEachIndexed { index, step ->
                append("    {")
                append("\"step_id\": ${step.stepId}, ")
                append("\"action\": \"${step.action}\", ")
                append("\"description\": \"${step.description}\"")
                step.outputKey?.let { append(", \"output_key\": \"$it\"") }
                append(", \"params\": ${paramsToJson(step.params)}")
                append("}")
                if (index < script.steps.size - 1) appendLine(",") else appendLine()
            }
            appendLine("  ],")
            appendLine("  \"metadata\": {")
            script.metadata.entries.forEachIndexed { index, (key, value) ->
                append("    \"$key\": \"$value\"")
                if (index < script.metadata.size - 1) appendLine(",") else appendLine()
            }
            appendLine("  }")
            append("}")
        }
    }
    
    private fun paramsToJson(params: Map<String, Any>): String {
        return buildString {
            append("{")
            params.entries.forEachIndexed { index, (key, value) ->
                append("\"$key\": ")
                append(valueToJson(value))
                if (index < params.size - 1) append(", ")
            }
            append("}")
        }
    }
    
    @Suppress("UNCHECKED_CAST")
    private fun valueToJson(value: Any): String {
        return when (value) {
            is String -> "\"$value\""
            is Number -> value.toString()
            is Boolean -> value.toString()
            is Map<*, *> -> paramsToJson(value as Map<String, Any>)
            else -> "\"${value}\""
        }
    }
}
