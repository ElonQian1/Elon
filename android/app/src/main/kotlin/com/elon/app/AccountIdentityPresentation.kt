package com.elon.app

internal fun maskedYilongAccount(account: String?): String {
    val value = account?.trim().orEmpty()
    if (value.isBlank()) return "当前登录账号"
    if (value.length >= 7 && value.all(Char::isDigit)) {
        return "${value.take(3)}****${value.takeLast(2)}"
    }
    return if (value.contains('@')) maskedIdentityEmail(value) else value
}

internal fun maskedIdentityEmail(email: String?): String {
    val value = email?.trim().orEmpty()
    val separator = value.indexOf('@')
    if (separator <= 0 || separator == value.lastIndex) return value.ifBlank { "Google 身份" }
    val local = value.substring(0, separator)
    val maskedLocal = when (local.length) {
        1 -> "${local.first()}***"
        2 -> "${local.first()}***${local.last()}"
        else -> "${local.take(2)}***${local.last()}"
    }
    return "$maskedLocal${value.substring(separator)}"
}

internal fun googleBindingSummary(
    identities: List<LinkedLoginIdentity>,
    googleConfigured: Boolean = true,
): String {
    val google = identities.firstOrNull { it.provider.equals("google", ignoreCase = true) }
        ?: return if (googleConfigured) "Google 未绑定 · 点击设置" else "Google 暂未配置"
    return "Google 已绑定 · ${maskedIdentityEmail(google.email)}"
}

internal fun federatedAuthErrorMessage(code: String?, fallback: String): String = when (code) {
    "google_oidc_not_configured" -> "Google 登录尚未配置，暂时无法绑定"
    "identity_owned_by_another_account" ->
        "这个 Google 账号已绑定到另一一龙账号，不能自动合并；请先在原账号解绑"
    "existing_account_requires_bind" -> "该账号已存在，请先登录原一龙账号后再主动绑定"
    "invalid_or_consumed_challenge" -> "本次 Google 登录已过期，请重新发起"
    "auth_rate_limited" -> "操作过于频繁，请稍后再试"
    "cannot_unlink_last_login" -> "这是账号最后一种登录方式，请先设置密码或绑定其他方式"
    "google_jwks_unavailable", "identity_service_unavailable" -> "身份服务暂时不可用，请稍后再试"
    else -> fallback.ifBlank { "Google 登录暂时不可用" }
}
