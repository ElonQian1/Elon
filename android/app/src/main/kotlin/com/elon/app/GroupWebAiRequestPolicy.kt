package com.elon.app

import org.json.JSONObject

internal object GroupWebAiRequestPolicy {
    fun consent(config: GroupAiConfiguration): String {
        val destination = if (config.engine == GroupAiEngine.GOOGLE)
            "Google AI 模式的独立新会话。Google 可能在账号历史中保存这次对话"
        else "你账号内本群专属的 ChatGPT 项目长期会话，结合此前明确提交的本群上下文分析"
        return "将把这个群最近最多 30 条文字发送到$destination，并把回答发回群里。不会读取个人会话，也不会读取群附件。"
    }

    fun permitted(config: GroupAiConfiguration, receipt: JSONObject): Boolean {
        val provider = config.engine.providerId ?: return false
        // An older server must never authorize Google while recording a ChatGPT dispatch.
        val observed = receipt.optString("web_provider", WebChatProviderId.CHATGPT_WEB.wireValue)
        return receipt.optBoolean("dispatch_permit") && observed == provider.wireValue
    }
}
