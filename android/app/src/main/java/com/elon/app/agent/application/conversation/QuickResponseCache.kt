// application/conversation/QuickResponseCache.kt
// module: application/conversation | layer: application | role: quick-response
// summary: 快速响应缓存 - 实现即时响应，零延迟回复常见问候

package com.elon.app.agent.application.conversation

import com.elon.app.agent.domain.conversation.Emotion
import com.elon.app.agent.domain.conversation.Response
import com.elon.app.agent.domain.conversation.ResponseTier

/**
 * ⚡ 快速响应缓存
 * 
 * 实现分层响应的核心组件：
 * - INSTANT 层：预设响应，本地匹配，<100ms
 * - FAST 层：规则匹配 + 模板，<500ms
 */
object QuickResponseCache : QuickResponseProvider {
    
    // ==================== 即时响应库 (INSTANT) ====================
    
    /**
     * 问候语响应
     * 触发词 → 响应列表（随机选择一个）
     */
    private val greetingResponses = mapOf(
        // 你好类
        listOf("你好", "您好") to listOf(
            "你好！有什么可以帮你的？",
            "你好！我在呢。",
            "嗨，有什么需要帮忙的吗？"
        ),
        
        // 嗨类
        listOf("嗨", "嘿", "hi", "hello", "hey") to listOf(
            "嗨！",
            "Hi！有什么事？",
            "嘿！我在听。"
        ),
        
        // 早安类
        listOf("早", "早上好", "早安") to listOf(
            "早上好！今天有什么安排？",
            "早安！需要我帮你做什么吗？"
        ),
        
        // 晚安类
        listOf("晚安", "拜拜", "再见", "bye") to listOf(
            "晚安！有需要随时叫我。",
            "拜拜！",
            "再见！"
        ),
        
        // 感谢类
        listOf("谢谢", "感谢", "多谢", "thanks", "thank you") to listOf(
            "不客气！",
            "不用谢！",
            "没事！还有什么需要帮忙的吗？"
        ),
        
        // 确认类
        listOf("好的", "好", "ok", "行", "可以", "没问题") to listOf(
            "好的！",
            "明白！"
        ),
        
        // 取消类
        listOf("算了", "不用了", "取消", "停") to listOf(
            "好的，已取消。",
            "明白，不做了。"
        )
    )
    
    // ==================== 常见问题响应 (FAST) ====================
    
    private val faqResponses = mapOf(
        // 自我介绍
        listOf("你是谁", "你叫什么", "介绍一下你自己") to Response(
            tier = ResponseTier.FAST,
            text = "我是你的手机助手，可以帮你操作手机、回答问题。试试说「打开微信」或「今天天气怎么样」。",
            emotion = Emotion.HAPPY
        ),
        
        // 能力询问
        listOf("你能做什么", "你会什么", "有什么功能") to Response(
            tier = ResponseTier.FAST,
            text = "我可以帮你：\n1. 打开应用，比如「打开微信」\n2. 搜索信息，比如「搜索附近美食」\n3. 发消息，比如「给妈妈发微信」\n4. 闲聊，比如「讲个笑话」",
            emotion = Emotion.HELPFUL
        ),
        
        // 使用方法
        listOf("怎么用", "怎么使用", "如何使用") to Response(
            tier = ResponseTier.FAST,
            text = "直接告诉我你想做什么就行！比如说「打开淘宝」、「搜索今日新闻」、「帮我订个闹钟」。",
            emotion = Emotion.HELPFUL
        )
    )
    
    // ==================== 语气词/无意义输入 ====================
    
    private val fillerResponses = mapOf(
        listOf("嗯", "啊", "哦", "呃") to listOf(
            "我在听，请继续。",
            "嗯？",
            "请说。"
        ),
        
        listOf("那个", "就是", "这个") to listOf(
            "什么呢？",
            "请继续说。"
        )
    )
    
    // ==================== 公开方法 ====================
    
    /**
     * 尝试获取快速响应
     * 
     * @return 匹配的响应，或null（需要AI处理）
     */
    override fun tryGetQuickResponse(input: String): Response? {
        val normalized = input.trim().lowercase()

        // 1. 匹配问候语 (INSTANT)
        //    既支持精确匹配，也支持"你好你好""你好你好你好"这类重复问候，
        //    但带操作词的（如"你好帮我打开微信"）不当问候处理，交给后续意图分析。
        greetingResponses.forEach { (triggers, responses) ->
            val exact = triggers.any { normalized == it || normalized == "${'$'}{it}啊" || normalized == "${'$'}{it}呀" }
            val looseGreeting = !hasOperationKeyword(normalized) &&
                normalized.length <= 8 &&
                triggers.any { normalized.contains(it) }
            if (exact || looseGreeting) {
                return Response(
                    tier = ResponseTier.INSTANT,
                    text = responses.random(),
                    emotion = Emotion.HAPPY
                )
            }
        }
        
        // 2. 匹配常见问题 (FAST)
        faqResponses.forEach { (triggers, response) ->
            if (triggers.any { normalized.contains(it) }) {
                return response
            }
        }
        
        // 3. 匹配语气词 (INSTANT)
        fillerResponses.forEach { (triggers, responses) ->
            if (triggers.any { normalized == it }) {
                return Response(
                    tier = ResponseTier.INSTANT,
                    text = responses.random(),
                    emotion = Emotion.CURIOUS
                )
            }
        }
        
        // 4. 短输入且无操作关键词 → 当作聊天
        if (input.length <= 3 && !hasOperationKeyword(normalized)) {
            return Response(
                tier = ResponseTier.INSTANT,
                text = "我在听，请说具体一点。",
                emotion = Emotion.CURIOUS
            )
        }
        
        // 无法快速匹配
        return null
    }
    
    /**
     * 获取打断时的即时响应
     */
    fun getInterruptionResponse(): Response {
        val responses = listOf(
            "好的，我听你说。",
            "请说。",
            "嗯，我在听。"
        )
        return Response(
            tier = ResponseTier.INSTANT,
            text = responses.random(),
            emotion = Emotion.CALM
        )
    }
    
    /**
     * 获取思考中的过渡响应
     */
    fun getThinkingResponse(): Response {
        val responses = listOf(
            "让我想想...",
            "嗯，稍等...",
            "我来看看..."
        )
        return Response(
            tier = ResponseTier.INSTANT,
            text = responses.random(),
            emotion = Emotion.THINKING
        )
    }
    
    /**
     * 获取错误恢复响应
     */
    fun getErrorRecoveryResponse(): Response {
        val responses = listOf(
            "抱歉，我没听清，能再说一遍吗？",
            "不好意思，可以再说一次吗？",
            "我没太明白，请再说一遍。"
        )
        return Response(
            tier = ResponseTier.FAST,
            text = responses.random(),
            emotion = Emotion.APOLOGETIC
        )
    }
    
    // ==================== 辅助方法 ====================
    
    private val operationKeywords = listOf(
        "打开", "启动", "搜索", "查找", "点击", "发送", "获取", "设置"
    )
    
    private fun hasOperationKeyword(input: String): Boolean {
        return operationKeywords.any { input.contains(it) }
    }
}
