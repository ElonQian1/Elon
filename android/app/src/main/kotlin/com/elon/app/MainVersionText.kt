package com.elon.app

internal fun localAppVersionLine(): String =
    "一龙 v${BuildConfig.VERSION_NAME}  (build ${BuildConfig.VERSION_CODE})"

internal fun serverVersionLine(info: ServerVersionInfo): String {
    val shortSha = info.gitSha
        ?.takeIf { it != "dev" }
        ?.take(8)
    return if (shortSha.isNullOrBlank()) {
        "服务器 v${info.versionName}"
    } else {
        "服务器 v${info.versionName} ($shortSha)"
    }
}
