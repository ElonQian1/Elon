package com.elon.app

private const val AUTO_CONVERSATION_TITLE_MAX_CHARS = 12

private val AUTO_CONVERSATION_PLACEHOLDER_TITLES = setOf(
    "一龙开发助手",
    "一龙项目",
    "项目开发会话",
    "等待你的第一个开发需求"
)

private val GENERIC_AUTO_CONVERSATION_TITLES = setOf(
    "新会话",
    "新的会话",
    "普通聊天",
    "我有一个想法",
    "我又有一个想法",
    "有一个想法",
    "有个想法",
    "聊聊想法",
    "讨论新想法",
    "帮我看看",
    "问个问题",
    "咨询一下"
)

private val TITLE_LEAD_IN_PREFIXES = listOf(
    "请你帮我",
    "麻烦你帮我",
    "能不能帮我",
    "可以帮我",
    "请帮我",
    "麻烦帮我",
    "再帮我",
    "在帮我",
    "帮我",
    "帮忙",
    "请你",
    "麻烦你",
    "我想让你",
    "我想请你",
    "我希望",
    "我需要",
    "我想要",
    "我想",
    "我要",
    "我又有一个想法",
    "我有一个想法",
    "有一个想法",
    "有个想法",
    "同理",
    "现在"
)

private val POLITE_ACTION_PREFIXES = listOf("请把", "请将", "请给", "请用", "请在")

private val TITLE_ACTION_WORDS = listOf(
    "自动生成",
    "重新生成",
    "改成",
    "改为",
    "修改为",
    "替换成",
    "换成",
    "变成",
    "修复",
    "排查",
    "优化",
    "调整",
    "替换",
    "新增",
    "添加",
    "删除",
    "移除",
    "隐藏",
    "显示",
    "同步",
    "更新",
    "发布",
    "安装",
    "部署",
    "生成",
    "总结",
    "打开",
    "关闭",
    "恢复",
    "保存",
    "记录",
    "查看",
    "查询",
    "排序",
    "置顶",
    "下载",
    "上传",
    "分享",
    "清理",
    "重命名",
    "迁移",
    "合并",
    "拆分",
    "创建",
    "做",
    "弄",
    "设计",
    "实现"
)

private val FILLER_AFTER_ACTION = Regex(
    "^(自动生成|重新生成|修复|排查|优化|调整|替换|新增|添加|删除|移除|隐藏|显示|同步|更新|发布|安装|部署|生成|总结|打开|关闭|恢复|保存|记录|查看|查询|排序|置顶|下载|上传|分享|清理|重命名|迁移|合并|拆分|创建|做|弄|设计|实现)(一下|一个|一条|这个|那个|当前|所有|全部|每一条)?"
)

private data class ConversationTitleCandidate(
    val title: String,
    val score: Int,
    val sourceIndex: Int
)

internal fun shouldAutoGenerateConversationTitle(conversation: AppConversation): Boolean {
    return !conversation.titleManuallyEdited
}

internal fun autoConversationTitleFromMessage(text: String): String {
    return bestConversationTitleCandidate(listOf(text))
        ?.title
        ?: fallbackConversationTitle(text)
}

internal fun updateConversationTitleFromFirstUserMessage(conversation: AppConversation): Boolean {
    if (!shouldAutoGenerateConversationTitle(conversation)) return false
    return updateConversationTitleFromMessages(conversation, null)
}

internal fun updateConversationTitleFromUserMessage(
    conversation: AppConversation,
    messageText: String
): Boolean {
    if (!shouldAutoGenerateConversationTitle(conversation)) return false
    return updateConversationTitleFromMessages(conversation, messageText)
}

private fun updateConversationTitleFromMessages(
    conversation: AppConversation,
    extraUserMessage: String?
): Boolean {
    val userMessages = conversation.messages
        .filter { it.role == "user" && it.content.isNotBlank() }
        .map { it.content }
        .toMutableList()
    extraUserMessage
        ?.takeIf { it.isNotBlank() }
        ?.let { userMessages.add(it) }
    val candidate = bestConversationTitleCandidate(userMessages) ?: return false
    val currentTitle = conversation.title.trim()
    if (!shouldReplaceConversationTitle(currentTitle, candidate)) return false
    if (currentTitle == candidate.title) return false
    conversation.title = candidate.title
    return true
}

private fun bestConversationTitleCandidate(messages: List<String>): ConversationTitleCandidate? {
    val candidates = messages
        .mapIndexedNotNull { index, message -> conversationTitleCandidate(message, index) }
    return candidates
        .sortedWith(
            compareByDescending<ConversationTitleCandidate> { it.score }
                .thenBy { it.sourceIndex }
                .thenBy { it.title.length }
        )
        .firstOrNull()
}

private fun conversationTitleCandidate(rawText: String, sourceIndex: Int): ConversationTitleCandidate? {
    val normalized = rawText.normalizedConversationTitleText()
    if (normalized.isBlank()) return null
    specialConversationTitle(normalized)?.let {
        return ConversationTitleCandidate(it, 12, sourceIndex)
    }

    val sentenceCandidates = normalized
        .split(Regex("[。！？!?；;]+"))
        .asSequence()
        .map { it.trimTitlePunctuation() }
        .filter { it.isNotBlank() }
        .take(6)
        .flatMap { sentence ->
            val cleaned = sentence
                .removeConversationTitleLeadIn()
                .trimTitlePunctuation()
            listOfNotNull(
                specialConversationTitle(cleaned)?.let {
                    ConversationTitleCandidate(it, 12, sourceIndex)
                },
                titleFromChangePattern(cleaned)?.let {
                    ConversationTitleCandidate(it, 11, sourceIndex)
                },
                titleFromActionPhrase(cleaned)?.let {
                    ConversationTitleCandidate(it, 9, sourceIndex)
                },
                titleFromGenericIdea(cleaned)?.let {
                    ConversationTitleCandidate(it, 2, sourceIndex)
                }
            ).asSequence()
        }
        .toList()

    return sentenceCandidates
        .sortedWith(
            compareByDescending<ConversationTitleCandidate> { it.score }
                .thenBy { it.title.length }
        )
        .firstOrNull()
        ?: ConversationTitleCandidate(fallbackConversationTitle(normalized), 3, sourceIndex)
}

private fun shouldReplaceConversationTitle(
    currentTitle: String,
    candidate: ConversationTitleCandidate
): Boolean {
    if (currentTitle.isBlank()) return true
    if (isPlaceholderConversationTitle(currentTitle)) return true
    if (isWeakConversationTitle(currentTitle)) return true
    val currentQuality = conversationTitleQuality(currentTitle)
    return candidate.score >= currentQuality + 2
}

private fun conversationTitleQuality(title: String): Int {
    val trimmed = title.trim()
    if (trimmed.isBlank()) return 0
    if (isPlaceholderConversationTitle(trimmed)) return 0
    if (trimmed in GENERIC_AUTO_CONVERSATION_TITLES) return 1
    var score = 4
    if (TITLE_ACTION_WORDS.any { trimmed.startsWith(it) }) score += 4
    if (trimmed.contains("…")) score -= 2
    if (titleLooksLikeRawLeadIn(trimmed)) score -= 2
    if (trimmed.contains("一个") || trimmed.contains("一下")) score -= 1
    return score.coerceAtLeast(0)
}

private fun isPlaceholderConversationTitle(title: String): Boolean {
    return title.startsWith("新会话") || title in AUTO_CONVERSATION_PLACEHOLDER_TITLES
}

private fun isWeakConversationTitle(title: String): Boolean {
    val trimmed = title.trim()
    return trimmed in GENERIC_AUTO_CONVERSATION_TITLES ||
        titleLooksLikeRawLeadIn(trimmed) ||
        (trimmed.contains("…") && (trimmed.contains("帮我") || trimmed.contains("一个")))
}

private fun titleLooksLikeRawLeadIn(title: String): Boolean {
    return TITLE_LEAD_IN_PREFIXES.any { title.startsWith(it) } ||
        title.startsWith("请") ||
        title.startsWith("麻烦")
}

private fun specialConversationTitle(text: String): String? {
    if (text.isBlank()) return null
    if (
        (text.contains("聊天列表") || text.contains("会话列表") || text.contains("当前聊天")) &&
        text.contains("标题") &&
        (text.contains("自动生成") || text.contains("短摘要") || text.contains("总结"))
    ) {
        return "自动生成聊天标题"
    }
    if (text.contains("打开应用") && (text.contains("更新") || text.contains("改为") || text.contains("改成"))) {
        return "将打开应用改为更新"
    }
    return null
}

private fun titleFromChangePattern(text: String): String? {
    val patterns = listOf(
        Regex("^(?:把|将)(.+?)(?:改成|改为|修改为|换成|替换成|变成)(.+)$"),
        Regex("^(.+?)(?:改成|改为|修改为|换成|替换成|变成)(.+)$")
    )
    patterns.forEach { pattern ->
        val match = pattern.find(text) ?: return@forEach
        val target = match.groupValues[1].cleanTitleObject()
        val result = match.groupValues[2].cleanTitleObject()
        if (target.isNotBlank() && result.isNotBlank()) {
            val title = "将${target.titlePart(5)}改为${result.titlePart(5)}"
            return summarize(title, AUTO_CONVERSATION_TITLE_MAX_CHARS)
        }
    }
    return null
}

private fun titleFromActionPhrase(text: String): String? {
    val action = TITLE_ACTION_WORDS
        .mapNotNull { word ->
            val index = text.indexOf(word)
            if (index >= 0) word to index else null
        }
        .minByOrNull { it.second }
        ?: return null
    if (action.first in setOf("改成", "改为", "修改为", "替换成", "换成", "变成")) return null
    val phrase = text.substring(action.second)
        .replace("自动生成的", "自动生成")
        .replace("生成的", "生成")
        .replace(FILLER_AFTER_ACTION) { match ->
            match.groupValues[1]
        }
        .cleanTitleObject()
    if (phrase.length < 2) return null
    return summarize(phrase, AUTO_CONVERSATION_TITLE_MAX_CHARS)
}

private fun titleFromGenericIdea(text: String): String? {
    val compact = text
        .removeConversationTitleLeadIn()
        .replace(Regex("\\s+"), "")
        .trimTitlePunctuation()
    return when {
        compact in setOf("我有一个想法", "我又有一个想法", "有一个想法", "有个想法") -> "讨论新想法"
        compact in setOf("帮我看看", "看一下", "看看这个") -> "查看问题"
        compact in setOf("你好", "在吗", "哈喽", "hello") -> "普通聊天"
        else -> null
    }
}

private fun String.removeConversationTitleLeadIn(): String {
    TITLE_LEAD_IN_PREFIXES.firstOrNull { startsWith(it) && length > it.length + 1 }?.let {
        return removePrefix(it)
    }
    POLITE_ACTION_PREFIXES.firstOrNull { startsWith(it) && length > it.length + 1 }?.let {
        return removePrefix("请")
    }
    return this
}

private fun String.normalizedConversationTitleText(): String {
    return replace(Regex("!\\[[^\\]]*]\\([^)]*\\)"), "图片")
        .replace(Regex("\\[[^\\]]*]\\([^)]*\\)"), " ")
        .replace(Regex("```[\\s\\S]*?```"), " ")
        .replace(Regex("[#>*`]+"), " ")
        .replace(Regex("\\s+"), " ")
        .trimTitlePunctuation()
}

private fun String.cleanTitleObject(): String {
    return trimTitlePunctuation()
        .replace(Regex("^(这个|那个|这里的|这里|当前|一下|一个|一条|每一条|所有|全部)+"), "")
        .replace(Regex("^(的|地|得)+"), "")
        .replace(Regex("(这个|那个|一下|吧|呢|哈|呀)$"), "")
        .trimTitlePunctuation()
}

private fun String.titlePart(maxChars: Int): String {
    val cleaned = cleanTitleObject()
    return if (cleaned.length <= maxChars) cleaned else cleaned.take(maxChars)
}

private fun String.trimTitlePunctuation(): String {
    return trim()
        .trim('：', ':', '，', ',', '。', '.', '、', ' ', '！', '!', '？', '?', '；', ';', '“', '”', '"', '\'')
}

private fun fallbackConversationTitle(text: String): String {
    val cleaned = text
        .normalizedConversationTitleText()
        .removeConversationTitleLeadIn()
        .cleanTitleObject()
    return summarize(cleaned.ifBlank { "普通聊天" }, AUTO_CONVERSATION_TITLE_MAX_CHARS)
}
