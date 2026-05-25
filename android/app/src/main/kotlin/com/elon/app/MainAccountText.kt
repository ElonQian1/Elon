package com.elon.app

import android.content.Context

internal fun accountInfoText(context: Context): String {
    return if (AuthManager.isLoggedIn(context)) {
        val name = AuthManager.displayName(context)
        val account = AuthManager.account(context)
        val tail = if (account != null && account != name) " · $account" else ""
        "我的开发工作台\n登录账号：$name$tail\n云端工作区已就绪，可在网页版和其它手机间同步。"
    } else {
        "我的开发工作台\n游客模式 · 项目仅保存在本机\n登录后可在网页版和其它手机间继续同一个项目。"
    }
}
