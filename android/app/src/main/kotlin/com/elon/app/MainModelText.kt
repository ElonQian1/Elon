package com.elon.app

import java.util.Locale

private val PROVIDER_PREFIX_REGEX = Regex(
    "^(?:github\\s*copilot|github\\s*(?:版本|版)?|copilot|codex\\s*cli|codex\\s*(?:版本|版)?|hunyuan|tokenhub|混元)\\s*/\\s*",
    RegexOption.IGNORE_CASE
)

internal fun modelLabel(name: String, model: String): String {
    return if (model.isBlank()) name else "$name [$model]"
}

internal fun displayModelLabel(provider: String, model: String, rawLabel: String): String {
    val modelName = model.trim()
    if (modelName.isNotBlank() && !modelName.equals("default", ignoreCase = true)) {
        return friendlyModelName(modelName)
    }

    val raw = rawLabel.trim()
    if (raw.isNotBlank()) {
        return cleanModelLabel(raw)
    }
    return providerGroupTitle(provider)
}

internal fun cleanModelLabel(label: String): String {
    val raw = label.trim()
    if (raw.isBlank()) return raw
    val normalized = stripProviderPrefix(raw)
    if (normalized.contains("/")) {
        val model = stripProviderPrefix(normalized.substringAfterLast("/").trim())
        if (model.isNotBlank()) {
            return friendlyModelName(model)
        }
    }
    providerDisplayNameOrNull(normalized)?.let { return it }
    return friendlyModelName(normalized)
}

internal fun providerGroupTitle(provider: String): String {
    return providerDisplayNameOrNull(provider) ?: provider.trim().ifBlank { "其他" }
}

private fun providerDisplayNameOrNull(provider: String): String? {
    val compact = provider.trim().replace(" ", "").lowercase(Locale.US)
    return when (compact) {
        "copilot", "github", "githubcopilot", "github版", "github版本" -> "GitHub"
        "codex", "codexcli", "codex版", "codex版本" -> "Codex"
        "hunyuan", "tokenhub", "混元" -> "混元"
        "openai" -> "OpenAI"
        "deepseek" -> "DeepSeek"
        "claude", "anthropic" -> "Claude"
        "custom", "自定义" -> "自定义"
        "local", "localcli", "本地模型" -> "本地模型"
        else -> null
    }
}

internal fun friendlyModelName(model: String): String {
    return when (model.trim().lowercase(Locale.US)) {
        "gpt-4o" -> "GPT-4o"
        "gpt-4o-mini" -> "GPT-4o mini"
        "gpt-4.1" -> "GPT-4.1"
        "gpt-4.5-preview" -> "GPT-4.5 Preview"
        "gpt-5" -> "GPT-5"
        "gpt-5-codex" -> "GPT-5 Codex"
        "gpt-5.1" -> "GPT-5.1"
        "gpt-5.1-codex" -> "GPT-5.1 Codex"
        "gpt-5.3-codex-spark" -> "GPT-5.3 Codex Spark"
        "gpt-5.4" -> "GPT-5.4"
        "gpt-5.4-mini" -> "GPT-5.4 mini"
        "gpt-5.5" -> "GPT-5.5"
        "claude-3.5-sonnet", "claude-3-5-sonnet-20241022" -> "Claude 3.5 Sonnet"
        "claude-3.7-sonnet", "claude-3-7-sonnet-20250219" -> "Claude 3.7 Sonnet"
        "claude-sonnet-4", "claude-sonnet-4-5" -> "Claude Sonnet 4"
        "o1-mini" -> "o1 mini"
        "o1-preview" -> "o1 preview"
        "o3-mini" -> "o3 mini"
        "gemini-2.0-flash", "gemini-2.0-flash-001" -> "Gemini 2.0 Flash"
        "gemini-2.5-pro", "gemini-2.5-pro-preview" -> "Gemini 2.5 Pro"
        "hunyuan-turbo" -> "混元 Turbo"
        "hunyuan-2.0-instruct-20251111" -> "混元 2.0 Instruct"
        "hy-image-v3.0" -> "混元生图 3.0"
        else -> model.trim()
    }
}

internal fun shortModelLabel(label: String): String {
    val m = cleanModelLabel(label)
    return when {
        m.startsWith("GPT-5.5", ignoreCase = true) -> codexCompactLabel("5.5", m)
        m.startsWith("GPT-5.4 mini", ignoreCase = true) -> codexCompactLabel("5.4m", m)
        m.startsWith("GPT-5.4", ignoreCase = true) -> codexCompactLabel("5.4", m)
        m.startsWith("Codex", ignoreCase = true) -> "Codex"
        m.startsWith("GitHub", ignoreCase = true) -> "GitHub"
        m.startsWith("混元") -> m.replace(" ", "").take(6)
        m.startsWith("GPT-5.1") -> "GPT-5.1"
        m.startsWith("GPT-5") -> "GPT-5"
        m.startsWith("服务器默认") -> "默认"
        m.startsWith("自定义") -> "自定"
        m.startsWith("GPT") -> m.replace(" ", "").take(6)
        m.startsWith("gpt-") -> m.take(6)
        m.startsWith("o1") || m.startsWith("o3") -> m.replace(" ", "").take(5)
        m.startsWith("Claude") -> "Cl" + m.substringAfterLast(" ").take(3)
        m.startsWith("Gemini") -> "G" + m.substringAfterLast(" ").take(4)
        else -> m.take(6)
    }
}

private fun codexCompactLabel(prefix: String, label: String): String {
    val lower = label.lowercase(Locale.US)
    val suffix = when {
        "xhigh" in lower -> "X"
        "high" in lower -> "高"
        "medium" in lower -> "中"
        "low" in lower -> "低"
        "minimal" in lower -> "微"
        else -> ""
    }
    return (prefix + suffix).take(6)
}

private fun stripProviderPrefix(label: String): String {
    var value = label.trim()
    while (true) {
        val next = value.replace(PROVIDER_PREFIX_REGEX, "")
        if (next == value) return value
        value = next.trim()
    }
}
