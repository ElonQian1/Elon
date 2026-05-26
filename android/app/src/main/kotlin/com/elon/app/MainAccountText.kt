package com.elon.app

import android.content.Context

internal fun accountInfoText(context: Context): String {
    val profile = UserProfileStore.load(context)
    return if (AuthManager.isLoggedIn(context)) {
        val account = AuthManager.account(context)
        val tail = if (account != null && account != profile.displayName) " · $account" else ""
        "我的开发工作台\n登录账号：${profile.displayName}$tail\n云端工作区已就绪，可在网页版和其它手机间同步。"
    } else {
        "我的开发工作台\n游客模式 · ${profile.displayName}\n登录后可在网页版和其它手机间继续同一个项目。"
    }
}
