package com.elon.app

/** Group ownership is independent; provider identity and row presentation are shared. */
internal object GroupAiProviderChoices {
    fun options(config: GroupAiConfiguration): List<ChatAiChoice> = GroupAiEngine.entries.map { engine ->
        val provider = engine.providerId?.let(WebChatProviderRegistry::get)
        ChatAiChoice(
            engine.name, engine.label,
            when (engine) {
                GroupAiEngine.CHATGPT -> "模型与档位"
                GroupAiEngine.GOOGLE -> "预览版"
                GroupAiEngine.WORK -> config.work.label
            },
            provider?.avatarResId ?: R.drawable.ic_msg_ai_reply,
            config.engine == engine, "group-ai-provider:${engine.name}",
        )
    }
}
