package com.elon.app

import java.util.Locale

internal fun modelLabel(name: String, model: String): String {
    return if (model.isBlank()) name else "$name [$model]"
}

internal fun displayModelLabel(provider: String, model: String, rawLabel: String): String {
    val modelName = model.trim()
    if (modelName.isNotBlank() && !modelName.equals("default", ignoreCase = true)) {
        return friendlyModelName(modelName)
    }
    return cleanModelLabel(rawLabel.ifBlank { modelLabel(provider, model) })
}

internal fun cleanModelLabel(label: String): String {
    val withoutCopilot = label.trim().replace(Regex("^copilot\\s*/\\s*", RegexOption.IGNORE_CASE), "")
    val bracket = Regex("\\[([^\\]]+)\\]\\s*$").find(withoutCopilot)
    if (bracket != null) return friendlyModelName(bracket.groupValues[1].trim())
    if (withoutCopilot.contains("/")) {
        val tail = withoutCopilot.substringAfterLast("/").trim()
        if (tail.isNotBlank()) return friendlyModelName(tail)
    }
    return friendlyModelName(withoutCopilot)
}

internal fun friendlyModelName(model: String): String {
    return when (model.trim().lowercase(Locale.US)) {
        "gpt-4o" -> "GPT-4o"
        "gpt-4o-mini" -> "GPT-4o mini"
        "gpt-4.1" -> "GPT-4.1"
        "gpt-4.5-preview" -> "GPT-4.5 Preview"
        "claude-3.5-sonnet", "claude-3-5-sonnet-20241022" -> "Claude 3.5 Sonnet"
        "claude-3.7-sonnet", "claude-3-7-sonnet-20250219" -> "Claude 3.7 Sonnet"
        "claude-sonnet-4", "claude-sonnet-4-5" -> "Claude Sonnet 4"
        "o1-mini" -> "o1 mini"
        "o1-preview" -> "o1 preview"
        "o3-mini" -> "o3 mini"
        "gemini-2.0-flash", "gemini-2.0-flash-001" -> "Gemini 2.0 Flash"
        "gemini-2.5-pro", "gemini-2.5-pro-preview" -> "Gemini 2.5 Pro"
        else -> model.trim()
    }
}

internal fun shortModelLabel(label: String): String {
    val m = cleanModelLabel(label)
    return when {
        m.startsWith("Codex", ignoreCase = true) -> "Codex"
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
